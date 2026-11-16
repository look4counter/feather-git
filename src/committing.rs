use super::{credentials, open_exact};
use anyhow::{Context, Result, bail};
use git2::{Repository, RepositoryState};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Debug)]
pub struct CommitDraft {
    pub head: Option<String>,
    pub reference: String,
    pub last_message: String,
    pub default_message: String,
    pub has_staged: bool,
    pub can_amend: bool,
    pub is_merge: bool,
}
#[derive(Clone, Debug)]
pub struct CommitRequest {
    pub expected_head: Option<String>,
    pub expected_reference: String,
    pub message: String,
    pub amend: bool,
    pub push: bool,
}
#[derive(Debug)]
pub struct CommitOutcome {
    pub id: String,
    pub error: Option<String>,
}
fn head_commit(repo: &Repository) -> Result<Option<git2::Commit<'_>>> {
    match repo.head() {
        Ok(head) => Ok(Some(head.peel_to_commit()?)),
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
fn draft(repo: &Repository) -> Result<CommitDraft> {
    if repo.is_bare() {
        bail!("작업 폴더가 있는 저장소에서 커밋하세요.");
    }
    let state = repo.state();
    if !matches!(state, RepositoryState::Clean | RepositoryState::Merge) {
        bail!("진행 중인 리베이스 또는 다른 Git 작업을 먼저 완료하거나 취소하세요.");
    }
    let head = head_commit(repo)?;
    let index = repo.index()?;
    if index.has_conflicts() {
        bail!("해결되지 않은 충돌이 있습니다. 충돌을 해결하고 스테이지한 뒤 커밋하세요.");
    }
    let tree = head.as_ref().map(|head| head.tree()).transpose()?;
    let has_staged = repo
        .diff_tree_to_index(tree.as_ref(), Some(&index), None)?
        .deltas()
        .len()
        > 0;
    let reference = repo
        .find_reference("HEAD")?
        .symbolic_target()?
        .unwrap_or("HEAD")
        .to_owned();
    Ok(CommitDraft {
        head: head.as_ref().map(|head| head.id().to_string()),
        reference,
        last_message: head
            .as_ref()
            .map(|head| String::from_utf8_lossy(head.message_bytes()).into_owned())
            .unwrap_or_default(),
        default_message: if state == RepositoryState::Merge {
            repo.message().unwrap_or_default()
        } else {
            String::new()
        },
        has_staged,
        can_amend: head.is_some() && state == RepositoryState::Clean,
        is_merge: state == RepositoryState::Merge,
    })
}
pub fn prepare_commit(path: &Path) -> Result<CommitDraft> {
    draft(&open_exact(path)?)
}
pub fn commit_staged(path: &Path, request: CommitRequest) -> Result<CommitOutcome> {
    let message = request.message.replace("\r\n", "\n").replace('\r', "\n");
    let message = message.trim();
    if message.is_empty() || message.contains('\0') {
        bail!("유효한 커밋 메시지를 입력하세요.");
    }
    let mut repo = open_exact(path)?;
    let index_path = repo
        .index()?
        .path()
        .context("인덱스 경로가 없습니다.")?
        .to_owned();
    let mut lock = index_path.as_os_str().to_owned();
    lock.push(".lock");
    let lock = std::path::PathBuf::from(lock);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .context("인덱스가 잠겨 있어 커밋할 수 없습니다.")?;
    struct Guard(std::path::PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Guard(lock);
    drop(file);
    repo.index()?.read(true)?;
    let current = draft(&repo)?;
    if current.head != request.expected_head || current.reference != request.expected_reference {
        bail!(
            "팝업을 연 뒤 HEAD 또는 브랜치가 변경되었습니다. 창을 다시 열어 현재 상태를 확인하세요."
        );
    }
    if request.amend && !current.can_amend {
        bail!("현재 상태에서는 마지막 커밋을 정정할 수 없습니다.");
    }
    if !request.amend && !current.has_staged && !current.is_merge {
        bail!("스테이지된 변경사항이 없습니다.");
    }
    let signature = repo
        .signature()
        .context("커밋 작성자 이름과 이메일을 환경설정에서 설정하세요.")?;
    let mut merge_heads = Vec::new();
    if current.is_merge {
        repo.mergehead_foreach(|id| {
            merge_heads.push(*id);
            true
        })?;
    }
    let mut index = repo.index()?;
    index.read(true)?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let head = head_commit(&repo)?;
    let id = if request.amend {
        head.as_ref()
            .context("정정할 마지막 커밋이 없습니다.")?
            .amend(
                Some(&request.expected_reference),
                None,
                Some(&signature),
                None,
                Some(message),
                Some(&tree),
            )?
    } else {
        let mut parents = Vec::new();
        if let Some(head) = head {
            parents.push(head);
        }
        for id in merge_heads {
            if !parents.iter().any(|parent| parent.id() == id) {
                parents.push(repo.find_commit(id)?);
            }
        }
        repo.commit(
            Some(&request.expected_reference),
            &signature,
            &signature,
            message,
            &tree,
            &parents.iter().collect::<Vec<_>>(),
        )?
    };
    let mut outcome = CommitOutcome {
        id: id.to_string(),
        error: None,
    };
    if current.is_merge && repo.cleanup_state().is_err() {
        outcome.error = Some(
            "커밋은 완료됐지만 병합 상태를 정리하지 못했습니다. 저장소 상태를 확인하세요.".into(),
        );
        return Ok(outcome);
    }
    if request.push
        && let Err(error) = push_current(&repo, &request.expected_reference)
    {
        outcome.error = Some(format!(
            "커밋은 완료됐지만 푸시하지 못했습니다. 로컬 커밋은 유지됩니다.\n{error}"
        ));
    }
    Ok(outcome)
}
fn push_current(repo: &Repository, reference: &str) -> Result<()> {
    let branch = reference
        .strip_prefix("refs/heads/")
        .context("푸시할 로컬 브랜치가 없습니다.")?;
    let config = repo.config()?;
    let remote_name = config
        .get_string(&format!("branch.{branch}.remote"))
        .unwrap_or_else(|_| "origin".into());
    let destination = config
        .get_string(&format!("branch.{branch}.merge"))
        .unwrap_or_else(|_| reference.to_owned());
    if !destination.starts_with("refs/heads/") || !git2::Reference::is_valid_name(&destination) {
        bail!("푸시 대상 브랜치 설정을 확인하세요.");
    }
    push_to(repo, reference, &remote_name, &destination)
}

pub(super) fn push_to(
    repo: &Repository,
    reference: &str,
    remote_name: &str,
    destination: &str,
) -> Result<()> {
    if !(destination.starts_with("refs/heads/") || destination.starts_with("refs/tags/")) || !git2::Reference::is_valid_name(destination) {
        bail!("푸시 대상 참조 설정을 확인하세요.");
    }
    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| anyhow::anyhow!("푸시할 원격 저장소가 없습니다. 원격 설정을 확인하세요."))?;
    let rejected = Arc::new(AtomicBool::new(false));
    let remote_rejected = rejected.clone();
    let mut callbacks = credentials();
    let tag_push = destination.starts_with("refs/tags/");
    callbacks.push_negotiation(move |updates| {
        if tag_push && updates.iter().any(|update| !update.src().is_zero() && update.src() != update.dst()) {
            return Err(git2::Error::from_str("기존 원격 태그는 덮어쓸 수 없습니다."));
        }
        Ok(())
    });
    callbacks.push_update_reference(move |_, status| {
        if status.is_some() {
            remote_rejected.store(true, Ordering::Relaxed);
        }
        Ok(())
    });
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(callbacks);
    remote.push(&[format!("{reference}:{destination}")],Some(&mut options))
        .map_err(|_|anyhow::anyhow!("원격 연결·인증 또는 참조 거부로 푸시하지 못했습니다. 강제 푸시는 수행하지 않았습니다."))?;
    if rejected.load(Ordering::Relaxed) {
        bail!("원격 서버가 푸시를 거부했습니다. 원격 참조와 권한을 확인하세요.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = std::env::current_dir().unwrap().join("target/commit-tests");
        std::fs::create_dir_all(&root).unwrap();
        let temp = tempfile::tempdir_in(root).unwrap();
        let path = temp.path().join("local");
        let repo = Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Current User").unwrap();
        config
            .set_str("user.email", "current@example.invalid")
            .unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        (temp, path)
    }
    fn stage(path: &Path, file: &str, text: &str) {
        std::fs::write(path.join(file), text).unwrap();
        let repo = Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
    }
    fn request(path: &Path, message: &str, amend: bool, push: bool) -> CommitRequest {
        let draft = prepare_commit(path).unwrap();
        CommitRequest {
            expected_head: draft.head,
            expected_reference: draft.reference,
            message: message.into(),
            amend,
            push,
        }
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
    fn raw_commit(path: &Path, message: &str, author: &str) -> Oid {
        let repo = Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = head_commit(&repo).unwrap();
        let signature = git2::Signature::now(author, "author@example.invalid").unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
    }
    #[test]
    fn unresolved_conflict_is_rejected_without_losing_the_conflict_state() {
        let (_temp, path) = fixture();
        stage(&path, "base.txt", "base\n");
        let base = raw_commit(&path, "base", "Original");
        stage(&path, "base.txt", "ours\n");
        let ours = raw_commit(&path, "ours", "Original");
        let pending = request(&path, "resolve", false, false);
        let repo = Repository::open(&path).unwrap();
        let parent = repo.find_commit(base).unwrap();
        let old_tree = parent.tree().unwrap();
        let mut builder = repo.treebuilder(Some(&old_tree)).unwrap();
        builder
            .insert("base.txt", repo.blob(b"theirs\n").unwrap(), 0o100644)
            .unwrap();
        let tree = repo.find_tree(builder.write().unwrap()).unwrap();
        let sig = git2::Signature::now("Other", "other@example.invalid").unwrap();
        let theirs = repo
            .commit(
                Some("refs/heads/other"),
                &sig,
                &sig,
                "theirs",
                &tree,
                &[&parent],
            )
            .unwrap();
        repo.merge(&[&repo.find_annotated_commit(theirs).unwrap()], None, None)
            .unwrap();
        assert!(repo.index().unwrap().has_conflicts());
        assert!(prepare_commit(&path).is_err());
        assert!(commit_staged(&path, pending).is_err());
        assert_eq!(head(&path), ours);
        assert!(repo.index().unwrap().has_conflicts());
        assert_eq!(repo.state(), RepositoryState::Merge);
    }
    #[test]
    fn push_uses_configured_upstream_remote_and_target_branch() {
        let (temp, path) = fixture();
        let remote_path = temp.path().join("upstream.git");
        let remote = Repository::init_bare(&remote_path).unwrap();
        remote.set_head("refs/heads/release").unwrap();
        let repo = Repository::open(&path).unwrap();
        repo.remote("upstream", remote_path.to_str().unwrap())
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("branch.main.remote", "upstream")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("branch.main.merge", "refs/heads/release")
            .unwrap();
        stage(&path, "a.txt", "one");
        let outcome = commit_staged(&path, request(&path, "first", false, true)).unwrap();
        assert!(outcome.error.is_none());
        assert_eq!(
            remote
                .find_reference("refs/heads/release")
                .unwrap()
                .target(),
            Some(head(&path))
        );
        assert!(remote.find_reference("refs/heads/main").is_err());
    }
    #[test]
    fn staged_only_initial_and_followup_commits_leave_worktree_and_untracked_files_alone() {
        let (_temp, path) = fixture();
        stage(&path, "file.txt", "staged\n");
        std::fs::write(path.join("file.txt"), "unstaged\n").unwrap();
        std::fs::write(path.join("untracked.txt"), "untracked").unwrap();
        let index = std::fs::read(path.join(".git/index")).unwrap();
        let outcome = commit_staged(&path, request(&path, "제목\n\n본문😀", false, false)).unwrap();
        assert!(outcome.error.is_none());
        let repo = Repository::open(&path).unwrap();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(commit.parent_count(), 0);
        assert_eq!(commit.message().unwrap(), "제목\n\n본문😀");
        let tree = commit.tree().unwrap();
        let entry = tree.get_name("file.txt").unwrap();
        assert_eq!(repo.find_blob(entry.id()).unwrap().content(), b"staged\n");
        assert!(tree.get_name("untracked.txt").is_none());
        assert_eq!(std::fs::read(path.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(path.join("file.txt")).unwrap(),
            "unstaged\n"
        );
        let before = commit.id();
        stage(&path, "second.txt", "second");
        let outcome = commit_staged(&path, request(&path, "second", false, false)).unwrap();
        let second = repo
            .find_commit(Oid::from_str(&outcome.id).unwrap())
            .unwrap();
        assert_eq!(second.parent_id(0).unwrap(), before);
    }
    #[test]
    fn amend_preserves_author_and_parent_and_allows_message_only_changes() {
        let (_temp, path) = fixture();
        stage(&path, "a.txt", "base");
        let parent = raw_commit(&path, "base", "Original Author");
        stage(&path, "a.txt", "next");
        let before = raw_commit(&path, "old subject", "Original Author");
        let outcome = commit_staged(
            &path,
            request(&path, "new subject\n\nnew body", true, false),
        )
        .unwrap();
        let repo = Repository::open(&path).unwrap();
        let amended = repo
            .find_commit(Oid::from_str(&outcome.id).unwrap())
            .unwrap();
        assert_ne!(amended.id(), before);
        assert_eq!(amended.parent_id(0).unwrap(), parent);
        assert_eq!(amended.author().name().unwrap(), "Original Author");
        assert_eq!(amended.committer().name().unwrap(), "Current User");
        assert_eq!(amended.message().unwrap(), "new subject\n\nnew body");
    }
    #[test]
    fn invalid_messages_no_changes_amend_without_head_and_existing_locks_do_not_commit() {
        let (_temp, path) = fixture();
        assert!(!prepare_commit(&path).unwrap().can_amend);
        for (message, amend) in [
            ("", false),
            (" \n\t", false),
            ("subject", false),
            ("subject", true),
        ] {
            assert!(commit_staged(&path, request(&path, message, amend, false)).is_err());
        }
        stage(&path, "a.txt", "content");
        std::fs::write(path.join(".git/index.lock"), "external lock").unwrap();
        assert!(commit_staged(&path, request(&path, "subject", false, false)).is_err());
        assert_eq!(
            std::fs::read_to_string(path.join(".git/index.lock")).unwrap(),
            "external lock"
        );
        assert!(Repository::open(&path).unwrap().head().is_err());
    }
    #[test]
    fn stale_head_or_missing_identity_does_not_create_another_commit() {
        let (_temp, path) = fixture();
        stage(&path, "a.txt", "one");
        raw_commit(&path, "one", "Original");
        stage(&path, "a.txt", "two");
        let stale = request(&path, "stale", false, false);
        let external = raw_commit(&path, "external", "Other");
        assert!(commit_staged(&path, stale).is_err());
        assert_eq!(head(&path), external);
        stage(&path, "a.txt", "three");
        Repository::open(&path)
            .unwrap()
            .config()
            .unwrap()
            .set_str("user.name", "")
            .unwrap();
        assert!(commit_staged(&path, request(&path, "new", false, false)).is_err());
        assert_eq!(head(&path), external);
    }
    #[test]
    fn commit_push_and_amend_push_rejection_preserve_the_local_commit() {
        let (temp, path) = fixture();
        let remote_path = temp.path().join("remote.git");
        let remote = Repository::init_bare(&remote_path).unwrap();
        remote.set_head("refs/heads/main").unwrap();
        let repo = Repository::open(&path).unwrap();
        repo.remote("origin", remote_path.to_str().unwrap())
            .unwrap();
        stage(&path, "a.txt", "one");
        let first = commit_staged(&path, request(&path, "first", false, true)).unwrap();
        assert!(first.error.is_none());
        let published = Oid::from_str(&first.id).unwrap();
        assert_eq!(
            remote.find_reference("refs/heads/main").unwrap().target(),
            Some(published)
        );
        let amended = commit_staged(&path, request(&path, "amended", true, true)).unwrap();
        assert!(amended.error.is_some());
        assert_eq!(head(&path), Oid::from_str(&amended.id).unwrap());
        assert_eq!(
            remote.find_reference("refs/heads/main").unwrap().target(),
            Some(published)
        );
    }
    #[test]
    fn push_failure_reports_partial_success_without_exposing_remote_path() {
        let (temp, path) = fixture();
        Repository::open(&path)
            .unwrap()
            .remote(
                "origin",
                temp.path().join("secret-token/missing").to_str().unwrap(),
            )
            .unwrap();
        stage(&path, "a.txt", "one");
        let outcome = commit_staged(&path, request(&path, "first", false, true)).unwrap();
        assert_eq!(head(&path), Oid::from_str(&outcome.id).unwrap());
        let error = outcome.error.unwrap();
        assert!(error.contains("커밋은 완료"));
        assert!(!error.contains("secret-token"));
    }
    #[test]
    fn resolved_merge_commit_keeps_both_parents_and_cleans_merge_state() {
        let (_temp, path) = fixture();
        stage(&path, "base.txt", "base");
        let base = raw_commit(&path, "base", "Original");
        stage(&path, "ours.txt", "ours");
        let ours = raw_commit(&path, "ours", "Original");
        let repo = Repository::open(&path).unwrap();
        let parent = repo.find_commit(base).unwrap();
        let base_tree = parent.tree().unwrap();
        let mut builder = repo.treebuilder(Some(&base_tree)).unwrap();
        let blob = repo.blob(b"theirs").unwrap();
        builder.insert("theirs.txt", blob, 0o100644).unwrap();
        let tree = repo.find_tree(builder.write().unwrap()).unwrap();
        let sig = git2::Signature::now("Other", "other@example.invalid").unwrap();
        let theirs = repo
            .commit(
                Some("refs/heads/other"),
                &sig,
                &sig,
                "theirs",
                &tree,
                &[&parent],
            )
            .unwrap();
        repo.merge(&[&repo.find_annotated_commit(theirs).unwrap()], None, None)
            .unwrap();
        let draft = prepare_commit(&path).unwrap();
        assert!(draft.is_merge);
        assert!(!draft.can_amend);
        let outcome = commit_staged(&path, request(&path, "merge", false, false)).unwrap();
        let merged = repo
            .find_commit(Oid::from_str(&outcome.id).unwrap())
            .unwrap();
        assert_eq!(merged.parent_count(), 2);
        assert_eq!(merged.parent_id(0).unwrap(), ours);
        assert_eq!(merged.parent_id(1).unwrap(), theirs);
        assert_eq!(repo.state(), RepositoryState::Clean);
    }
}
