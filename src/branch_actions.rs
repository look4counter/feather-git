use super::{committing::{push_to, push_to_with_lease}, credentials, open_exact};
use anyhow::{Context, Result, bail};
use git2::{Branch, RepositoryState};
use std::path::Path;

#[derive(Debug)]
pub struct BranchOutcome {
    pub error: Option<String>,
}

pub fn push_branch(path: &Path) -> Result<()> {
    push_branch_options(path, false)
}

pub fn push_branch_with_lease(path: &Path) -> Result<()> {
    push_branch_options(path, true)
}

fn push_branch_options(path: &Path, force_with_lease: bool) -> Result<()> {
    let repo = open_exact(path)?;
    let head = match repo.head() {
        Ok(head) => head,
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => return Ok(()),
        Err(error) => return Err(error).context("푸시할 커밋을 읽을 수 없습니다."),
    };
    if !head.is_branch() {
        bail!("로컬 브랜치를 체크아웃한 뒤 Push를 실행하세요.");
    }
    let reference = head
        .name()
        .context("현재 브랜치 이름을 읽을 수 없습니다.")?;
    let name = reference
        .strip_prefix("refs/heads/")
        .context("로컬 브랜치가 아닙니다.")?;
    let mut config = repo.config()?;
    let upstream = config
        .get_string(&format!("branch.{name}.remote"))
        .ok()
        .zip(config.get_string(&format!("branch.{name}.merge")).ok());
    let (remote, destination) = upstream
        .clone()
        .unwrap_or_else(|| ("origin".into(), reference.into()));
    if force_with_lease {
        // 네트워크 연결 전에 로컬 추적 값을 고정한다.
        let configured_remote = repo.find_remote(&remote)
            .context("푸시할 원격 저장소를 찾지 못했습니다.")?;
        let tracking = configured_remote.refspecs()
            .find(|spec| spec.direction() == git2::Direction::Fetch && spec.src_matches(&destination))
            .map(|spec| spec.transform(&destination)).transpose()?;
        let expected = match tracking {
            Some(tracking) => {
                let name = tracking.as_str().context("원격 추적 참조 이름을 읽지 못했습니다.")?;
                match repo.refname_to_id(name) {
                    Ok(id) => id,
                    Err(error) if error.code() == git2::ErrorCode::NotFound => git2::Oid::ZERO_SHA1,
                    Err(error) => return Err(error).context("원격 추적 참조를 읽지 못했습니다."),
                }
            }
            None => git2::Oid::ZERO_SHA1,
        };
        push_to_with_lease(&repo, reference, &remote, &destination, expected)?;
    } else if !push_is_empty(&repo, reference, &remote, &destination)? {
        push_to(&repo, reference, &remote, &destination)?;
    }
    if upstream.is_none() {
        config.set_str(&format!("branch.{name}.remote"), &remote)
            .and_then(|_| config.set_str(&format!("branch.{name}.merge"), &destination))
            .map_err(|_| anyhow::anyhow!("푸시는 완료됐지만 추적 브랜치를 설정하지 못했습니다. 원격 브랜치는 생성되어 있습니다."))?;
    }
    Ok(())
}

fn push_is_empty(
    repo: &git2::Repository,
    reference: &str,
    remote_name: &str,
    destination: &str,
) -> Result<bool> {
    if !destination.starts_with("refs/heads/") || !git2::Reference::is_valid_name(destination) {
        bail!("푸시 대상 브랜치 설정을 확인하세요.");
    }
    let local = repo.find_reference(reference)?.peel_to_commit()?.id();
    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| anyhow::anyhow!("푸시할 원격 저장소가 없습니다. 원격 설정을 확인하세요."))?;
    let remote_id = {
        let connection = remote
            .connect_auth(git2::Direction::Push, Some(credentials()), None)
            .map_err(|_| {
                anyhow::anyhow!("원격 연결 또는 인증에 실패하여 푸시 상태를 확인하지 못했습니다.")
            })?;
        connection
            .list()
            .map_err(|_| anyhow::anyhow!("원격 브랜치 상태를 읽지 못했습니다."))?
            .iter()
            .find(|head| head.name() == destination)
            .map(|head| head.oid())
    };
    let Some(remote_id) = remote_id else {
        return Ok(false);
    };
    if local == remote_id {
        return Ok(true);
    }
    if repo.find_commit(remote_id).is_err() {
        let mut options = git2::FetchOptions::new();
        options
            .remote_callbacks(credentials())
            .download_tags(git2::AutotagOption::None)
            .update_fetchhead(false);
        remote
            .download(&[destination], Some(&mut options))
            .map_err(|_| {
                anyhow::anyhow!("원격 커밋 상태를 읽지 못했습니다. 연결·인증을 확인하세요.")
            })?;
        remote
            .disconnect()
            .map_err(|_| anyhow::anyhow!("원격 연결을 종료하지 못했습니다."))?;
    }
    let (ahead, _) = repo
        .graph_ahead_behind(local, remote_id)
        .context("푸시할 커밋 상태를 비교하지 못했습니다.")?;
    Ok(ahead == 0)
}

pub fn checkout_reference(path: &Path, reference: &str) -> Result<()> {
    checkout_reference_checked(path, reference, None)
}

pub fn checkout_commit_branch(path: &Path, reference: &str, expected: &str) -> Result<()> {
    if !reference.starts_with("refs/heads/") { anyhow::bail!("체크아웃할 로컬 브랜치를 선택하세요."); }
    let expected = git2::Oid::from_str(expected)?;
    checkout_reference_checked(path, reference, Some(expected))
}

fn checkout_reference_checked(path: &Path, reference: &str, expected: Option<git2::Oid>) -> Result<()> {
    let local = reference.starts_with("refs/heads/");
    let commit_id = (reference.len() == 40).then(|| git2::Oid::from_str(reference).ok()).flatten();
    if commit_id.is_none() && (!git2::Reference::is_valid_name(reference)
        || !(local || reference.starts_with("refs/remotes/") || reference.starts_with("refs/tags/")))
    {
        bail!("체크아웃할 브랜치 또는 태그를 확인하세요.");
    }
    let repo = open_exact(path)?;
    if repo.is_bare() || repo.state() != RepositoryState::Clean {
        bail!("진행 중인 병합·리베이스를 마친 뒤 작업 폴더에서 체크아웃하세요.");
    }
    match std::fs::symlink_metadata(repo.path().join("index.lock")) {
        Ok(_) => bail!("인덱스가 잠겨 있어 체크아웃할 수 없습니다."),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("인덱스 잠금 상태를 확인하지 못했습니다."),
    }
    if repo.index()?.has_conflicts() {
        bail!("미해결 충돌을 먼저 해결하세요.");
    }
    let target = if let Some(id) = commit_id {
        repo.find_commit(id).context("체크아웃할 커밋을 읽을 수 없습니다.")?
    } else {
        repo.find_reference(reference)
            .context("체크아웃할 참조를 찾을 수 없습니다.")?
            .peel_to_commit()
            .context("대상 참조의 커밋을 읽을 수 없습니다.")?
    };
    if expected.is_some_and(|expected| expected != target.id()) { anyhow::bail!("선택한 브랜치의 커밋이 변경되었습니다. 목록을 갱신한 뒤 다시 선택하세요."); }
    let mut transaction = repo.transaction()?;
    transaction
        .lock_ref("HEAD")
        .context("HEAD가 잠겨 있어 체크아웃할 수 없습니다.")?;
    if local {
        transaction
            .lock_ref(reference)
            .context("대상 브랜치가 잠겨 있어 체크아웃할 수 없습니다.")?;
        if repo.find_reference(reference)?.peel_to_commit()?.id() != target.id() {
            bail!("대상 브랜치가 변경되었습니다. 다시 실행하세요.");
        }
        transaction.set_symbolic_target("HEAD", reference, None, "checkout: branch")?;
    } else {
        transaction.set_target("HEAD", target.id(), None, "checkout: commit")?;
    }
    repo.checkout_tree(
        target.as_object(),
        Some(git2::build::CheckoutBuilder::new().safe()),
    )
    .context(
        "로컬 변경사항·미추적 파일과의 충돌 또는 인덱스 잠금 때문에 체크아웃하지 못했습니다.",
    )?;
    transaction
        .commit()
        .context("작업 파일 갱신 후 HEAD를 저장하지 못했습니다. 저장소 상태를 확인하세요.")?;
    Ok(())
}

pub fn create_tracking_branch(path: &Path, name: &str, remote: &str) -> Result<BranchOutcome> {
    let name = name.trim();
    if !Branch::name_is_valid(name).unwrap_or(false) || name.starts_with('-') || name == "HEAD" {
        bail!("올바른 브랜치명을 입력하세요.");
    }
    let Some(upstream) = remote.strip_prefix("refs/remotes/") else { bail!("원격 추적 브랜치를 선택하세요."); };
    let repo = open_exact(path)?;
    if repo.is_bare() || repo.state() != RepositoryState::Clean || repo.index()?.has_conflicts() {
        bail!("진행 중인 병합·리베이스와 충돌을 먼저 해결하세요.");
    }
    match repo.find_branch(name, git2::BranchType::Local) {
        Ok(branch) => {
            checkout_reference(path, branch.get().name()?)?;
            return Ok(BranchOutcome { error: None });
        },
        Err(error) if error.code() == git2::ErrorCode::NotFound => {},
        Err(error) => return Err(error.into()),
    }
    let commit = repo.find_reference(remote)?.peel_to_commit()?;
    let mut branch = repo.branch(name, &commit, false).context("브랜치를 만들지 못했습니다. 이름 중복·잠금·권한을 확인하세요.")?;
    if let Err(error) = branch.set_upstream(Some(upstream)) {
        return Ok(BranchOutcome { error: Some(format!("브랜치는 생성됐지만 원격 추적 설정에 실패했습니다: {error}")) });
    }
    let reference = branch.get().name().context("생성된 브랜치 이름을 읽을 수 없습니다.")?.to_owned();
    match checkout_reference(path, &reference) {
        Ok(()) => Ok(BranchOutcome { error: None }),
        Err(error) => Ok(BranchOutcome { error: Some(format!("추적 브랜치는 생성됐지만 체크아웃하지 못했습니다. 생성된 브랜치는 유지됩니다: {error:#}")) }),
    }
}

pub fn create_branch(path: &Path, name: &str, checkout: bool) -> Result<BranchOutcome> {
    let name = name.trim();
    if !Branch::name_is_valid(name).unwrap_or(false) || name.starts_with('-') || name == "HEAD" {
        bail!("올바른 브랜치명을 입력하세요.");
    }
    let repo = open_exact(path)?;
    if repo.is_bare() || repo.state() != RepositoryState::Clean {
        bail!("작업 폴더가 있는 저장소에서 진행 중인 병합·리베이스를 마친 뒤 실행하세요.");
    }
    let head = repo.head().context("브랜치를 생성할 커밋이 없습니다.")?;
    let commit = head
        .peel_to_commit()
        .context("현재 커밋을 읽을 수 없습니다.")?;
    let branch = repo
        .branch(name, &commit, false)
        .context("브랜치를 만들지 못했습니다. 이름 중복·잠금·권한을 확인하세요.")?;
    let mut outcome = BranchOutcome { error: None };
    if checkout {
        let reference = branch
            .get()
            .name()
            .context("생성된 브랜치 이름을 읽을 수 없습니다.")?;
        if repo.set_head(reference).is_err() {
            outcome.error = Some(
                "브랜치는 생성됐지만 체크아웃하지 못했습니다. 생성된 브랜치는 유지됩니다.".into(),
            );
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    fn fixture() -> (tempfile::TempDir, Repository) {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/branch-action-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repo = Repository::init(dir.path().join("local")).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        (dir, repo)
    }
    fn commit(repo: &Repository) -> git2::Oid {
        let tree = repo
            .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        let parent = repo.head().ok().map(|h| h.peel_to_commit().unwrap());
        let parents = parent.iter().collect::<Vec<_>>();
        repo.commit(Some("HEAD"), &sig, &sig, "message", &tree, &parents)
            .unwrap()
    }
    #[test]
    fn history_checkout_branch_stays_attached_and_rejects_moved_reference() {
        let (_dir, repo) = fixture();
        let old = file_commit(&repo, "old");
        repo.branch("old", &repo.find_commit(old).unwrap(), false).unwrap();
        let new = file_commit(&repo, "new");
        let path = repo.workdir().unwrap();
        checkout_commit_branch(path, "refs/heads/old", &old.to_string()).unwrap();
        assert!(!repo.head_detached().unwrap());
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/old");
        checkout_reference(path, "refs/heads/main").unwrap();
        repo.find_reference("refs/heads/old").unwrap().set_target(new, "moved").unwrap();
        assert!(checkout_commit_branch(path, "refs/heads/old", &old.to_string()).is_err());
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
        assert_eq!(std::fs::read_to_string(path.join("tracked")).unwrap(), "new");
    }
    #[test]
    fn branch_creation_checkout_preserves_index_and_worktree() {
        let (_dir, repo) = fixture();
        let id = commit(&repo);
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("file"), "staged").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file")).unwrap();
        index.write().unwrap();
        std::fs::write(path.join("file"), "unstaged").unwrap();
        std::fs::write(path.join("untracked"), "keep").unwrap();
        let bytes = std::fs::read(repo.path().join("index")).unwrap();
        create_branch(path, "feature/one", false).unwrap();
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
        assert_eq!(
            repo.find_reference("refs/heads/feature/one")
                .unwrap()
                .target(),
            Some(id)
        );
        create_branch(path, "feature/two", true).unwrap();
        assert_eq!(
            repo.head().unwrap().name().unwrap(),
            "refs/heads/feature/two"
        );
        assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), bytes);
        assert_eq!(
            std::fs::read_to_string(path.join("file")).unwrap(),
            "unstaged"
        );
        assert_eq!(
            std::fs::read_to_string(path.join("untracked")).unwrap(),
            "keep"
        );
        assert!(create_branch(path, "feature/two", false).is_err());
    }
    #[test]
    fn invalid_names_unborn_state_and_head_lock_are_handled() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        assert!(create_branch(path, "new", false).is_err());
        for name in ["", "bad name", "a..b", "-bad", "HEAD", "a.lock"] {
            assert!(create_branch(path, name, false).is_err());
        }
        let id = commit(&repo);
        std::fs::write(repo.path().join("MERGE_HEAD"), format!("{id}\n")).unwrap();
        assert!(create_branch(path, "merging", false).is_err());
        std::fs::remove_file(repo.path().join("MERGE_HEAD")).unwrap();
        std::fs::write(repo.path().join("HEAD.lock"), "").unwrap();
        let result = create_branch(path, "created", true).unwrap();
        assert!(result.error.is_some());
        assert!(repo.find_reference("refs/heads/created").is_ok());
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/main");
    }
    #[test]
    fn force_with_lease_rewrites_only_when_tracking_matches_remote() {
        let (dir, repo) = fixture();
        let first = file_commit(&repo, "first");
        let remote = Repository::init_bare(dir.path().join("remote")).unwrap();
        repo.remote("origin", remote.path().to_str().unwrap()).unwrap();
        let path = repo.workdir().unwrap();
        push_branch_with_lease(path).unwrap();
        assert_eq!(repo.config().unwrap().get_string("branch.main.remote").unwrap(), "origin");
        assert_eq!(repo.config().unwrap().get_string("branch.main.merge").unwrap(), "refs/heads/main");
        let second = file_commit(&repo, "second");
        push_branch(path).unwrap();
        repo.reference("refs/heads/main", first, true, "rewrite").unwrap();
        let rewritten = file_commit(&repo, "rewritten");
        assert!(push_branch(path).is_err());
        push_branch_with_lease(path).unwrap();
        assert_eq!(remote.refname_to_id("refs/heads/main").unwrap(), rewritten);
        assert_ne!(second, rewritten);
        let server = file_commit(&repo, "server");
        push_branch(path).unwrap();
        repo.reference("refs/remotes/origin/main", rewritten, true, "stale").unwrap();
        repo.reference("refs/heads/main", first, true, "rewrite").unwrap();
        file_commit(&repo, "stale rewrite");
        let error = push_branch_with_lease(path).unwrap_err().to_string();
        assert!(error.contains("원격 추적"), "{error}");
        assert_eq!(remote.refname_to_id("refs/heads/main").unwrap(), server);
        assert_eq!(repo.refname_to_id("refs/remotes/origin/main").unwrap(), rewritten);
        remote.find_reference("refs/heads/main").unwrap().delete().unwrap();
        assert!(push_branch_with_lease(path).is_err());
        assert!(remote.find_reference("refs/heads/main").is_err());
    }

    #[test]
    fn force_with_lease_missing_tracking_only_creates_absent_remote_branch() {
        let (dir, repo) = fixture();
        let first = file_commit(&repo, "first");
        let remote = Repository::init_bare(dir.path().join("remote")).unwrap();
        repo.remote("custom", remote.path().to_str().unwrap()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("branch.main.remote", "custom").unwrap();
        config.set_str("branch.main.merge", "refs/heads/target").unwrap();
        config.set_str("remote.custom.fetch", "+refs/heads/*:refs/remotes/mapped/*").unwrap();
        let path = repo.workdir().unwrap();
        push_branch_with_lease(path).unwrap();
        assert_eq!(remote.refname_to_id("refs/heads/target").unwrap(), first);
        let second = file_commit(&repo, "second");
        repo.find_reference("refs/remotes/mapped/target").unwrap().delete().unwrap();
        assert!(push_branch_with_lease(path).is_err());
        assert_eq!(remote.refname_to_id("refs/heads/target").unwrap(), first);
        repo.reference("refs/remotes/mapped/target", first, true, "known").unwrap();
        push_branch_with_lease(path).unwrap();
        assert_eq!(remote.refname_to_id("refs/heads/target").unwrap(), second);
    }
    #[test]
    fn initial_push_sets_upstream_existing_upstream_is_used_and_rejection_is_safe() {
        let (dir, repo) = fixture();
        commit(&repo);
        let path = repo.workdir().unwrap();
        let remote = Repository::init_bare(dir.path().join("remote")).unwrap();
        repo.remote("origin", remote.path().to_str().unwrap())
            .unwrap();
        push_branch(path).unwrap();
        let cfg = repo.config().unwrap();
        assert_eq!(cfg.get_string("branch.main.remote").unwrap(), "origin");
        assert_eq!(
            cfg.get_string("branch.main.merge").unwrap(),
            "refs/heads/main"
        );
        let next = commit(&repo);
        push_branch(path).unwrap();
        assert_eq!(
            remote.find_reference("refs/heads/main").unwrap().target(),
            Some(next)
        );
        let other = Repository::init_bare(dir.path().join("other")).unwrap();
        repo.remote("custom", other.path().to_str().unwrap())
            .unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("branch.main.remote", "custom").unwrap();
        cfg.set_str("branch.main.merge", "refs/heads/target")
            .unwrap();
        push_branch(path).unwrap();
        assert_eq!(
            other.find_reference("refs/heads/target").unwrap().target(),
            Some(next)
        );
        let old = repo.find_commit(next).unwrap().parent_id(0).unwrap();
        repo.reference("refs/heads/main", old, true, "test")
            .unwrap();
        // 로컬이 뒤에만 있는 경우에는 전송을 생략한다.
        push_branch(path).unwrap();
        let parent = repo.find_commit(old).unwrap();
        let tree = parent.tree().unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "diverged", &tree, &[&parent])
            .unwrap();
        assert!(push_branch(path).is_err());
        assert_eq!(
            other.find_reference("refs/heads/target").unwrap().target(),
            Some(next)
        );
    }
    #[test]
    fn push_ignores_unborn_but_rejects_detached_and_missing_remote_without_setting_upstream() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        assert!(push_branch(path).is_ok());
        let id = commit(&repo);
        assert!(push_branch(path).is_err());
        assert!(
            repo.config()
                .unwrap()
                .get_string("branch.main.remote")
                .is_err()
        );
        repo.set_head_detached(id).unwrap();
        assert!(push_branch(path).is_err());
    }
    fn file_commit(repo: &Repository, text: &str) -> git2::Oid {
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("tracked"), text).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("tracked")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        let parent = repo.head().ok().map(|h| h.peel_to_commit().unwrap());
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
    #[test]
    fn push_same_and_unknown_remote_descendant_are_ignored_without_updating_refs_or_fetch_head() {
        let (dir, repo) = fixture();
        let first = commit(&repo);
        let path = repo.workdir().unwrap();
        let remote = Repository::init_bare(dir.path().join("remote-noop")).unwrap();
        repo.remote("origin", remote.path().to_str().unwrap())
            .unwrap();
        push_branch(path).unwrap();
        push_branch(path).unwrap();
        let tree = remote.find_commit(first).unwrap().tree().unwrap();
        let sig = Signature::now("Remote", "remote@example.invalid").unwrap();
        let parent = remote.find_commit(first).unwrap();
        let next = remote
            .commit(
                Some("refs/heads/main"),
                &sig,
                &sig,
                "remote only",
                &tree,
                &[&parent],
            )
            .unwrap();
        assert!(repo.find_commit(next).is_err());
        std::fs::write(repo.path().join("FETCH_HEAD"), "preserve").unwrap();
        let tracking = repo
            .find_reference("refs/remotes/origin/main")
            .unwrap()
            .target();
        push_branch(path).unwrap();
        assert_eq!(repo.head().unwrap().target(), Some(first));
        assert_eq!(
            remote.find_reference("refs/heads/main").unwrap().target(),
            Some(next)
        );
        assert_eq!(
            repo.find_reference("refs/remotes/origin/main")
                .unwrap()
                .target(),
            tracking
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("FETCH_HEAD")).unwrap(),
            "preserve"
        );
    }
    #[test]
    fn checkout_local_remote_and_both_tag_types_updates_files_and_head_kind() {
        let (_dir, repo) = fixture();
        let old = file_commit(&repo, "old");
        repo.branch("old", &repo.find_commit(old).unwrap(), false)
            .unwrap();
        let new = file_commit(&repo, "new");
        let path = repo.workdir().unwrap();
        repo.reference("refs/remotes/origin/old", old, true, "test")
            .unwrap();
        repo.tag_lightweight("light", repo.find_commit(new).unwrap().as_object(), false)
            .unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        repo.tag(
            "annotated",
            repo.find_commit(old).unwrap().as_object(),
            &sig,
            "tag",
            false,
        )
        .unwrap();
        checkout_reference(path, "refs/heads/old").unwrap();
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/old");
        assert!(!repo.head_detached().unwrap());
        assert_eq!(
            std::fs::read_to_string(path.join("tracked")).unwrap(),
            "old"
        );
        checkout_reference(path, "refs/tags/light").unwrap();
        assert!(repo.head_detached().unwrap());
        assert_eq!(repo.head().unwrap().target(), Some(new));
        assert_eq!(
            std::fs::read_to_string(path.join("tracked")).unwrap(),
            "new"
        );
        for target in ["refs/remotes/origin/old", "refs/tags/annotated"] {
            checkout_reference(path, target).unwrap();
            assert!(repo.head_detached().unwrap());
            assert_eq!(repo.head().unwrap().target(), Some(old));
        }
        checkout_reference(path, "refs/heads/main").unwrap();
        assert!(!repo.head_detached().unwrap());
        assert_eq!(repo.head().unwrap().target(), Some(new));
    }
    #[test]
    fn tracking_branch_uses_remote_tip_and_upstream() {
        let (_dir, repo) = fixture();
        let old = file_commit(&repo, "old");
        let new = file_commit(&repo, "new");
        let path = repo.workdir().unwrap();
        repo.remote("origin", path.to_str().unwrap()).unwrap();
        repo.reference("refs/remotes/origin/topic", old, true, "fixture").unwrap();
        assert!(create_tracking_branch(path, "local-topic", "refs/remotes/origin/topic").unwrap().error.is_none());
        assert_eq!(repo.head().unwrap().name().unwrap(), "refs/heads/local-topic");
        assert_eq!(repo.head().unwrap().target(), Some(old));
        assert_eq!(repo.find_branch("local-topic", git2::BranchType::Local).unwrap().upstream().unwrap().get().name().unwrap(), "refs/remotes/origin/topic");
        assert_eq!(std::fs::read_to_string(path.join("tracked")).unwrap(), "old");
        assert!(create_tracking_branch(path, "local-topic", "refs/remotes/origin/topic").unwrap().error.is_none());
        checkout_reference(path, "refs/heads/main").unwrap();
        std::fs::write(path.join("tracked"), "local").unwrap();
        let outcome = create_tracking_branch(path, "blocked", "refs/remotes/origin/topic").unwrap();
        assert!(outcome.error.is_some());
        let previous = repo.find_branch("main", git2::BranchType::Local).unwrap().upstream().ok().map(|branch| branch.get().name().unwrap().to_owned());
        assert!(create_tracking_branch(path, "main", "refs/remotes/origin/topic").unwrap().error.is_none());
        assert_eq!(repo.find_branch("main", git2::BranchType::Local).unwrap().upstream().ok().map(|branch| branch.get().name().unwrap().to_owned()), previous);
        assert_eq!(repo.head().unwrap().target(), Some(new));
        assert_eq!(std::fs::read_to_string(path.join("tracked")).unwrap(), "local");
    }

    #[test]
    fn tracking_branch_rejects_missing_remote() {
        let (_dir, repo) = fixture();
        file_commit(&repo, "old");
        assert!(create_tracking_branch(repo.workdir().unwrap(), "topic", "refs/remotes/origin/missing").is_err());
    }

    #[test]
    fn checkout_exact_commit_detaches_and_preserves_conflicting_changes() {
        let (_dir, repo) = fixture();
        let old = file_commit(&repo, "old");
        let new = file_commit(&repo, "new");
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("tracked"), "local").unwrap();
        assert!(checkout_reference(path, &old.to_string()).is_err());
        assert_eq!(repo.head().unwrap().target(), Some(new));
        assert_eq!(std::fs::read_to_string(path.join("tracked")).unwrap(), "local");
        std::fs::write(path.join("tracked"), "new").unwrap();
        checkout_reference(path, &old.to_string()).unwrap();
        assert!(repo.head_detached().unwrap());
        assert_eq!(repo.head().unwrap().target(), Some(old));
        assert_eq!(std::fs::read_to_string(path.join("tracked")).unwrap(), "old");
        for invalid in ["HEAD~1", "deadbeef", "0000000000000000000000000000000000000000"] {
            assert!(checkout_reference(path, invalid).is_err());
            assert_eq!(repo.head().unwrap().target(), Some(old));
        }
    }

    #[test]
    fn checkout_conflicts_missing_refs_and_locks_preserve_head_and_local_changes() {
        let (_dir, repo) = fixture();
        let old = file_commit(&repo, "old");
        repo.branch("old", &repo.find_commit(old).unwrap(), false)
            .unwrap();
        let new = file_commit(&repo, "new");
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("tracked"), "local").unwrap();
        let bytes = std::fs::read(repo.path().join("index")).unwrap();
        assert!(checkout_reference(path, "refs/heads/old").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(new));
        assert_eq!(
            std::fs::read_to_string(path.join("tracked")).unwrap(),
            "local"
        );
        assert_eq!(std::fs::read(repo.path().join("index")).unwrap(), bytes);
        for target in ["refs/heads/missing", "refs/stash", "HEAD"] {
            assert!(checkout_reference(path, target).is_err());
        }
        std::fs::write(path.join("tracked"), "new").unwrap();
        for lock in ["HEAD.lock", "index.lock"] {
            std::fs::write(repo.path().join(lock), "").unwrap();
            assert!(checkout_reference(path, "refs/heads/old").is_err());
            assert_eq!(repo.head().unwrap().target(), Some(new));
            assert_eq!(
                std::fs::read_to_string(path.join("tracked")).unwrap(),
                "new"
            );
            std::fs::remove_file(repo.path().join(lock)).unwrap();
        }
        std::fs::write(repo.path().join("MERGE_HEAD"), format!("{old}\n")).unwrap();
        assert!(checkout_reference(path, "refs/heads/old").is_err());
    }
    #[test]
    fn checkout_preserves_nonconflicting_changes_and_rejects_untracked_overwrite() {
        let (_dir, repo) = fixture();
        let old = file_commit(&repo, "old");
        repo.branch("old", &repo.find_commit(old).unwrap(), false)
            .unwrap();
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("added"), "tracked added").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("added")).unwrap();
        index.write().unwrap();
        let new = file_commit(&repo, "new");
        checkout_reference(path, "refs/heads/old").unwrap();
        assert_eq!(repo.head().unwrap().target(), Some(old));
        std::fs::write(path.join("added"), "untracked keep").unwrap();
        assert!(checkout_reference(path, "refs/heads/main").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(old));
        assert_eq!(
            std::fs::read_to_string(path.join("added")).unwrap(),
            "untracked keep"
        );
        std::fs::remove_file(path.join("added")).unwrap();
        std::fs::write(path.join("unrelated"), "keep").unwrap();
        checkout_reference(path, "refs/heads/main").unwrap();
        assert_eq!(repo.head().unwrap().target(), Some(new));
        assert_eq!(
            std::fs::read_to_string(path.join("unrelated")).unwrap(),
            "keep"
        );
    }
}
