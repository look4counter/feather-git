use super::{credentials, open_exact, index_conflict_paths, conflict_paths_from_index};
use anyhow::{Context, Result, bail};
use git2::{Oid, Repository, RepositoryState, build::CheckoutBuilder};
use std::{collections::BTreeSet, path::{Path, PathBuf}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeFailureKind {
    LocalChanges,
    Locked,
    MergeConflicts,
    Other,
}

#[derive(Debug)]
pub struct MergeFailure {
    pub kind: MergeFailureKind,
    pub conflict_paths: Vec<PathBuf>,
}
impl std::fmt::Display for MergeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.kind {
            MergeFailureKind::LocalChanges if self.conflict_paths.is_empty() => "로컬 변경사항 때문에 병합할 수 없습니다. 원인 파일 경로를 확인하지 못했습니다. 변경사항을 커밋하거나 스태시에 보관한 뒤 다시 시도하세요.",
            MergeFailureKind::LocalChanges => "로컬 변경사항 때문에 병합할 수 없습니다. 아래 파일의 변경사항을 커밋하거나 스태시에 보관한 뒤 다시 시도하세요.",
            MergeFailureKind::Locked => "다른 Git 작업의 잠금 때문에 작업을 진행할 수 없습니다. 실행 중인 Git 작업과 잠금 상태를 확인하세요.",
            MergeFailureKind::MergeConflicts => "병합 충돌이 발생했습니다. 아래 파일의 충돌을 해결하고 스테이지한 뒤 병합을 완료하세요.",
            MergeFailureKind::Other => "Git 작업을 완료하지 못했습니다. 아래 오류와 파일 상태를 확인하세요.",
        })
    }
}
impl std::error::Error for MergeFailure {}

fn checkout_failure(error: git2::Error, paths: BTreeSet<PathBuf>) -> anyhow::Error {
    let kind = match error.code() {
        git2::ErrorCode::Conflict => MergeFailureKind::LocalChanges,
        git2::ErrorCode::Locked => MergeFailureKind::Locked,
        _ => MergeFailureKind::Other,
    };
    let conflict_paths = if kind == MergeFailureKind::Locked { Vec::new() } else { paths.into_iter().collect() };
    anyhow::Error::new(error).context(MergeFailure { kind, conflict_paths })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncMode {
    Fetch,
    Merge,
    FastForwardOnly,
    Rebase,
}
impl SyncMode {
    pub const OPTIONS: [Self; 3] = [
        Self::Fetch,
        Self::Merge,
        Self::FastForwardOnly,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Fetch => "fetch",
            Self::Merge => "merge",
            Self::FastForwardOnly => "ff-only",
            Self::Rebase => "rebase",
        }
    }
    pub fn from_key(key: &str) -> Self {
        match key {
            "fetch" => Self::Fetch,
            "ff-only" => Self::FastForwardOnly,
            "rebase" => Self::Rebase,
            _ => Self::Merge,
        }
    }
    pub fn from_option_key(key: &str) -> Self {
        let mode = Self::from_key(key);
        if Self::OPTIONS.contains(&mode) { mode } else { Self::Merge }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Fetch => "Fetch",
            Self::Merge => "Pull (fast-forward if possible)",
            Self::FastForwardOnly => "Pull (fast-forward only)",
            Self::Rebase => "Pull (rebase)",
        }
    }
}

pub fn abort_merge(path: &Path, expected_head: &str) -> Result<()> {
    let repo = open_exact(path)?;
    if repo.is_bare() || repo.state() != RepositoryState::Merge {
        bail!("취소할 병합이 없습니다. 저장소 상태를 다시 확인하세요.");
    }
    let head = repo.head()?;
    if !head.is_branch() || head.peel_to_commit()?.id().to_string() != expected_head {
        bail!("현재 브랜치가 변경되어 병합 취소를 중단했습니다.");
    }
    let root = repo.workdir().context("작업 폴더가 없습니다.")?;
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(root).args(["merge", "--abort"])
        .env_remove("GIT_DIR").env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE").env_remove("GIT_COMMON_DIR")
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().context("Git을 실행하지 못해 병합을 취소할 수 없습니다.")?;
    if !output.status.success() {
        bail!("병합 취소에 실패했습니다. 저장소 상태를 확인하세요.\n{}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(())
}
pub fn merge_commit(path: &Path, id: &str) -> Result<()> {
    let repo = open_exact(path)?;
    if repo.is_bare() || repo.state() != RepositoryState::Clean {
        bail!("작업 폴더와 진행 중인 Git 작업 상태를 확인한 뒤 병합하세요.");
    }
    let head = repo.head().context("현재 체크아웃된 브랜치가 없습니다.")?;
    if !head.is_branch() {
        bail!("로컬 브랜치에 체크아웃한 뒤 병합하세요.");
    }
    let ours = head.peel_to_commit()?.id();
    let theirs = Oid::from_str(id).context("병합할 커밋 식별자가 올바르지 않습니다.")?;
    repo.find_commit(theirs).context("병합할 커밋을 찾을 수 없습니다.")?;
    if ours == theirs || repo.graph_descendant_of(ours, theirs)? {
        return Ok(());
    }
    if repo.graph_descendant_of(theirs, ours)? {
        return with_locked_index(&repo, || fast_forward(&repo, head.name().context("브랜치 이름이 없습니다.")?, ours, theirs));
    }
    repo.merge_base(ours, theirs).context("공통 커밋이 없는 기록은 병합할 수 없습니다.")?;
    with_locked_index(&repo, || merge(&repo, ours, theirs, &format!("Merge commit {id}")))
}
pub fn synchronize(path: &Path, mode: SyncMode) -> Result<()> {
    let repo = open_exact(path)?;
    if repo.is_bare() {
        bail!("작업 폴더가 있는 저장소에서 실행하세요.");
    }
    let head = repo.head().ok();
    if mode != SyncMode::Fetch {
        if repo.state() != RepositoryState::Clean {
            bail!("진행 중인 병합 또는 리베이스를 먼저 완료하거나 취소하세요.");
        }
        if !head.as_ref().is_some_and(|head| head.is_branch()) {
            bail!("현재 로컬 브랜치에 체크아웃한 뒤 Pull을 실행하세요.");
        }
    }
    let local_name = head
        .as_ref()
        .filter(|head| head.is_branch())
        .and_then(|head| head.name().ok());
    let upstream = if mode != SyncMode::Fetch {
        Some(
            repo.branch_upstream_name(local_name.unwrap())
                .context("현재 브랜치의 upstream이 없습니다. 추적 브랜치를 먼저 설정하세요.")?
                .as_str()
                .context("upstream 이름을 읽을 수 없습니다.")?
                .to_owned(),
        )
    } else {
        None
    };
    let remote = local_name
        .and_then(|name| repo.branch_upstream_remote(name).ok())
        .map(|name| {
            name.as_str()
                .map(str::to_owned)
                .context("원격 이름을 읽을 수 없습니다.")
        })
        .transpose()?
        .unwrap_or_else(|| "origin".into());
    if remote != "." {
        let mut remote = repo.find_remote(&remote).map_err(|_| {
            anyhow::anyhow!("가져올 원격 저장소가 없습니다. 원격 설정을 확인하세요.")
        })?;
        let mut options = git2::FetchOptions::new();
        options.remote_callbacks(credentials());
        remote.fetch(&[] as &[&str],Some(&mut options),Some("Feather Git: fetch"))
            .map_err(|_|anyhow::anyhow!("원격 변경사항을 가져오지 못했습니다. 원격 주소, 연결 및 인증 상태를 확인하세요."))?;
    }
    let Some(upstream) = upstream else {
        return Ok(());
    };
    if remote != "." {
        let branch = local_name
            .unwrap()
            .strip_prefix("refs/heads/")
            .context("로컬 브랜치 이름이 올바르지 않습니다.")?;
        let merge_ref = repo
            .config()?
            .get_string(&format!("branch.{branch}.merge"))?;
        let mut fetched = false;
        repo.fetchhead_foreach(|name, _, _, _| {
            fetched |= name == merge_ref;
            true
        })?;
        if !fetched {
            bail!(
                "설정된 upstream 브랜치를 원격에서 가져오지 못했습니다. 원격에서 삭제되었는지 확인하세요."
            );
        }
    }
    let current_head = repo.head()?;
    if current_head.name().ok() != local_name
        || current_head.target() != head.as_ref().and_then(|head| head.target())
    {
        bail!(
            "가져오는 동안 체크아웃된 브랜치가 변경되었습니다. 현재 상태를 확인하고 다시 실행하세요."
        );
    }
    let head = current_head;
    let ours = head.peel_to_commit()?.id();
    let theirs = repo
        .find_reference(&upstream)
        .context("가져온 upstream 브랜치를 찾을 수 없습니다.")?
        .peel_to_commit()?
        .id();
    if ours == theirs || repo.graph_descendant_of(ours, theirs)? {
        return Ok(());
    }
    if repo.graph_descendant_of(theirs, ours)? {
        return with_locked_index(&repo, || {
            fast_forward(
                &repo,
                head.name().context("현재 브랜치 이름이 없습니다.")?,
                ours,
                theirs,
            )
        });
    }
    match mode {
        SyncMode::FastForwardOnly => {
            bail!("fast-forward할 수 없습니다. 로컬과 원격 브랜치가 분기되어 있습니다.")
        }
        SyncMode::Merge => {
            repo.merge_base(ours, theirs)
                .context("공통 커밋이 없는 브랜치는 기본 Pull로 병합할 수 없습니다.")?;
            with_locked_index(&repo, || merge(&repo, ours, theirs, "Merge upstream"))
        }
        SyncMode::Rebase => with_locked_index(&repo, || rebase(&repo, theirs)),
        SyncMode::Fetch => unreachable!(),
    }
}

// Checkout must not update worktree files before discovering another writer's index lock.
pub(super) fn with_locked_index(repo: &Repository, work: impl FnOnce() -> Result<()>) -> Result<()> {
    use std::io::Write;
    let index_path = repo
        .index()?
        .path()
        .context("인덱스 경로가 없습니다.")?
        .to_owned();
    let mut lock_name = index_path.as_os_str().to_owned();
    lock_name.push(".lock");
    let lock_path = std::path::PathBuf::from(lock_name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|error| {
            let kind = if error.kind() == std::io::ErrorKind::AlreadyExists { MergeFailureKind::Locked } else { MergeFailureKind::Other };
            anyhow::Error::new(error).context(MergeFailure { kind, conflict_paths: Vec::new() })
        })?;
    struct Guard(std::path::PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Guard(lock_path.clone());
    file.write_all(&std::fs::read(&index_path)?)?;
    file.sync_all()?;
    drop(file);
    let mut index = git2::Index::open(&lock_path)?;
    repo.set_index(&mut index)?;
    let result = work();
    if result.is_ok() || repo.state() != RepositoryState::Clean {
        std::fs::rename(&lock_path, &index_path)
            .context("작업 인덱스를 저장하지 못했습니다. 저장소 상태를 확인하세요.")?;
    }
    result
}

fn fast_forward(repo: &Repository, branch: &str, ours: Oid, theirs: Oid) -> Result<()> {
    let mut transaction = repo.transaction()?;
    transaction.lock_ref(branch).map_err(|error| checkout_failure(error, BTreeSet::new()))?;
    if repo.find_reference(branch)?.target() != Some(ours) {
        bail!("현재 브랜치가 다른 작업으로 변경되어 fast-forward를 중단했습니다.");
    }
    transaction.set_target(branch, theirs, None, "pull: fast-forward")?;
    repo.reference("ORIG_HEAD", ours, true, "pull: before fast-forward")?;
    let mut paths = BTreeSet::new();
    let mut checkout = CheckoutBuilder::new();
    checkout.safe().notify_on(git2::CheckoutNotificationType::CONFLICT)
        .notify(|_, path, _, _, _| {
            if let Some(path) = path { paths.insert(path.to_path_buf()); }
            true
        });
    let result = repo.checkout_tree(repo.find_commit(theirs)?.as_object(), Some(&mut checkout));
    drop(checkout);
    result.map_err(|error| checkout_failure(error, paths))?;
    transaction.commit().context("작업 파일 갱신 후 브랜치 참조를 저장하지 못했습니다. 저장소 상태를 확인하세요.")?;
    Ok(())
}

fn merge_blocking_paths(repo: &Repository, ours: Oid, theirs: Oid) -> Result<BTreeSet<PathBuf>> {
    use gix::bstr::ByteSlice;
    let ours = repo.find_commit(ours)?;
    let theirs = repo.find_commit(theirs)?;
    let tree = ours.tree()?;
    let merged = repo.merge_commits(&ours, &theirs, None)?;
    let current = repo.index()?;
    let mut merged_paths = conflict_paths_from_index(&merged)?;
    let mut options = git2::DiffOptions::new();
    options.ignore_case(false);
    for delta in repo.diff_tree_to_index(Some(&tree), Some(&merged), Some(&mut options))?.deltas() {
        if let Some(path) = delta.new_file().path_bytes() {
            merged_paths.insert(gix::path::from_bstr(path.as_bstr())?.to_path_buf());
        }
    }
    let mut blocked = BTreeSet::new();
    for delta in repo.diff_tree_to_index(Some(&tree), Some(&current), Some(&mut options))?.deltas() {
        let Some(raw) = delta.new_file().path_bytes() else { continue; };
        let path = gix::path::from_bstr(raw.as_bstr())?;
        let entries = |index: &git2::Index| (0..4).map(|stage| {
            index.get_path(path.as_ref(), stage).map(|entry| (entry.id, entry.mode))
        }).collect::<Vec<_>>();
        if entries(&current) != entries(&merged) { blocked.insert(path.to_path_buf()); }
    }
    if !merged_paths.is_empty() {
        let mut options = git2::DiffOptions::new();
        options.include_untracked(true).disable_pathspec_match(true).ignore_submodules(true);
        for path in &merged_paths { options.pathspec(path); }
        for delta in repo.diff_index_to_workdir(Some(&current), Some(&mut options))?.deltas() {
            if let Some(path) = delta.new_file().path_bytes() {
                blocked.insert(gix::path::from_bstr(path.as_bstr())?.to_path_buf());
            }
        }
    }
    Ok(blocked)
}
fn merge(repo: &Repository, ours: Oid, theirs: Oid, message: &str) -> Result<()> {
    let signature = repo.signature().context("병합 커밋을 만들 작성자 이름과 이메일을 설정하세요.")?;
    let theirs_annotated = repo.find_annotated_commit(theirs)?;
    let mut paths = BTreeSet::new();
    let mut checkout = CheckoutBuilder::new();
    checkout.safe().allow_conflicts(true).conflict_style_merge(true)
        .notify_on(git2::CheckoutNotificationType::CONFLICT)
        .notify(|_, path, _, _, _| {
            if let Some(path) = path { paths.insert(path.to_path_buf()); }
            true
        });
    let result = repo.merge(&[&theirs_annotated], None, Some(&mut checkout));
    drop(checkout);
    if let Err(error) = result {
        if error.code() == git2::ErrorCode::Conflict && error.class() == git2::ErrorClass::Merge {
            match merge_blocking_paths(repo, ours, theirs) {
                Ok(blocked) => paths.extend(blocked),
                Err(diagnostic) => return Err(checkout_failure(error, paths)
                    .context(format!("원인 파일 경로를 확인하지 못했습니다: {diagnostic:#}"))),
            }
        }
        return Err(checkout_failure(error, paths));
    }
    let mut index = repo.index()?;
    if index.has_conflicts() {
        return Err(MergeFailure {
            kind: MergeFailureKind::MergeConflicts,
            conflict_paths: index_conflict_paths(repo)?.into_iter().collect(),
        }.into());
    }
    let tree = repo.find_tree(index.write_tree()?)?;
    let ours = repo.find_commit(ours)?;
    let theirs = repo.find_commit(theirs)?;
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&ours, &theirs])?;
    repo.cleanup_state()?;
    Ok(())
}
fn rebase(repo: &Repository, theirs: Oid) -> Result<()> {
    super::rebase_actions::rebase_onto(repo, theirs)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_merge_fast_forward_noop_and_protection() {
        let f = Fixture::new();
        let repo = Repository::open(&f.local).unwrap();
        let base = head(&f.local);
        let next = commit(&f.local, "base.txt", "incoming\n", "incoming");
        repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
        std::fs::write(f.local.join("base.txt"), "unsaved\n").unwrap();
        assert!(merge_commit(&f.local, &next.to_string()).is_err());
        assert_eq!(head(&f.local), base);
        assert_eq!(std::fs::read_to_string(f.local.join("base.txt")).unwrap(), "unsaved\n");
        repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
        std::fs::write(repo.path().join("index.lock"), "lock").unwrap();
        assert!(merge_commit(&f.local, &next.to_string()).is_err());
        assert_eq!(std::fs::read_to_string(repo.path().join("index.lock")).unwrap(), "lock");
        std::fs::remove_file(repo.path().join("index.lock")).unwrap();
        merge_commit(&f.local, &next.to_string()).unwrap();
        assert_eq!(head(&f.local), next);
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
        merge_commit(&f.local, &base.to_string()).unwrap();
        assert_eq!(head(&f.local), next);
        assert!(merge_commit(&f.local, "invalid").is_err());
        repo.set_head_detached(next).unwrap();
        assert!(merge_commit(&f.local, &base.to_string()).is_err());
    }
    #[test]
    fn history_merge_divergence_and_conflicts() {
        for conflict in [false, true] {
            let f = Fixture::new();
            let repo = Repository::open(&f.local).unwrap();
            let base = head(&f.local);
            let theirs = commit(&f.local, "incoming.txt", "incoming\n", "incoming");
            repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
            let ours = commit(&f.local, if conflict { "incoming.txt" } else { "ours.txt" }, "ours\n", "ours");
            let result = merge_commit(&f.local, &theirs.to_string());
            if conflict {
                assert!(result.is_err());
                assert_eq!(repo.state(), RepositoryState::Merge);
                assert!(Repository::open(&f.local).unwrap().index().unwrap().has_conflicts());
                assert_eq!(head(&f.local), ours);
                assert!(merge_commit(&f.local, &theirs.to_string()).is_err());
            } else {
                result.unwrap();
                let merged = repo.find_commit(head(&f.local)).unwrap();
                assert_eq!(merged.parent_id(0).unwrap(), ours);
                assert_eq!(merged.parent_id(1).unwrap(), theirs);
                assert_eq!(repo.state(), RepositoryState::Clean);
            }
        }
    }
    #[test]
    fn history_merge_rejects_unborn_missing_and_unrelated_commits() {
        let f = Fixture::new();
        let repo = Repository::open(&f.local).unwrap();
        let before = head(&f.local);
        assert!(merge_commit(&f.local, &Oid::ZERO_SHA1.to_string()).is_err());
        let tree = repo.find_tree(repo.treebuilder(None).unwrap().write().unwrap()).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let unrelated = repo.commit(None, &sig, &sig, "unrelated", &tree, &[]).unwrap();
        assert!(merge_commit(&f.local, &unrelated.to_string()).is_err());
        assert_eq!(head(&f.local), before);
        repo.set_head("refs/heads/unborn").unwrap();
        assert!(merge_commit(&f.local, &before.to_string()).is_err());
    }
    #[test]
    fn merge_abort_preserves_head_local_files_and_rejects_locks_or_stale_targets() {
        for resolve in [false, true] {
            let f = Fixture::new();
            let repo = Repository::open(&f.local).unwrap();
            let base = head(&f.local);
            let theirs = commit(&f.local, "base.txt", "incoming\n", "incoming");
            repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
            let ours = commit(&f.local, "base.txt", "ours\n", "ours");
            std::fs::write(f.local.join("local-only.txt"), "keep\n").unwrap();
            std::fs::write(f.local.join(".gitattributes"), "* -text\n# local keep\n").unwrap();
            assert!(merge_commit(&f.local, &theirs.to_string()).is_err());
            if resolve {
                std::fs::write(f.local.join("base.txt"), "resolved\n").unwrap();
                let mut index = repo.index().unwrap(); index.read(true).unwrap();
                index.add_path(Path::new("base.txt")).unwrap(); index.write().unwrap();
            }
            assert!(abort_merge(&f.local, &base.to_string()).is_err());
            assert_eq!(repo.state(), RepositoryState::Merge);
            let before = std::fs::read(f.local.join("base.txt")).unwrap();
            std::fs::write(repo.path().join("index.lock"), "existing lock").unwrap();
            assert!(abort_merge(&f.local, &ours.to_string()).is_err());
            assert_eq!(std::fs::read(f.local.join("base.txt")).unwrap(), before);
            assert_eq!(repo.state(), RepositoryState::Merge);
            std::fs::remove_file(repo.path().join("index.lock")).unwrap();
            if resolve {
                let result = crate::workspace_backend::ToolbarRequest {
                    revision: 1, project: 0, path: f.local.clone(), name: "fixture".into(),
                    action: crate::workspace_backend::ToolbarAction::AbortMerge { expected_head: ours.to_string() },
                }.execute();
                assert!(result.outcome.is_ok());
                assert!(result.refreshed.unwrap().0.work_state.operation.is_none());
            } else {
                abort_merge(&f.local, &ours.to_string()).unwrap();
            }
            assert_eq!(head(&f.local), ours);
            assert_eq!(repo.state(), RepositoryState::Clean);
            assert_eq!(std::fs::read_to_string(f.local.join("base.txt")).unwrap(), "ours\n");
            assert_eq!(std::fs::read_to_string(f.local.join("local-only.txt")).unwrap(), "keep\n");
            assert_eq!(std::fs::read_to_string(f.local.join(".gitattributes")).unwrap(), "* -text\n# local keep\n");
            assert!(abort_merge(&f.local, &ours.to_string()).is_err());
        }
    }

    #[test]
    fn merge_failure_reports_only_real_blockers_after_stage_and_unstage() {
        for divergent in [false, true] {
            let f = Fixture::new();
            let repo = Repository::open(&f.local).unwrap();
            let base = commit(&f.local, "한글 file.txt", "base\n", "base file");
            commit(&f.local, "base.txt", "incoming\n", "incoming one");
            let theirs = commit(&f.local, "한글 file.txt", "incoming\n", "incoming two");
            repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
            if divergent { commit(&f.local, "ours.txt", "ours\n", "ours"); }
            let ours = head(&f.local);
            let local_content = if divergent { "incoming\n" } else { "local edits\n" };
            for path in ["base.txt", "한글 file.txt"] {
                std::fs::write(f.local.join(path), local_content).unwrap();
                let file = crate::diff_git::load_stage(&f.local).unwrap().into_iter().find(|file| file.label == path && !file.staged).unwrap();
                crate::diff_git::toggle_stage(&f.local, &file).unwrap();
                let file = crate::diff_git::load_stage(&f.local).unwrap().into_iter().find(|file| file.label == path && file.staged).unwrap();
                crate::diff_git::toggle_stage(&f.local, &file).unwrap();
            }
            std::fs::write(f.local.join(".gitattributes"), "* -text\n# unrelated\n").unwrap();
            let error = merge_commit(&f.local, &theirs.to_string()).unwrap_err();
            let details = error.downcast_ref::<MergeFailure>().unwrap();
            assert_eq!(details.kind, MergeFailureKind::LocalChanges);
            assert_eq!(details.conflict_paths, [std::path::PathBuf::from("base.txt"), std::path::PathBuf::from("한글 file.txt")], "divergent={divergent}, error={error:#}");
            assert_eq!(head(&f.local), ours);
            assert_eq!(std::fs::read_to_string(f.local.join("base.txt")).unwrap(), local_content);
            let result = crate::workspace_backend::ToolbarRequest { revision: 1, project: 0,
                path: f.local.clone(), name: "fixture".into(),
                action: crate::workspace_backend::ToolbarAction::MergeCommit { id: theirs.to_string() } }.execute();
            assert_eq!(result.conflict_paths, details.conflict_paths);
            assert!(result.outcome.unwrap_err().contains("로컬 변경사항"));
            assert!(result.refreshed.is_ok());
        }
    }
    #[test]
    fn merge_failure_distinguishes_index_locks_and_actual_merge_conflicts() {
        let f = Fixture::new();
        let repo = Repository::open(&f.local).unwrap();
        let base = head(&f.local);
        let theirs = commit(&f.local, "base.txt", "incoming\n", "incoming");
        repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
        std::fs::write(repo.path().join("index.lock"), "existing").unwrap();
        let error = merge_commit(&f.local, &theirs.to_string()).unwrap_err();
        let details = error.downcast_ref::<MergeFailure>().unwrap();
        assert_eq!(details.kind, MergeFailureKind::Locked);
        assert!(details.conflict_paths.is_empty());
        assert_eq!(std::fs::read_to_string(repo.path().join("index.lock")).unwrap(), "existing");
        std::fs::remove_file(repo.path().join("index.lock")).unwrap();
        commit(&f.local, "base.txt", "ours\n", "ours");
        let error = merge_commit(&f.local, &theirs.to_string()).unwrap_err();
        let details = error.downcast_ref::<MergeFailure>().unwrap();
        assert_eq!(details.kind, MergeFailureKind::MergeConflicts);
        assert_eq!(details.conflict_paths, [std::path::PathBuf::from("base.txt")]);
        assert_eq!(repo.state(), RepositoryState::Merge);
    }
    #[test]
    fn merge_failure_paths_reach_pull_results_and_preserve_dirty_files() {
        let f = Fixture::new();
        commit(&f.remote, "base.txt", "incoming\n", "incoming");
        let ours = commit(&f.local, "ours.txt", "ours\n", "ours");
        std::fs::write(f.local.join("base.txt"), "local edits\n").unwrap();
        let result = crate::workspace_backend::SyncRequest { revision: 1, project: 0,
            path: f.local.clone(), name: "fixture".into(), mode: SyncMode::Merge }.execute();
        assert_eq!(result.conflict_paths, [PathBuf::from("base.txt")]);
        assert!(result.outcome.unwrap_err().contains("로컬 변경사항"));
        assert!(result.refreshed.is_ok());
        assert_eq!(head(&f.local), ours);
        assert_eq!(std::fs::read_to_string(f.local.join("base.txt")).unwrap(), "local edits\n");
    }
    #[test]
    fn merge_failure_preflight_checks_staged_changes_against_actual_merge_result() {
        for staged_matches_result in [false, true] {
            let f = Fixture::new();
            let repo = Repository::open(&f.local).unwrap();
            let base = head(&f.local);
            let theirs = commit(&f.local, "base.txt", "incoming\n", "incoming");
            repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
            let ours = commit(&f.local, "ours.txt", "ours\n", "ours");
            let (path, text) = if staged_matches_result { ("base.txt", "incoming\n") } else { ("ours.txt", "staged local\n") };
            std::fs::write(f.local.join(path), text).unwrap();
            let mut index = repo.index().unwrap();
            index.read(true).unwrap();
            index.add_path(Path::new(path)).unwrap(); index.write().unwrap();
            let blocked = merge_blocking_paths(&repo, ours, theirs).unwrap();
            if staged_matches_result {
                assert!(blocked.is_empty());
                merge_commit(&f.local, &theirs.to_string()).unwrap();
            } else {
                assert_eq!(blocked, BTreeSet::from([PathBuf::from("ours.txt")]));
                let error = merge_commit(&f.local, &theirs.to_string()).unwrap_err();
                assert_eq!(error.downcast_ref::<MergeFailure>().unwrap().conflict_paths, [PathBuf::from("ours.txt")]);
                assert_eq!(head(&f.local), ours);
                assert_eq!(std::fs::read_to_string(f.local.join(path)).unwrap(), text);
            }
        }
    }
    struct Fixture {
        _temp: tempfile::TempDir,
        remote: std::path::PathBuf,
        local: std::path::PathBuf,
    }
    fn commit(path: &Path, file: &str, text: &str, message: &str) -> Oid {
        let repo = Repository::open(path).unwrap();
        std::fs::write(path.join(file), text).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents = parent.iter().collect::<Vec<_>>();
        let signature = git2::Signature::now("Test User", "test@example.invalid").unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .unwrap()
    }
    fn head(path: &Path) -> Oid {
        Repository::open(path)
            .unwrap()
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::current_dir().unwrap().join("target/pull-tests");
            std::fs::create_dir_all(&root).unwrap();
            let temp = tempfile::tempdir_in(root).unwrap();
            let remote = temp.path().join("remote");
            let local = temp.path().join("local");
            let repo = Repository::init(&remote).unwrap();
            repo.set_head("refs/heads/main").unwrap();
            commit(&remote, ".gitattributes", "* -text\n", "test attributes");
            commit(&remote, "base.txt", "base\n", "base");
            git2::build::RepoBuilder::new()
                .remote_create(|repo, name, url| {
                    repo.config()?.set_bool("core.autocrlf", false)?;
                    repo.remote(name, url)
                })
                .clone(remote.to_str().unwrap(), &local)
                .unwrap();
            let mut config = Repository::open(&local).unwrap().config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config
                .set_str("user.email", "test@example.invalid")
                .unwrap();
            Self {
                _temp: temp,
                remote,
                local,
            }
        }
        fn diverge(&self) -> (Oid, Oid) {
            let ours = commit(&self.local, "local.txt", "local\n", "local commit");
            let theirs = commit(&self.remote, "remote.txt", "remote\n", "remote commit");
            (ours, theirs)
        }
    }
    #[test]
    fn selected_mode_keys_round_trip_and_unknown_values_use_normal_pull() {
        for mode in SyncMode::OPTIONS {
            assert_eq!(SyncMode::from_key(mode.key()), mode);
        }
        assert_eq!(SyncMode::from_key(""), SyncMode::Merge);
        assert_eq!(SyncMode::from_key("future-option"), SyncMode::Merge);
    }
    #[test]
    fn fetch_updates_remote_refs_without_touching_head_index_or_worktree() {
        let f = Fixture::new();
        let before = head(&f.local);
        std::fs::write(f.local.join("draft.txt"), "draft").unwrap();
        let repo = Repository::open(&f.local).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("draft.txt")).unwrap();
        index.write().unwrap();
        let original = std::fs::read(f.local.join(".git/index")).unwrap();
        let remote = commit(&f.remote, "remote.txt", "remote", "remote");
        synchronize(&f.local, SyncMode::Fetch).unwrap();
        assert_eq!(head(&f.local), before);
        assert_eq!(std::fs::read(f.local.join(".git/index")).unwrap(), original);
        assert_eq!(
            std::fs::read_to_string(f.local.join("draft.txt")).unwrap(),
            "draft"
        );
        assert!(!f.local.join("remote.txt").exists());
        assert_eq!(
            repo.find_reference("refs/remotes/origin/main")
                .unwrap()
                .target(),
            Some(remote)
        );
    }
    #[test]
    fn all_pull_modes_fast_forward_and_repeated_pull_is_a_noop() {
        for mode in [SyncMode::Merge, SyncMode::FastForwardOnly, SyncMode::Rebase] {
            let f = Fixture::new();
            let target = commit(&f.remote, "remote.txt", "remote", "remote");
            synchronize(&f.local, mode).unwrap();
            assert_eq!(head(&f.local), target);
            assert_eq!(
                std::fs::read_to_string(f.local.join("remote.txt")).unwrap(),
                "remote"
            );
            synchronize(&f.local, mode).unwrap();
            assert_eq!(head(&f.local), target);
            assert_eq!(
                Repository::open(&f.local).unwrap().state(),
                RepositoryState::Clean
            );
        }
    }
    #[test]
    fn divergent_modes_merge_reject_or_rebase_as_selected() {
        for mode in [SyncMode::Merge, SyncMode::FastForwardOnly, SyncMode::Rebase] {
            let f = Fixture::new();
            let (ours, theirs) = f.diverge();
            let repo_config = Repository::open(&f.local).unwrap();
            let mut config = repo_config.config().unwrap();
            config.set_str("pull.ff", "only").unwrap();
            config.set_bool("pull.rebase", true).unwrap();
            let result = synchronize(&f.local, mode);
            let repo = Repository::open(&f.local).unwrap();
            if mode == SyncMode::FastForwardOnly {
                assert!(result.is_err());
                assert_eq!(head(&f.local), ours);
                assert!(!f.local.join("remote.txt").exists());
            } else {
                result.unwrap();
                let tip = repo.head().unwrap().peel_to_commit().unwrap();
                assert_eq!(
                    std::fs::read_to_string(f.local.join("local.txt")).unwrap(),
                    "local\n"
                );
                assert_eq!(
                    std::fs::read_to_string(f.local.join("remote.txt")).unwrap(),
                    "remote\n"
                );
                if mode == SyncMode::Merge {
                    assert_eq!(tip.parent_count(), 2);
                    assert_eq!(tip.parent_id(0).unwrap(), ours);
                    assert_eq!(tip.parent_id(1).unwrap(), theirs);
                } else {
                    assert_eq!(tip.parent_count(), 1);
                    assert_eq!(tip.parent_id(0).unwrap(), theirs);
                    assert_ne!(tip.id(), ours);
                    assert_eq!(tip.message().unwrap(), "local commit");
                    assert_eq!(tip.author().name().unwrap(), "Test User");
                }
            }
            assert_eq!(head(&f.remote), theirs);
            assert_eq!(repo.state(), RepositoryState::Clean);
        }
    }
    #[test]
    fn local_ahead_does_not_make_extra_commits() {
        for mode in [SyncMode::Merge, SyncMode::FastForwardOnly, SyncMode::Rebase] {
            let f = Fixture::new();
            let ours = commit(&f.local, "local.txt", "local", "local");
            synchronize(&f.local, mode).unwrap();
            assert_eq!(head(&f.local), ours);
        }
    }
    #[test]
    fn merge_and_rebase_conflicts_are_preserved_for_resolution() {
        for mode in [SyncMode::Merge, SyncMode::Rebase] {
            let f = Fixture::new();
            commit(&f.local, "base.txt", "ours\n", "ours");
            commit(&f.remote, "base.txt", "theirs\n", "theirs");
            assert!(synchronize(&f.local, mode).is_err());
            let repo = Repository::open(&f.local).unwrap();
            assert!(repo.index().unwrap().has_conflicts());
            assert_ne!(repo.state(), RepositoryState::Clean);
            assert!(
                std::fs::read_to_string(f.local.join("base.txt"))
                    .unwrap()
                    .contains("<<<<<<<")
            );
            assert!(synchronize(&f.local, SyncMode::Merge).is_err());
        }
    }
    #[test]
    fn dirty_files_index_locks_and_reference_locks_are_not_overwritten() {
        for blocker in ["dirty", "index", "ref"] {
            let f = Fixture::new();
            let original = head(&f.local);
            commit(&f.remote, "base.txt", "upstream", "remote");
            let index = std::fs::read(f.local.join(".git/index")).unwrap();
            match blocker {
                "dirty" => std::fs::write(f.local.join("base.txt"), "unsaved").unwrap(),
                "index" => {
                    std::fs::write(f.local.join(".git/index.lock"), "existing lock").unwrap()
                }
                _ => std::fs::write(f.local.join(".git/refs/heads/main.lock"), "existing lock")
                    .unwrap(),
            }
            let before = std::fs::read(f.local.join("base.txt")).unwrap();
            assert!(synchronize(&f.local, SyncMode::FastForwardOnly).is_err());
            assert_eq!(head(&f.local), original);
            assert_eq!(std::fs::read(f.local.join("base.txt")).unwrap(), before);
            assert_eq!(std::fs::read(f.local.join(".git/index")).unwrap(), index);
        }
    }
    #[test]
    fn missing_upstream_detached_head_and_missing_remote_are_explicit() {
        let f = Fixture::new();
        let repo = Repository::open(&f.local).unwrap();
        let original = head(&f.local);
        repo.config().unwrap().remove("branch.main.merge").unwrap();
        assert!(synchronize(&f.local, SyncMode::Merge).is_err());
        assert_eq!(head(&f.local), original);
        repo.set_head_detached(original).unwrap();
        assert!(synchronize(&f.local, SyncMode::Rebase).is_err());
        synchronize(&f.local, SyncMode::Fetch).unwrap();
        repo.remote_set_url(
            "origin",
            f._temp
                .path()
                .join("secret-token/missing")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let error = synchronize(&f.local, SyncMode::Fetch)
            .unwrap_err()
            .to_string();
        assert!(!error.contains("secret-token"));
        repo.remote_delete("origin").unwrap();
        assert!(synchronize(&f.local, SyncMode::Fetch).is_err());
    }
    #[test]
    fn rebase_dirty_worktree_and_missing_signature_do_not_start_integration() {
        for dirty in [true, false] {
            let f = Fixture::new();
            let (ours, _) = f.diverge();
            let repo = Repository::open(&f.local).unwrap();
            if dirty {
                std::fs::write(f.local.join("base.txt"), "unsaved").unwrap();
            } else {
                repo.config().unwrap().set_str("user.name", "").unwrap();
            }
            assert!(
                synchronize(
                    &f.local,
                    if dirty {
                        SyncMode::Rebase
                    } else {
                        SyncMode::Merge
                    }
                )
                .is_err()
            );
            assert_eq!(head(&f.local), ours);
            assert_eq!(repo.state(), RepositoryState::Clean);
            if dirty {
                assert_eq!(
                    std::fs::read_to_string(f.local.join("base.txt")).unwrap(),
                    "unsaved"
                );
            }
        }
    }
    #[test]
    fn fetch_supports_unborn_head_but_pull_requires_an_existing_branch() {
        let f = Fixture::new();
        let path = f._temp.path().join("unborn");
        let repo = Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        repo.remote("origin", f.remote.to_str().unwrap()).unwrap();
        synchronize(&path, SyncMode::Fetch).unwrap();
        assert!(repo.head().is_err());
        assert!(repo.find_reference("refs/remotes/origin/main").is_ok());
        assert!(synchronize(&path, SyncMode::Merge).is_err());
    }
    #[test]
    fn default_pull_rejects_unrelated_histories_without_changing_local_head() {
        let f = Fixture::new();
        let before = head(&f.local);
        let repo = Repository::open(&f.remote).unwrap();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = commit.tree().unwrap();
        let signature = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let orphan = repo
            .commit(
                Some("refs/heads/orphan"),
                &signature,
                &signature,
                "new root",
                &tree,
                &[],
            )
            .unwrap();
        repo.reference("refs/heads/main", orphan, true, "fixture unrelated")
            .unwrap();
        assert!(synchronize(&f.local, SyncMode::Merge).is_err());
        assert_eq!(head(&f.local), before);
        assert_eq!(
            Repository::open(&f.local).unwrap().state(),
            RepositoryState::Clean
        );
    }
    #[test]
    fn pull_rejects_deleted_upstream_instead_of_using_a_stale_tracking_ref() {
        let f = Fixture::new();
        let before = head(&f.local);
        let repo = Repository::open(&f.remote).unwrap();
        repo.reference("refs/heads/other", before, true, "fixture")
            .unwrap();
        repo.set_head("refs/heads/other").unwrap();
        repo.find_reference("refs/heads/main")
            .unwrap()
            .delete()
            .unwrap();
        synchronize(&f.local, SyncMode::Fetch).unwrap();
        assert!(
            Repository::open(&f.local)
                .unwrap()
                .find_reference("refs/remotes/origin/main")
                .is_ok()
        );
        assert!(synchronize(&f.local, SyncMode::Merge).is_err());
        assert_eq!(head(&f.local), before);
    }
    #[test]
    fn unrelated_worktree_edits_survive_fast_forward() {
        let f = Fixture::new();
        std::fs::write(f.local.join("base.txt"), "local edit").unwrap();
        let target = commit(&f.remote, "remote.txt", "remote", "remote");
        synchronize(&f.local, SyncMode::Merge).unwrap();
        assert_eq!(head(&f.local), target);
        assert_eq!(
            std::fs::read_to_string(f.local.join("base.txt")).unwrap(),
            "local edit"
        );
    }
}
