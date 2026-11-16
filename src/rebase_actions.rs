use super::synchronization::with_locked_index;
use super::{index_conflict_paths, open_exact};
use anyhow::{Context, Result, bail};
use git2::{Oid, Repository, RepositoryState, build::CheckoutBuilder};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebaseRequest {
    pub target: String,
    pub expected_head: String,
    pub expected_reference: String,
    pub reverse_branch: Option<String>,
}

#[derive(Debug)]
pub struct RebaseFailure {
    pub conflict_paths: Vec<PathBuf>,
    message: String,
}
impl std::fmt::Display for RebaseFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RebaseFailure {}

pub fn is_rebase_state(state: RepositoryState) -> bool {
    matches!(
        state,
        RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge
    )
}

fn dirty_paths(repo: &Repository, staged: bool) -> Result<BTreeSet<PathBuf>> {
    use gix::bstr::ByteSlice;
    let mut paths = BTreeSet::new();
    let mut index = repo.index()?;
    index.read(true)?;
    if staged {
        let tree = repo.head()?.peel_to_tree()?;
        for delta in repo
            .diff_tree_to_index(Some(&tree), Some(&index), None)?
            .deltas()
        {
            if let Some(raw) = delta.new_file().path_bytes() {
                paths.insert(gix::path::from_bstr(raw.as_bstr())?.to_path_buf());
            }
        }
    }
    for delta in repo.diff_index_to_workdir(Some(&index), None)?.deltas() {
        if let Some(raw) = delta.new_file().path_bytes() {
            paths.insert(gix::path::from_bstr(raw.as_bstr())?.to_path_buf());
        }
    }
    Ok(paths)
}

fn ensure_clean(repo: &Repository, staged: bool) -> Result<()> {
    let conflicts = index_conflict_paths(repo)?;
    if !conflicts.is_empty() {
        return Err(RebaseFailure {
            conflict_paths: conflicts.into_iter().collect(),
            message: "리베이스 충돌이 남아 있습니다. 아래 파일을 해결하고 Stage한 뒤 계속하세요."
                .into(),
        }
        .into());
    }
    let paths = dirty_paths(repo, staged)?;
    if !paths.is_empty() {
        return Err(RebaseFailure { conflict_paths: paths.into_iter().collect(),
            message: if staged { "로컬 변경사항 때문에 리베이스를 시작할 수 없습니다. 아래 파일을 커밋하거나 스태시에 보관하세요." }
                else { "Stage하지 않은 변경사항이 있습니다. 아래 파일을 확인하고 Stage한 뒤 리베이스를 계속하세요." }.into() }.into());
    }
    Ok(())
}

fn failure(error: git2::Error, paths: BTreeSet<PathBuf>) -> anyhow::Error {
    let message = match error.code() {
        git2::ErrorCode::Locked => {
            "Git 잠금 때문에 리베이스를 진행할 수 없습니다. 다른 Git 작업과 잠금 상태를 확인하세요."
        }
        git2::ErrorCode::Conflict => {
            "로컬 파일과의 충돌 때문에 리베이스를 진행할 수 없습니다. 아래 경로와 파일 상태를 확인하세요."
        }
        _ => "리베이스를 완료하지 못했습니다. 아래 원인과 현재 저장소 상태를 확인하세요.",
    };
    anyhow::Error::new(error).context(RebaseFailure {
        conflict_paths: paths.into_iter().collect(),
        message: message.into(),
    })
}

fn branch_available(repo: &Repository, reference: &str) -> Result<()> {
    if repo.is_worktree() {
        let main = Repository::open(repo.commondir())?;
        if main
            .head()
            .ok()
            .is_some_and(|head| head.name().ok() == Some(reference))
        {
            bail!("기본 작업 트리에서 사용 중인 브랜치는 리베이스할 수 없습니다.");
        }
    }
    for name in repo.worktrees()?.iter() {
        let Some(name) = name? else {
            continue;
        };
        let other = Repository::open_from_worktree(&repo.find_worktree(name)?)?;
        if other.workdir() != repo.workdir()
            && other
                .head()
                .ok()
                .is_some_and(|head| head.name().ok() == Some(reference))
        {
            bail!("다른 작업 트리에서 사용 중인 브랜치는 리베이스할 수 없습니다.");
        }
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ReturnContext {
    branch: String,
    head: String,
    rebased_branch: String,
    rebased_head: String,
}

fn return_context_path(repo: &Repository) -> PathBuf {
    repo.path().join("rebase-merge/feather-return.json")
}

fn validate_return_context(repo: &Repository, context: &ReturnContext) -> Result<()> {
    if !context.branch.starts_with("refs/heads/")
        || !git2::Reference::is_valid_name(&context.branch)
        || !context.rebased_branch.starts_with("refs/heads/")
        || !git2::Reference::is_valid_name(&context.rebased_branch)
        || context.branch == context.rebased_branch
    {
        bail!("리베이스 복귀 브랜치 문맥이 올바르지 않습니다.");
    }
    Oid::from_str(&context.head).context("리베이스 복귀 HEAD 문맥이 올바르지 않습니다.")?;
    let original = Oid::from_str(&context.rebased_head)
        .context("리베이스 시작 HEAD 문맥이 올바르지 않습니다.")?;
    let rebase = repo.open_rebase(None)?;
    if rebase.orig_head_name()? != Some(context.rebased_branch.as_str())
        || rebase.orig_head_id() != Some(original)
    {
        bail!("저장된 복귀 문맥이 현재 리베이스와 일치하지 않습니다.");
    }
    Ok(())
}

fn read_return_context(repo: &Repository) -> Result<Option<ReturnContext>> {
    let path = return_context_path(repo);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("리베이스 복귀 문맥을 확인하지 못했습니다."),
        Ok(metadata) if !metadata.is_file() || metadata.len() > 65536 => {
            bail!("리베이스 복귀 문맥 파일이 올바르지 않습니다.")
        }
        Ok(_) => {}
    }
    let context: ReturnContext = serde_json::from_slice(&std::fs::read(path)?)
        .context("리베이스 복귀 문맥을 읽을 수 없습니다.")?;
    validate_return_context(repo, &context)?;
    Ok(Some(context))
}

fn save_return_context(repo: &Repository, context: &ReturnContext) -> Result<()> {
    use std::io::Write;
    validate_return_context(repo, context)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(return_context_path(repo))
        .context("리베이스 복귀 문맥을 저장하지 못했습니다. 현재 리베이스 상태를 확인하세요.")?;
    file.write_all(&serde_json::to_vec(context)?)?;
    file.sync_all()?;
    Ok(())
}

fn restore_start_branch(path: &Path, context: &ReturnContext, operation: &str) -> Result<()> {
    let restore = || -> Result<()> {
        let repo = open_exact(path)?;
        let branch = repo
            .find_reference(&context.branch)
            .context("복귀 브랜치를 찾을 수 없습니다.")?;
        if branch.peel_to_commit()?.id().to_string() != context.head {
            bail!("복귀 브랜치의 커밋이 변경되었습니다.");
        }
        branch_available(&repo, &context.branch)?;
        super::checkout_commit_branch(path, &context.branch, &context.head)
    };
    restore().with_context(|| format!("{operation}는 완료되었지만 원래 현재 브랜치 {}로 복귀하지 못했습니다. 작업 결과는 유지됩니다.", context.branch.trim_start_matches("refs/heads/")))
}
pub fn start_rebase(path: &Path, request: &RebaseRequest) -> Result<()> {
    let repo = open_exact(path)?;
    if repo.is_bare() || repo.state() != RepositoryState::Clean {
        bail!("작업 폴더에서 진행 중인 Git 작업을 완료한 뒤 리베이스하세요.");
    }
    let head = repo.head().context("현재 로컬 브랜치가 없습니다.")?;
    if !head.is_branch()
        || head.name()? != request.expected_reference
        || head.peel_to_commit()?.id().to_string() != request.expected_head
    {
        bail!("현재 브랜치 또는 HEAD가 변경되었습니다. 목록을 갱신한 뒤 다시 시도하세요.");
    }
    let current = head.peel_to_commit()?.id();
    let target =
        Oid::from_str(&request.target).context("리베이스 대상 커밋이 올바르지 않습니다.")?;
    repo.find_commit(target)
        .context("리베이스 대상 커밋을 찾을 수 없습니다.")?;
    let branch = if let Some(reference) = &request.reverse_branch {
        if !reference.starts_with("refs/heads/")
            || !git2::Reference::is_valid_name(reference)
            || reference == &request.expected_reference
        {
            bail!("역방향 리베이스할 다른 로컬 브랜치를 선택하세요.");
        }
        branch_available(&repo, reference)?;
        let branch = repo
            .find_reference(reference)
            .context("역방향 대상 브랜치를 찾을 수 없습니다.")?;
        if branch.peel_to_commit()?.id() != target {
            bail!("대상 브랜치의 커밋이 변경되었습니다. 다시 선택하세요.");
        }
        Some(repo.reference_to_annotated_commit(&branch)?)
    } else {
        branch_available(&repo, &request.expected_reference)?;
        None
    };
    let (ours, onto) = if branch.is_some() {
        (target, current)
    } else {
        (current, target)
    };
    repo.merge_base(ours, onto)
        .context("공통 조상이 없는 커밋으로는 리베이스할 수 없습니다.")?;
    ensure_clean(&repo, true)?;
    if ours == onto || repo.graph_descendant_of(ours, onto)? {
        return Ok(());
    }
    let upstream = repo.find_annotated_commit(onto)?;
    let result = with_locked_index(&repo, || {
        run_rebase(&repo, branch.as_ref(), Some(&upstream), false)
    });
    if let Some(rebased_branch) = &request.reverse_branch {
        let context = ReturnContext {
            branch: request.expected_reference.clone(),
            head: request.expected_head.clone(),
            rebased_branch: rebased_branch.clone(),
            rebased_head: request.target.clone(),
        };
        match result {
            Ok(()) => restore_start_branch(path, &context, "리베이스"),
            Err(error) => {
                if is_rebase_state(repo.state())
                    && let Err(context_error) = save_return_context(&repo, &context)
                {
                    return Err(error.context(format!(
                        "복귀 문맥을 저장하지 못했습니다: {context_error:#}"
                    )));
                }
                Err(error)
            }
        }
    } else {
        result
    }
}
pub(super) fn rebase_onto(repo: &Repository, target: Oid) -> Result<()> {
    let upstream = repo.find_annotated_commit(target)?;
    run_rebase(repo, None, Some(&upstream), false)
}

fn check_pending(repo: &Repository, expected_head: &str) -> Result<()> {
    if repo.is_bare() || !is_rebase_state(repo.state()) {
        bail!("진행 중인 리베이스가 없습니다.");
    }
    if repo.head()?.peel_to_commit()?.id().to_string() != expected_head {
        bail!("리베이스 HEAD가 변경되었습니다. 현재 상태를 다시 확인하세요.");
    }
    Ok(())
}

pub fn continue_rebase(path: &Path, expected_head: &str) -> Result<()> {
    let repo = open_exact(path)?;
    check_pending(&repo, expected_head)?;
    let context = read_return_context(&repo)?;
    with_locked_index(&repo, || run_rebase(&repo, None, None, true))?;
    if let Some(context) = context {
        restore_start_branch(path, &context, "리베이스")?;
    }
    Ok(())
}

pub fn abort_rebase(path: &Path, expected_head: &str) -> Result<()> {
    let repo = open_exact(path)?;
    check_pending(&repo, expected_head)?;
    let context = read_return_context(&repo)?;
    for lock in [
        repo.path().join("index.lock"),
        repo.path().join("HEAD.lock"),
    ] {
        match std::fs::symlink_metadata(lock) {
            Ok(_) => bail!("Git 잠금 때문에 리베이스를 취소할 수 없습니다."),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("리베이스 잠금 상태를 확인하지 못했습니다."),
        }
    }
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(repo.workdir().context("작업 폴더가 없습니다.")?)
        .args(["rebase", "--abort"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command
        .output()
        .context("Git을 실행하지 못해 리베이스를 취소할 수 없습니다.")?;
    if !output.status.success() {
        bail!(
            "리베이스 취소에 실패했습니다.\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if let Some(context) = context {
        restore_start_branch(path, &context, "리베이스 취소")?;
    }
    Ok(())
}

fn commit_current(rebase: &mut git2::Rebase<'_>, signature: &git2::Signature<'_>) -> Result<()> {
    match rebase.commit(None, signature, None) {
        Ok(_) => Ok(()),
        Err(error) if error.code() == git2::ErrorCode::Applied => Ok(()),
        Err(error) => Err(failure(error, BTreeSet::new())),
    }
}

fn check_initial_checkout(
    repo: &Repository,
    branch: Option<&git2::AnnotatedCommit<'_>>,
    upstream: &git2::AnnotatedCommit<'_>,
) -> Result<()> {
    use gix::bstr::ByteSlice;
    let current = repo.head()?.peel_to_tree()?;
    let target = repo.find_commit(upstream.id())?.tree()?;
    let mut changed = BTreeSet::new();
    let mut add_paths = |before: Option<&git2::Tree<'_>>, after: &git2::Tree<'_>| -> Result<()> {
        for delta in repo.diff_tree_to_tree(before, Some(after), None)?.deltas() {
            if delta.status() == git2::Delta::Deleted {
                continue;
            }
            if let Some(raw) = delta.new_file().path_bytes() {
                changed.insert(gix::path::from_bstr(raw.as_bstr())?.to_path_buf());
            }
        }
        Ok(())
    };
    add_paths(Some(&current), &target)?;
    let original = branch
        .map(|branch| branch.id())
        .unwrap_or(repo.head()?.peel_to_commit()?.id());
    let mut commits = repo.revwalk()?;
    commits.push(original)?;
    commits.hide(upstream.id())?;
    for id in commits {
        let commit = repo.find_commit(id?)?;
        if commit.parent_count() > 1 {
            continue;
        }
        let parent = if commit.parent_count() == 0 {
            None
        } else {
            Some(commit.parent(0)?.tree()?)
        };
        add_paths(parent.as_ref(), &commit.tree()?)?;
    }
    if changed.is_empty() {
        return Ok(());
    }
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(true)
        .recurse_ignored_dirs(true)
        .disable_pathspec_match(true);
    let root = repo.workdir().context("작업 폴더가 없습니다.")?;
    for target in &changed {
        options.pathspec(target);
        for parent in target
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            match std::fs::symlink_metadata(root.join(parent)) {
                Ok(metadata) if !metadata.is_dir() => {
                    options.pathspec(parent);
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    let mut paths = BTreeSet::new();
    for entry in repo.statuses(Some(&mut options))?.iter() {
        if !entry.status().is_wt_new() && !entry.status().is_ignored() {
            continue;
        }
        let path = gix::path::from_bstr(entry.path_bytes().as_bstr())?.to_path_buf();
        if changed
            .iter()
            .any(|target| target.starts_with(&path) || path.starts_with(target))
        {
            paths.insert(path);
        }
    }
    if !paths.is_empty() {
        return Err(RebaseFailure { conflict_paths: paths.into_iter().collect(),
            message: "추적되지 않은 파일을 덮어쓸 수 있어 리베이스를 시작하지 못했습니다. 아래 파일을 보관하거나 다른 위치로 옮긴 뒤 다시 시도하세요.".into() }.into());
    }
    Ok(())
}
fn run_rebase(
    repo: &Repository,
    branch: Option<&git2::AnnotatedCommit<'_>>,
    upstream: Option<&git2::AnnotatedCommit<'_>>,
    continuing: bool,
) -> Result<()> {
    ensure_clean(repo, !continuing)?;
    let signature = repo
        .signature()
        .context("리베이스 커밋을 만들 작성자 이름과 이메일을 설정하세요.")?;
    if !continuing && let Some(upstream) = upstream {
        check_initial_checkout(repo, branch, upstream)?;
    }
    let paths = RefCell::new(BTreeSet::new());
    let mut options = git2::RebaseOptions::new();
    let mut checkout = CheckoutBuilder::new();
    checkout
        .safe()
        .allow_conflicts(true)
        .conflict_style_merge(true)
        .notify_on(git2::CheckoutNotificationType::CONFLICT)
        .notify(|_, path, _, _, _| {
            if let Some(path) = path {
                paths.borrow_mut().insert(path.to_path_buf());
            }
            true
        });
    options.checkout_options(checkout);
    let mut rebase = if continuing {
        repo.open_rebase(Some(&mut options))
    } else {
        repo.rebase(branch, upstream, None, Some(&mut options))
    }
    .map_err(|error| failure(error, paths.borrow().clone()))?;
    if continuing && rebase.operation_current().is_some() {
        commit_current(&mut rebase, &signature)?;
    }
    while let Some(operation) = rebase.next() {
        operation.map_err(|error| failure(error, paths.borrow().clone()))?;
        let conflicts = index_conflict_paths(repo)?;
        if !conflicts.is_empty() {
            return Err(RebaseFailure { conflict_paths: conflicts.into_iter().collect(),
                message: "리베이스 충돌이 발생했습니다. 아래 파일을 해결하고 Stage한 뒤 리베이스를 계속하거나 취소하세요.".into() }.into());
        }
        commit_current(&mut rebase, &signature)?;
    }
    rebase
        .finish(Some(&signature))
        .map_err(|error| failure(error, paths.borrow().clone()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Oid, Repository, RepositoryState};
    use std::path::Path;

    fn fixture() -> (tempfile::TempDir, Repository) {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/history-rebase-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        repo.config()
            .unwrap()
            .set_str("user.name", "Committer")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "committer@example.invalid")
            .unwrap();
        commit(&repo, ".gitattributes", "* -text\n", "base");
        (dir, repo)
    }
    fn head(repo: &Repository) -> Oid {
        repo.head().unwrap().peel_to_commit().unwrap().id()
    }
    fn commit(repo: &Repository, file: &str, text: &str, message: &str) -> Oid {
        std::fs::write(repo.workdir().unwrap().join(file), text).unwrap();
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let author = git2::Signature::now("Original author", "author@example.invalid").unwrap();
        repo.commit(
            Some("HEAD"),
            &author,
            &author,
            message,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
    }
    fn checkout(repo: &Repository, branch: &str) {
        super::super::checkout_reference(repo.workdir().unwrap(), branch).unwrap();
    }
    fn request(repo: &Repository, target: Oid, reverse: bool) -> RebaseRequest {
        RebaseRequest {
            target: target.to_string(),
            expected_head: head(repo).to_string(),
            expected_reference: repo.head().unwrap().name().unwrap().into(),
            reverse_branch: reverse.then(|| "refs/heads/target".into()),
        }
    }
    fn split(repo: &Repository, conflicts: bool) -> (Oid, Oid) {
        if conflicts {
            commit(repo, "one.txt", "base\n", "base one");
            commit(repo, "two.txt", "base\n", "base two");
        }
        repo.branch("target", &repo.find_commit(head(repo)).unwrap(), false)
            .unwrap();
        let ours = commit(repo, "one.txt", "ours\n", "ours one");
        let ours = if conflicts {
            commit(repo, "two.txt", "ours\n", "ours two")
        } else {
            ours
        };
        checkout(repo, "refs/heads/target");
        if conflicts {
            commit(repo, "one.txt", "theirs\n", "theirs one");
        }
        let theirs = commit(repo, "two.txt", "theirs\n", "theirs two");
        checkout(repo, "refs/heads/main");
        (ours, theirs)
    }
    #[test]
    fn reverse_rebase_rejects_damaged_return_context_before_continuing_or_aborting() {
        let (dir, repo) = fixture();
        let (_, theirs) = split(&repo, true);
        assert!(start_rebase(dir.path(), &request(&repo, theirs, true)).is_err());
        let before = head(&repo);
        let file = return_context_path(&repo);
        let mut context: ReturnContext =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        context.rebased_head = Oid::ZERO_SHA1.to_string();
        std::fs::write(&file, serde_json::to_vec(&context).unwrap()).unwrap();
        assert!(continue_rebase(dir.path(), &before.to_string()).is_err());
        assert!(abort_rebase(dir.path(), &before.to_string()).is_err());
        assert_eq!(head(&repo), before);
        assert!(is_rebase_state(repo.state()));
    }
    #[test]
    fn reverse_rebase_noop_stays_on_initial_branch_and_return_lock_reports_partial_success() {
        let (dir, repo) = fixture();
        let (ours, theirs) = split(&repo, false);
        let request = request(&repo, theirs, true);
        let lock = repo.path().join("refs/heads/main.lock");
        std::fs::write(&lock, "existing").unwrap();
        let error = start_rebase(dir.path(), &request).unwrap_err();
        assert!(format!("{error:#}").contains("복귀"));
        assert_eq!(
            repo.find_reference("refs/heads/main").unwrap().target(),
            Some(ours)
        );
        assert!(
            repo.graph_descendant_of(
                repo.find_reference("refs/heads/target")
                    .unwrap()
                    .target()
                    .unwrap(),
                ours
            )
            .unwrap()
        );
        assert_eq!(repo.state(), RepositoryState::Clean);
        assert_eq!(std::fs::read_to_string(&lock).unwrap(), "existing");
        std::fs::remove_file(lock).unwrap();
        checkout(&repo, "refs/heads/main");
        let target = repo
            .find_reference("refs/heads/target")
            .unwrap()
            .target()
            .unwrap();
        start_rebase(dir.path(), &super::tests::request(&repo, target, true)).unwrap();
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
        assert_eq!(head(&repo), ours);
    }
    #[test]
    fn history_rebase_preflight_checks_literal_nested_paths_and_obstructing_parents() {
        let (dir, repo) = fixture();
        let base = head(&repo);
        repo.branch("target", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        commit(&repo, "ours.txt", "ours\n", "ours");
        checkout(&repo, "refs/heads/target");
        std::fs::create_dir(dir.path().join("nested")).unwrap();
        let target = commit(&repo, "nested/space [x].txt", "incoming\n", "incoming");
        checkout(&repo, "refs/heads/main");
        let before = head(&repo);
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("nested/space [x].txt"), "keep\n").unwrap();
        let error = start_rebase(dir.path(), &request(&repo, target, false)).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<RebaseFailure>()
                .unwrap()
                .conflict_paths,
            [std::path::PathBuf::from("nested/space [x].txt")]
        );
        assert_eq!(head(&repo), before);
        std::fs::remove_file(dir.path().join("nested/space [x].txt")).unwrap();
        std::fs::remove_dir(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("nested"), "parent keep\n").unwrap();
        let error = start_rebase(dir.path(), &request(&repo, target, false)).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<RebaseFailure>()
                .unwrap()
                .conflict_paths,
            [std::path::PathBuf::from("nested")]
        );
        assert_eq!(head(&repo), before);
        assert_eq!(repo.state(), RepositoryState::Clean);
    }
    #[test]
    fn history_rebase_noop_fast_forward_and_changed_reverse_targets_are_safe() {
        let (dir, repo) = fixture();
        let base = head(&repo);
        repo.branch("target", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        let next = commit(&repo, "next.txt", "next\n", "next");
        start_rebase(dir.path(), &request(&repo, base, false)).unwrap();
        assert_eq!(head(&repo), next);
        checkout(&repo, "refs/heads/target");
        let expected = request(&repo, next, false);
        start_rebase(dir.path(), &expected).unwrap();
        assert_eq!(head(&repo), next);
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/target");
        checkout(&repo, "refs/heads/main");
        let expected = request(&repo, next, true);
        repo.find_reference("refs/heads/target")
            .unwrap()
            .set_target(base, "move target")
            .unwrap();
        assert!(start_rebase(dir.path(), &expected).is_err());
        assert_eq!(head(&repo), next);
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
        assert!(continue_rebase(dir.path(), &next.to_string()).is_err());
        assert!(abort_rebase(dir.path(), &next.to_string()).is_err());
    }
    #[test]
    fn history_rebase_rejects_untracked_overwrite_and_other_worktree_branch() {
        let (dir, repo) = fixture();
        let (_, theirs) = split(&repo, false);
        std::fs::write(dir.path().join("two.txt"), "untracked keep\n").unwrap();
        let before = head(&repo);
        assert!(start_rebase(dir.path(), &request(&repo, theirs, false)).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("two.txt")).unwrap(),
            "untracked keep\n"
        );
        assert_eq!(head(&repo), before);
        assert!(start_rebase(dir.path(), &request(&repo, theirs, true)).is_err());
        assert_eq!(head(&repo), before);
        assert_eq!(repo.state(), RepositoryState::Clean);
        std::fs::remove_file(dir.path().join("two.txt")).unwrap();
        let linked = dir.path().join("linked");
        let branch = repo.find_reference("refs/heads/target").unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&branch));
        let _worktree = repo.worktree("linked", &linked, Some(&options)).unwrap();
        assert!(start_rebase(dir.path(), &request(&repo, theirs, true)).is_err());
        assert_eq!(head(&repo), before);
        assert_eq!(repo.state(), RepositoryState::Clean);
    }
    #[test]
    fn history_rebase_toolbar_results_preserve_conflict_paths_and_refresh_after_abort() {
        let (dir, repo) = fixture();
        let (ours, theirs) = split(&repo, true);
        let result = crate::workspace_backend::ToolbarRequest {
            revision: 1,
            project: 0,
            path: dir.path().into(),
            name: "fixture".into(),
            action: crate::workspace_backend::ToolbarAction::RebaseCommit {
                request: request(&repo, theirs, false),
            },
        }
        .execute();
        assert!(result.outcome.is_err());
        assert!(!result.conflict_paths.is_empty());
        assert!(
            result
                .refreshed
                .unwrap()
                .0
                .work_state
                .operation
                .is_some_and(is_rebase_state)
        );
        let before = head(&repo);
        std::fs::write(repo.path().join("index.lock"), "existing lock").unwrap();
        assert!(abort_rebase(dir.path(), &before.to_string()).is_err());
        assert_eq!(head(&repo), before);
        assert!(is_rebase_state(repo.state()));
        std::fs::remove_file(repo.path().join("index.lock")).unwrap();
        let result = crate::workspace_backend::ToolbarRequest {
            revision: 2,
            project: 0,
            path: dir.path().into(),
            name: "fixture".into(),
            action: crate::workspace_backend::ToolbarAction::AbortRebase {
                expected_head: before.to_string(),
            },
        }
        .execute();
        assert!(result.outcome.is_ok());
        assert!(result.refreshed.unwrap().0.work_state.operation.is_none());
        assert_eq!(head(&repo), ours);
    }
    #[test]
    fn history_rebase_both_directions_preserve_branch_author_message_and_other_tip() {
        for reverse in [false, true] {
            let (dir, repo) = fixture();
            let (ours, theirs) = split(&repo, false);
            start_rebase(dir.path(), &request(&repo, theirs, reverse)).unwrap();
            let tip = repo
                .find_reference(if reverse {
                    "refs/heads/target"
                } else {
                    "refs/heads/main"
                })
                .unwrap()
                .peel_to_commit()
                .unwrap();
            assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
            assert_eq!(
                tip.parent_id(0).unwrap(),
                if reverse { ours } else { theirs }
            );
            assert_eq!(
                tip.message().unwrap(),
                if reverse { "theirs two" } else { "ours one" }
            );
            assert_eq!(tip.author().name().unwrap(), "Original author");
            let untouched = repo
                .find_reference(if reverse {
                    "refs/heads/main"
                } else {
                    "refs/heads/target"
                })
                .unwrap()
                .target()
                .unwrap();
            assert_eq!(untouched, if reverse { ours } else { theirs });
            assert_eq!(repo.state(), RepositoryState::Clean);
            if reverse {
                assert_eq!(head(&repo), ours);
                assert!(!dir.path().join("two.txt").exists());
            }
        }
    }
    #[test]
    fn history_rebase_continues_multiple_conflicts_and_abort_restores_rebased_branch() {
        for reverse in [false, true] {
            let (dir, repo) = fixture();
            let (ours, theirs) = split(&repo, true);
            let error = start_rebase(dir.path(), &request(&repo, theirs, reverse)).unwrap_err();
            assert!(
                !error
                    .downcast_ref::<RebaseFailure>()
                    .unwrap()
                    .conflict_paths
                    .is_empty()
            );
            if reverse {
                assert!(
                    repo.path()
                        .join("rebase-merge/feather-return.json")
                        .exists()
                );
            }
            assert!(continue_rebase(dir.path(), &head(&repo).to_string()).is_err());
            let mut rounds = 0;
            while is_rebase_state(repo.state()) {
                let fresh = Repository::open(dir.path()).unwrap();
                let conflicts = super::super::index_conflict_paths(&fresh).unwrap();
                for path in conflicts {
                    std::fs::write(dir.path().join(&path), "resolved\n").unwrap();
                    let mut index = fresh.index().unwrap();
                    index.read(true).unwrap();
                    index.add_path(&path).unwrap();
                    index.write().unwrap();
                }
                if let Err(error) = continue_rebase(dir.path(), &head(&repo).to_string()) {
                    assert!(error.downcast_ref::<RebaseFailure>().is_some(), "{error:#}");
                    assert!(is_rebase_state(repo.state()));
                }
                rounds += 1;
                assert!(rounds < 4);
            }
            assert!(rounds >= 2);
            assert_eq!(repo.state(), RepositoryState::Clean);
            assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
            assert!(
                repo.graph_descendant_of(
                    repo.find_reference(if reverse {
                        "refs/heads/target"
                    } else {
                        "refs/heads/main"
                    })
                    .unwrap()
                    .target()
                    .unwrap(),
                    if reverse { ours } else { theirs }
                )
                .unwrap()
            );
        }
        for reverse in [false, true] {
            let (dir, repo) = fixture();
            let (ours, theirs) = split(&repo, true);
            assert!(start_rebase(dir.path(), &request(&repo, theirs, reverse)).is_err());
            assert!(abort_rebase(dir.path(), &Oid::ZERO_SHA1.to_string()).is_err());
            abort_rebase(dir.path(), &head(&repo).to_string()).unwrap();
            assert_eq!(head(&repo), ours);
            assert_eq!(
                repo.find_reference("refs/heads/target").unwrap().target(),
                Some(theirs)
            );
            assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
            assert_eq!(repo.state(), RepositoryState::Clean);
        }
    }
    #[test]
    fn history_rebase_rejects_stale_dirty_locked_and_detached_requests_without_writes() {
        let (dir, repo) = fixture();
        let (ours, theirs) = split(&repo, false);
        let initial = request(&repo, theirs, false);
        std::fs::write(dir.path().join("one.txt"), "local edits\n").unwrap();
        let error = start_rebase(dir.path(), &initial).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<RebaseFailure>()
                .unwrap()
                .conflict_paths,
            [std::path::PathBuf::from("one.txt")]
        );
        assert_eq!(head(&repo), ours);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("one.txt")).unwrap(),
            "local edits\n"
        );
        std::fs::write(dir.path().join("one.txt"), "ours\n").unwrap();
        std::fs::write(repo.path().join("index.lock"), "existing lock").unwrap();
        assert!(start_rebase(dir.path(), &initial).is_err());
        assert_eq!(
            std::fs::read_to_string(repo.path().join("index.lock")).unwrap(),
            "existing lock"
        );
        std::fs::remove_file(repo.path().join("index.lock")).unwrap();
        let mut stale = initial.clone();
        stale.expected_head = Oid::ZERO_SHA1.to_string();
        assert!(start_rebase(dir.path(), &stale).is_err());
        repo.set_head_detached(ours).unwrap();
        assert!(start_rebase(dir.path(), &initial).is_err());
        assert_eq!(head(&repo), ours);
        assert_eq!(repo.state(), RepositoryState::Clean);
    }
}
