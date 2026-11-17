use super::{git_command, index_conflict_paths, open_exact};
use anyhow::{Context, Result, bail};
use git2::{Oid, RepositoryState};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitHistoryRequest {
    pub target: String,
    pub expected_head: String,
    pub expected_reference: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResetMode {
    Soft,
    #[default]
    Mixed,
    Hard,
}
impl ResetMode {
    pub const ALL: [Self; 3] = [Self::Soft, Self::Mixed, Self::Hard];
    pub fn name(self) -> &'static str {
        match self {
            Self::Soft => "Soft",
            Self::Mixed => "Mixed",
            Self::Hard => "Hard",
        }
    }
    fn flag(self) -> &'static str {
        match self {
            Self::Soft => "--soft",
            Self::Mixed => "--mixed",
            Self::Hard => "--hard",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Soft => "HEAD만 이동하며 인덱스와 작업 파일을 유지합니다.",
            Self::Mixed => "HEAD와 인덱스를 변경하며 작업 파일을 유지합니다.",
            Self::Hard => {
                "HEAD·인덱스·작업 파일을 변경합니다. 미커밋 변경과 체크아웃을 방해하는 미추적 경로가 제거될 수 있습니다."
            }
        }
    }
}

#[derive(Debug)]
pub struct HistoryActionFailure {
    pub conflict_paths: Vec<PathBuf>,
    message: String,
}
impl std::fmt::Display for HistoryActionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for HistoryActionFailure {}

fn validate(repo: &git2::Repository, request: &CommitHistoryRequest) -> Result<Oid> {
    if repo.is_bare() || repo.state() != RepositoryState::Clean || repo.index()?.has_conflicts() {
        bail!("진행 중인 Git 작업이나 충돌을 먼저 완료하세요.");
    }
    let head = repo.head()?;
    let reference = if head.is_branch() {
        Some(head.name()?.to_owned())
    } else {
        None
    };
    if head.peel_to_commit()?.id().to_string() != request.expected_head
        || reference != request.expected_reference
    {
        bail!("확인한 HEAD 또는 브랜치가 변경되었습니다. 기록을 갱신한 뒤 다시 실행하세요.");
    }
    let id = Oid::from_str(&request.target).context("대상 커밋 ID가 올바르지 않습니다.")?;
    repo.find_commit(id)
        .context("대상 커밋을 찾을 수 없습니다.")?;
    Ok(id)
}

fn run(repo: &git2::Repository, mut command: Command, reverting: bool) -> Result<()> {
    let output = command.output().context("Git을 실행하지 못했습니다.")?;
    if !output.status.success() {
        let (paths, diagnostic) = match index_conflict_paths(repo) {
            Ok(paths) => (paths.into_iter().collect::<Vec<_>>(), String::new()),
            Err(error) => (
                Vec::new(),
                format!("\n충돌 경로를 읽지 못했습니다: {error:#}"),
            ),
        };
        let recovery = if reverting && !paths.is_empty() {
            "\n충돌을 해결한 뒤 git revert --continue 또는 git revert --abort로 처리하세요."
        } else {
            ""
        };
        return Err(HistoryActionFailure {
            conflict_paths: paths,
            message: format!(
                "{} 실패: {}{recovery}{diagnostic}",
                if reverting { "Revert" } else { "Reset" },
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }
        .into());
    }
    Ok(())
}

const REVERT_CONTEXT: &str = "feather-git-revert.json";
#[derive(serde::Serialize, serde::Deserialize)]
struct RevertContext {
    request: CommitHistoryRequest,
    no_commit: bool,
}

pub fn is_revert_state(state: RepositoryState) -> bool {
    matches!(
        state,
        RepositoryState::Revert | RepositoryState::RevertSequence
    )
}
fn read_revert_context(repo: &git2::Repository) -> Result<Option<RevertContext>> {
    match std::fs::read(repo.path().join(REVERT_CONTEXT)) {
        Ok(bytes) => Ok(Some(
            serde_json::from_slice(&bytes).context("Revert 옵션 문맥을 읽지 못했습니다.")?,
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("Revert 옵션 문맥을 읽지 못했습니다."),
    }
}
fn remove_revert_context(repo: &git2::Repository) -> Result<()> {
    match std::fs::remove_file(repo.path().join(REVERT_CONTEXT)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Revert는 종료되었지만 옵션 문맥을 정리하지 못했습니다."),
    }
}
fn finish_revert_context(repo: &git2::Repository, outcome: Result<()>) -> Result<()> {
    if !is_revert_state(repo.state())
        && let Err(cleanup) = remove_revert_context(repo)
    {
        return match outcome {
            Ok(()) => Err(cleanup),
            Err(error) => Err(error.context(format!("{cleanup:#}"))),
        };
    }
    outcome
}
fn validate_revert_control(
    repo: &git2::Repository,
    expected_head: &str,
) -> Result<Option<RevertContext>> {
    if repo.is_bare() || !is_revert_state(repo.state()) {
        bail!("진행 중인 Revert가 없습니다.");
    }
    let head = repo.head()?;
    if head.peel_to_commit()?.id().to_string() != expected_head {
        bail!("HEAD가 변경되어 Revert 작업을 중단했습니다.");
    }
    let context = read_revert_context(repo)?;
    if let Some(context) = &context {
        let reference = if head.is_branch() {
            Some(head.name()?.to_owned())
        } else {
            None
        };
        let target = repo
            .find_reference("REVERT_HEAD")?
            .peel_to_commit()?
            .id()
            .to_string();
        if context.request.expected_head != expected_head
            || context.request.expected_reference != reference
            || context.request.target != target
        {
            bail!("진행 중인 Revert가 저장된 옵션 문맥과 다릅니다. 저장소 상태를 확인하세요.");
        }
    }
    Ok(context)
}

pub fn continue_revert(path: &Path, expected_head: &str) -> Result<()> {
    let repo = open_exact(path)?;
    let context = validate_revert_control(&repo, expected_head)?;
    let conflicts = index_conflict_paths(&repo)?;
    if !conflicts.is_empty() {
        return Err(HistoryActionFailure {
            conflict_paths: conflicts.into_iter().collect(),
            message: "Revert 충돌을 해결하고 Stage한 뒤 계속하세요.".into(),
        }
        .into());
    }
    let mut cmd = git_command(&repo)?;
    cmd.args([
        "revert",
        if context.is_some_and(|context| context.no_commit) {
            "--quit"
        } else {
            "--continue"
        },
    ])
    .env("GIT_EDITOR", "true");
    let outcome = run(&repo, cmd, true);
    finish_revert_context(&repo, outcome)
}
pub fn abort_revert(path: &Path, expected_head: &str) -> Result<()> {
    let repo = open_exact(path)?;
    validate_revert_control(&repo, expected_head)?;
    let mut cmd = git_command(&repo)?;
    cmd.args(["revert", "--abort"]);
    let outcome = run(&repo, cmd, true);
    finish_revert_context(&repo, outcome)
}

pub fn revert_commit(path: &Path, request: &CommitHistoryRequest, no_commit: bool) -> Result<()> {
    let repo = open_exact(path)?;
    let target = validate(&repo, request)?;
    // Git이 종료한 이전 앱 문맥만 정리하며 손상된 문맥은 숨기지 않는다.
    if read_revert_context(&repo)?.is_some() {
        remove_revert_context(&repo)?;
    }
    let context = RevertContext {
        request: request.clone(),
        no_commit,
    };
    let bytes = serde_json::to_vec(&context)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(repo.path().join(REVERT_CONTEXT))
        .context("Revert 옵션을 저장하지 못해 작업을 시작하지 않았습니다.")?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    let mut cmd = git_command(&repo)?;
    cmd.args(["revert", "--no-edit"]);
    if no_commit {
        cmd.arg("--no-commit");
    }
    if repo.find_commit(target)?.parent_count() > 1 {
        cmd.args(["--mainline", "1"]);
    }
    cmd.arg(target.to_string());
    let outcome = run(&repo, cmd, true);
    finish_revert_context(&repo, outcome)
}
pub fn reset_commit(path: &Path, request: &CommitHistoryRequest, mode: ResetMode) -> Result<()> {
    let repo = open_exact(path)?;
    let target = validate(&repo, request)?;
    let mut cmd = git_command(&repo)?;
    cmd.args(["reset", mode.flag()])
        .arg(target.to_string())
        .arg("--");
    run(&repo, cmd, false)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn commit(repo: &git2::Repository, text: &str) -> Oid {
        std::fs::write(repo.workdir().unwrap().join("file.txt"), text).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("test", "test@example.invalid").unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            text,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
    }
    fn temporary_repo_dir() -> tempfile::TempDir {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/history-action-tests");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }
    fn fixture() -> (tempfile::TempDir, git2::Repository, Oid, Oid) {
        let dir = temporary_repo_dir();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config
            .set_str("user.email", "test@example.invalid")
            .unwrap();
        config.set_bool("commit.gpgsign", false).unwrap();
        config.set_str("core.autocrlf", "false").unwrap();
        let first = commit(&repo, "first\n");
        let second = commit(&repo, "second\n");
        (dir, repo, first, second)
    }
    fn request(repo: &git2::Repository, target: Oid) -> CommitHistoryRequest {
        let head = repo.head().unwrap();
        CommitHistoryRequest {
            target: target.to_string(),
            expected_head: head.peel_to_commit().unwrap().id().to_string(),
            expected_reference: head.is_branch().then(|| head.name().unwrap().into()),
        }
    }
    fn indexed(repo: &git2::Repository) -> Vec<u8> {
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        let entry = index.get_path(Path::new("file.txt"), 0).unwrap();
        repo.find_blob(entry.id).unwrap().content().to_vec()
    }
    pub(crate) fn custom_fixture(layout: &str) -> (tempfile::TempDir, git2::Repository, git2::Repository, Oid, Oid) {
        let dir = temporary_repo_dir();
        // 잘못된 Git 탐색도 테스트 영역 밖의 저장소에 도달하지 못하게 한다.
        std::fs::write(dir.path().join(".git"), "invalid test boundary").unwrap();
        let source = dir.path().join("source");
        let original = git2::Repository::init(&source).unwrap();
        let first = commit(&original, "first\n");
        let second = commit(&original, "second\n");
        let root = match layout {
            "shared" => source.clone(),
            "ancestor" => source.join("nested"),
            _ => dir.path().join("standalone"),
        };
        std::fs::create_dir_all(&root).unwrap();
        let gitdir = root.join(".git2");
        let selected = git2::build::RepoBuilder::new().bare(true).clone(source.to_str().unwrap(), &gitdir).unwrap();
        super::super::attach_workdir(&selected, &root).unwrap();
        drop(selected);
        let selected = git2::Repository::open(&gitdir).unwrap();
        let mut config = selected.config().unwrap();
        config.set_str("user.name", "QA Tester").unwrap();
        config.set_str("user.email", "qa@example.invalid").unwrap();
        config.set_bool("commit.gpgsign", false).unwrap();
        config.set_str("core.autocrlf", "false").unwrap();
        let mut index = selected.index().unwrap();
        index.read_tree(&selected.head().unwrap().peel_to_tree().unwrap()).unwrap();
        index.write().unwrap();
        if root != source {
            std::fs::copy(source.join("file.txt"), root.join("file.txt")).unwrap();
        }
        (dir, original, selected, first, second)
    }

    #[test]
    fn custom_git_command_explicitly_selects_repository_and_clears_inherited_paths() {
        let (_dir, original, selected, _, _) = custom_fixture("shared");
        let mut cmd = git_command(&selected).unwrap();
        let env: std::collections::BTreeMap<_, _> = cmd.get_envs().collect();
        for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
            assert_eq!(env.get(std::ffi::OsStr::new(key)), Some(&None));
        }
        // 명시적 인수는 다른 저장소의 환경변수가 있어도 우선해야 한다.
        cmd.env("GIT_DIR", original.path()).env("GIT_WORK_TREE", original.workdir().unwrap());
        let output = cmd.args(["rev-parse", "--absolute-git-dir"]).output().unwrap();
        assert!(output.status.success());
        let resolved = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
        assert_eq!(resolved.canonicalize().unwrap(), selected.path().canonicalize().unwrap());
    }

    #[test]
    fn custom_git_reset_preserves_sibling_and_ancestor_metadata_in_all_modes() {
        for layout in ["standalone", "shared", "ancestor"] {
            for mode in ResetMode::ALL {
                let (_dir, original, selected, first, second) = custom_fixture(layout);
                let original_index = std::fs::read(original.path().join("index")).unwrap();
                reset_commit(&selected.path().canonicalize().unwrap(), &request(&selected, first), mode).unwrap();
                assert_eq!(selected.head().unwrap().target(), Some(first), "{layout}: {mode:?}");
                assert_eq!(original.head().unwrap().target(), Some(second));
                assert_eq!(std::fs::read(original.path().join("index")).unwrap(), original_index);
                assert_eq!(indexed(&selected), if mode == ResetMode::Soft { b"second\n".as_slice() } else { b"first\n".as_slice() });
            }
        }
    }

    #[test]
    fn custom_git_revert_preserves_sibling_and_ancestor_metadata() {
        for layout in ["standalone", "shared", "ancestor"] {
            for no_commit in [false, true] {
                let (_dir, original, selected, _first, second) = custom_fixture(layout);
                let original_index = std::fs::read(original.path().join("index")).unwrap();
                revert_commit(&selected.path().canonicalize().unwrap(), &request(&selected, second), no_commit).unwrap();
                assert_eq!(indexed(&selected), b"first\n", "{layout}");
                assert_eq!(selected.head().unwrap().target() == Some(second), no_commit);
                assert_eq!(original.head().unwrap().target(), Some(second));
                assert_eq!(std::fs::read(original.path().join("index")).unwrap(), original_index);
            }
        }
    }

    #[test]
    fn custom_git_revert_controls_only_consume_selected_repository_state() {
        for layout in ["standalone", "shared", "ancestor"] {
            for (abort, no_commit) in [(false, false), (false, true), (true, false), (true, true)] {
                let (_dir, original, selected, _first, second) = custom_fixture(layout);
                let original_index = std::fs::read(original.path().join("index")).unwrap();
                let third = commit(&selected, "third\n");
                assert!(revert_commit(&selected.path().canonicalize().unwrap(), &request(&selected, second), no_commit).is_err());
                assert!(is_revert_state(selected.state()));
                if abort {
                    abort_revert(&selected.path().canonicalize().unwrap(), &third.to_string()).unwrap();
                    assert_eq!(selected.head().unwrap().target(), Some(third));
                } else {
                    std::fs::write(selected.workdir().unwrap().join("file.txt"), "resolved\n").unwrap();
                    let mut index = selected.index().unwrap();
                    index.add_path(Path::new("file.txt")).unwrap();
                    index.write().unwrap();
                    continue_revert(&selected.path().canonicalize().unwrap(), &third.to_string()).unwrap();
                    assert_eq!(selected.head().unwrap().target() == Some(third), no_commit);
                }
                assert_eq!(selected.state(), RepositoryState::Clean);
                assert_eq!(original.head().unwrap().target(), Some(second));
                assert_eq!(std::fs::read(original.path().join("index")).unwrap(), original_index);
                assert_eq!(original.state(), RepositoryState::Clean);
            }
        }
    }

    #[test]
    fn revert_modes_preserve_or_create_head_and_apply_inverse_to_index_and_worktree() {
        for no_commit in [false, true] {
            let (dir, repo, _, second) = fixture();
            revert_commit(dir.path(), &request(&repo, second), no_commit).unwrap();
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            assert_eq!(head.id() == second, no_commit);
            if !no_commit {
                assert_eq!(head.parent_id(0).unwrap(), second);
                assert!(head.message().unwrap().contains("Revert"));
            }
            assert_eq!(indexed(&repo), b"first\n");
            assert_eq!(
                std::fs::read(dir.path().join("file.txt")).unwrap(),
                b"first\n"
            );
        }
    }
    #[test]
    fn reset_modes_change_exactly_the_requested_layers() {
        for mode in ResetMode::ALL {
            let (dir, repo, first, _) = fixture();
            std::fs::write(dir.path().join("file.txt"), "local\n").unwrap();
            reset_commit(dir.path(), &request(&repo, first), mode).unwrap();
            assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), first);
            assert_eq!(
                indexed(&repo),
                if mode == ResetMode::Soft {
                    b"second\n".as_slice()
                } else {
                    b"first\n"
                }
            );
            assert_eq!(
                std::fs::read(dir.path().join("file.txt")).unwrap(),
                if mode == ResetMode::Hard {
                    b"first\n".as_slice()
                } else {
                    b"local\n"
                }
            );
        }
    }
    #[test]
    fn stale_head_and_branch_invalid_target_and_lock_do_not_mutate_files() {
        let (dir, repo, first, second) = fixture();
        let mut stale = request(&repo, first);
        stale.expected_head = first.to_string();
        assert!(reset_commit(dir.path(), &stale, ResetMode::Hard).is_err());
        stale = request(&repo, first);
        stale.expected_reference = Some("refs/heads/missing".into());
        assert!(revert_commit(dir.path(), &stale, true).is_err());
        stale = request(&repo, first);
        stale.target = "not-an-id".into();
        assert!(reset_commit(dir.path(), &stale, ResetMode::Mixed).is_err());
        std::fs::write(repo.path().join("index.lock"), "external owner").unwrap();
        assert!(reset_commit(dir.path(), &request(&repo, first), ResetMode::Mixed).is_err());
        assert_eq!(
            std::fs::read(repo.path().join("index.lock")).unwrap(),
            b"external owner"
        );
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), second);
        assert_eq!(
            std::fs::read(dir.path().join("file.txt")).unwrap(),
            b"second\n"
        );
    }
    #[test]
    fn revert_conflict_keeps_git_state_and_reports_paths_and_rejects_next_operation() {
        let (dir, repo, first, second) = fixture();
        let third = commit(&repo, "third\n");
        let error = revert_commit(dir.path(), &request(&repo, second), false).unwrap_err();
        assert!(
            !error
                .downcast_ref::<HistoryActionFailure>()
                .unwrap()
                .conflict_paths
                .is_empty()
        );
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        assert!(index.has_conflicts());
        assert_ne!(repo.state(), RepositoryState::Clean);
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), third);
        assert!(reset_commit(dir.path(), &request(&repo, first), ResetMode::Hard).is_err());
    }
    #[test]
    fn dirty_revert_preserves_local_changes_and_root_commit_reverts_without_parent() {
        let (dir, repo, _, second) = fixture();
        std::fs::write(dir.path().join("file.txt"), "local\n").unwrap();
        assert!(revert_commit(dir.path(), &request(&repo, second), false).is_err());
        assert_eq!(
            std::fs::read(dir.path().join("file.txt")).unwrap(),
            b"local\n"
        );
        let rootdir = temporary_repo_dir();
        let root = git2::Repository::init(rootdir.path()).unwrap();
        root.config().unwrap().set_str("user.name", "test").unwrap();
        root.config()
            .unwrap()
            .set_str("user.email", "test@example.invalid")
            .unwrap();
        root.config()
            .unwrap()
            .set_bool("commit.gpgsign", false)
            .unwrap();
        let id = commit(&root, "root\n");
        revert_commit(rootdir.path(), &request(&root, id), false).unwrap();
        assert!(!rootdir.path().join("file.txt").exists());
        assert_eq!(
            root.head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .parent_id(0)
                .unwrap(),
            id
        );
    }
    #[test]
    fn merge_revert_uses_first_parent_and_reset_allows_detached_head() {
        let (dir, repo, first, second) = fixture();
        let second_commit = repo.find_commit(second).unwrap();
        let first_commit = repo.find_commit(first).unwrap();
        let tree = second_commit.tree().unwrap();
        let sig = git2::Signature::now("test", "test@example.invalid").unwrap();
        let merge = repo
            .commit(
                None,
                &sig,
                &sig,
                "merge",
                &tree,
                &[&first_commit, &second_commit],
            )
            .unwrap();
        repo.set_head_detached(merge).unwrap();
        revert_commit(dir.path(), &request(&repo, merge), false).unwrap();
        assert_eq!(
            std::fs::read(dir.path().join("file.txt")).unwrap(),
            b"first\n"
        );
        repo.set_head_detached(merge).unwrap();
        reset_commit(dir.path(), &request(&repo, second), ResetMode::Hard).unwrap();
        assert!(repo.head_detached().unwrap());
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), second);
    }
    #[test]
    fn toolbar_refreshes_after_reset_and_commit_snapshot_reads_unreferenced_target() {
        let (dir, repo, first, second) = fixture();
        let sig = git2::Signature::now("test", "test@example.invalid").unwrap();
        let tree = repo.find_commit(first).unwrap().tree().unwrap();
        let orphan = repo.commit(None, &sig, &sig, "orphan", &tree, &[]).unwrap();
        let snapshot =
            super::super::commit_snapshot(dir.path(), "fixture", &orphan.to_string()).unwrap();
        assert!(
            snapshot
                .commits
                .iter()
                .any(|commit| commit.id == orphan.to_string())
        );
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), second);
        let action = crate::workspace_backend::ToolbarAction::ResetCommit {
            request: request(&repo, first),
            mode: ResetMode::Mixed,
        };
        let result = crate::workspace_backend::ToolbarRequest {
            revision: 1,
            project: 0,
            path: dir.path().into(),
            name: "fixture".into(),
            action,
        }
        .execute();
        assert!(result.outcome.is_ok());
        assert_eq!(
            result.refreshed.unwrap().0.head.as_deref(),
            Some(first.to_string().as_str())
        );
    }
    fn conflicting_revert(no_commit: bool) -> (tempfile::TempDir, git2::Repository, Oid) {
        let (dir, repo, _, second) = fixture();
        let head = commit(&repo, "third\n");
        assert!(revert_commit(dir.path(), &request(&repo, second), no_commit).is_err());
        assert!(repo.path().join(REVERT_CONTEXT).exists());
        (dir, repo, head)
    }
    fn stage_resolution(repo: &git2::Repository) {
        std::fs::write(repo.workdir().unwrap().join("file.txt"), "resolved\n").unwrap();
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
    }
    #[test]
    fn revert_continue_commits_normally_but_keeps_no_commit_after_repository_reopen() {
        for no_commit in [false, true] {
            let (dir, repo, head) = conflicting_revert(no_commit);
            assert!(continue_revert(dir.path(), &head.to_string()).is_err());
            stage_resolution(&repo);
            drop(repo);
            continue_revert(dir.path(), &head.to_string()).unwrap();
            let repo = git2::Repository::open(dir.path()).unwrap();
            assert_eq!(
                repo.head().unwrap().peel_to_commit().unwrap().id() == head,
                no_commit
            );
            assert_eq!(indexed(&repo), b"resolved\n");
            assert_eq!(repo.state(), RepositoryState::Clean);
            assert!(!repo.path().join(REVERT_CONTEXT).exists());
        }
    }
    #[test]
    fn revert_abort_restores_start_state_and_rejects_stale_or_absent_operation() {
        for no_commit in [false, true] {
            let (dir, repo, head) = conflicting_revert(no_commit);
            assert!(abort_revert(dir.path(), &Oid::ZERO_SHA1.to_string()).is_err());
            assert!(is_revert_state(repo.state()));
            abort_revert(dir.path(), &head.to_string()).unwrap();
            assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().id(), head);
            assert_eq!(
                std::fs::read(dir.path().join("file.txt")).unwrap(),
                b"third\n"
            );
            assert_eq!(repo.state(), RepositoryState::Clean);
            assert!(!repo.path().join(REVERT_CONTEXT).exists());
            assert!(continue_revert(dir.path(), &head.to_string()).is_err());
            assert!(abort_revert(dir.path(), &head.to_string()).is_err());
        }
    }
    #[test]
    fn external_revert_continues_without_app_context_and_locks_preserve_pending_state() {
        let (dir, repo, _, second) = fixture();
        let head = commit(&repo, "third\n");
        let mut cmd = git_command(&repo).unwrap();
        cmd.args(["revert", "--no-edit"]).arg(second.to_string());
        assert!(!cmd.output().unwrap().status.success());
        assert!(!repo.path().join(REVERT_CONTEXT).exists());
        stage_resolution(&repo);
        std::fs::write(repo.path().join("index.lock"), "external owner").unwrap();
        assert!(continue_revert(dir.path(), &head.to_string()).is_err());
        assert!(is_revert_state(repo.state()));
        assert_eq!(
            std::fs::read(repo.path().join("index.lock")).unwrap(),
            b"external owner"
        );
        std::fs::remove_file(repo.path().join("index.lock")).unwrap();
        continue_revert(dir.path(), &head.to_string()).unwrap();
        assert_ne!(repo.head().unwrap().peel_to_commit().unwrap().id(), head);
    }
}
