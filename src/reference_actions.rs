use super::{credentials, open_exact};
use anyhow::{Context, Result, bail};
use git2::{Direction, Oid, Repository, RepositoryState};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceDeletion {
    pub reference: String,
    pub expected: String,
    pub description: Option<String>,
    pub remote: Option<(String, String)>,
}

fn stash_index(reference: &str) -> Option<usize> {
    reference
        .strip_prefix("stash@{")?
        .strip_suffix('}')?
        .parse()
        .ok()
}

fn remote_target(repo: &Repository, reference: &str) -> Result<(String, String)> {
    let mut targets = Vec::new();
    for name in repo.remotes()?.iter() {
        let Some(name) = name? else {
            continue;
        };
        let remote = repo.find_remote(name)?;
        for spec in remote.refspecs() {
            if spec.direction() == Direction::Fetch && spec.dst_matches(reference) {
                let source = spec.rtransform(reference)?;
                let source = source.as_str()?;
                if source.starts_with("refs/heads/") {
                    targets.push((name.to_owned(), source.to_owned()));
                }
            }
        }
    }
    targets.sort();
    targets.dedup();
    if targets.len() != 1 {
        bail!("삭제할 서버 브랜치를 유일하게 확인할 수 없습니다. 원격 fetch 설정을 확인하세요.");
    }
    Ok(targets.remove(0))
}

pub fn prepare_reference_deletion(path: &Path, reference: &str) -> Result<ReferenceDeletion> {
    let repo = open_exact(path)?;
    let (id, remote, description) = if let Some(index) = stash_index(reference) {
        let log = repo.reflog("refs/stash")?;
        let entry = log.get(index).context("삭제할 스태시가 없습니다.")?;
        (entry.id_new(), None, entry.message()?.map(str::to_owned))
    } else {
        if !git2::Reference::is_valid_name(reference)
            || !["refs/heads/", "refs/remotes/", "refs/tags/"]
                .iter()
                .any(|prefix| reference.starts_with(prefix))
        {
            bail!("삭제할 브랜치·태그·스태시를 확인하세요.");
        }
        let current = repo.find_reference(reference)?;
        let id = current
            .target()
            .context("별칭 참조는 삭제할 수 없습니다.")?;
        let remote = reference
            .starts_with("refs/remotes/")
            .then(|| remote_target(&repo, reference))
            .transpose()?;
        (id, remote, None)
    };
    Ok(ReferenceDeletion {
        reference: reference.into(),
        expected: id.to_string(),
        description,
        remote,
    })
}

fn check_local_branch(repo: &Repository, reference: &str, expected: Oid) -> Result<()> {
    if repo
        .head()
        .ok()
        .is_some_and(|head| head.name().ok() == Some(reference))
    {
        bail!("현재 체크아웃된 브랜치는 삭제할 수 없습니다.");
    }
    if repo.is_worktree() {
        let main = Repository::open(repo.commondir())?;
        if main
            .head()
            .ok()
            .is_some_and(|head| head.name().ok() == Some(reference))
        {
            bail!("기본 작업 트리에서 체크아웃된 브랜치는 삭제할 수 없습니다.");
        }
    }
    for name in repo.worktrees()?.iter() {
        let Some(name) = name? else {
            continue;
        };
        let worktree = repo.find_worktree(name)?;
        let other = Repository::open_from_worktree(&worktree)?;
        if other
            .head()
            .ok()
            .is_some_and(|head| head.name().ok() == Some(reference))
        {
            bail!("다른 작업 트리에서 사용 중인 브랜치는 삭제할 수 없습니다.");
        }
    }
    let branch = repo.find_branch(
        reference.trim_start_matches("refs/heads/"),
        git2::BranchType::Local,
    )?;
    let merged_into = match branch.upstream() {
        Ok(upstream) => upstream.get().peel_to_commit()?.id(),
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            repo.head()?.peel_to_commit()?.id()
        }
        Err(error) => return Err(error.into()),
    };
    if merged_into != expected && !repo.graph_descendant_of(merged_into, expected)? {
        bail!("병합되지 않은 브랜치는 삭제할 수 없습니다. 먼저 변경을 병합하세요.");
    }
    Ok(())
}

fn remove_reference(repo: &Repository, request: &ReferenceDeletion) -> Result<()> {
    let mut transaction = repo.transaction()?;
    if request.reference.starts_with("refs/heads/") {
        transaction.lock_ref("HEAD")?;
    }
    transaction.lock_ref(&request.reference)?;
    let current = repo.find_reference(&request.reference)?;
    let expected = Oid::from_str(&request.expected)?;
    if current.target() != Some(expected) {
        bail!("삭제 대상이 변경되었습니다. 목록을 갱신한 후 다시 실행하세요.");
    }
    if request.reference.starts_with("refs/heads/") {
        check_local_branch(repo, &request.reference, expected)?;
    }
    transaction.remove(&request.reference)?;
    transaction.commit()?;
    if let Some(name) = request.reference.strip_prefix("refs/heads/") {
        let mut config = repo.config()?;
        for setting in [
            "remote",
            "merge",
            "pushRemote",
            "rebase",
            "description",
            "mergeOptions",
        ] {
            let key = format!("branch.{name}.{setting}");
            match config.remove_multivar(&key, ".*") {
                Ok(()) => {}
                Err(error) if error.code() == git2::ErrorCode::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .context("브랜치는 삭제됐지만 추적 설정 정리에 실패했습니다.");
                }
            }
        }
    }
    Ok(())
}

pub fn delete_reference(path: &Path, request: ReferenceDeletion) -> Result<()> {
    delete_reference_with_remote_tag(path, request, false)
}

pub fn delete_reference_with_remote_tag(
    path: &Path,
    mut request: ReferenceDeletion,
    delete_remote_tag: bool,
) -> Result<()> {
    let repo = open_exact(path)?;
    if repo.state() != RepositoryState::Clean {
        bail!("진행 중인 병합·리베이스를 마친 뒤 삭제하세요.");
    }
    if prepare_reference_deletion(path, &request.reference)? != request {
        bail!("삭제 대상이 변경되었습니다. 다시 확인하세요.");
    }
    if delete_remote_tag {
        if !request.reference.starts_with("refs/tags/") {
            bail!("원격 태그 삭제는 태그에만 사용할 수 있습니다.");
        }
        request.remote = Some((
            super::tag_actions::tag_push_remote(&repo)?,
            request.reference.clone(),
        ));
    }
    if let Some(index) = stash_index(&request.reference) {
        let mut transaction = repo.transaction()?;
        transaction.lock_ref("refs/stash")?;
        let mut log = repo.reflog("refs/stash")?;
        if log.get(index).map(|entry| entry.id_new()) != Some(Oid::from_str(&request.expected)?) {
            bail!("스태시 순서가 변경되었습니다. 다시 확인하세요.");
        }
        log.remove(index, true)?;
        if let Some(first) = log.get(0) {
            transaction.set_target("refs/stash", first.id_new(), None, "stash: drop")?;
            transaction.set_reflog("refs/stash", log)?;
        } else {
            transaction.remove("refs/stash")?;
        }
        transaction.commit()?;
        return Ok(());
    }
    if let Some((name, destination)) = &request.remote {
        let mut remote = repo.find_remote(name)?;
        let expected = Oid::from_str(&request.expected)?;
        let destination_check = destination.clone();
        let tag_delete = destination.starts_with("refs/tags/");
        let rejected = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let server_rejected = rejected.clone();
        let mut callbacks = credentials();
        callbacks.push_negotiation(move |updates| {
            if updates.len() != 1
                || (updates[0].src() != expected && !(tag_delete && updates[0].src().is_zero()))
                || !updates[0].dst().is_zero()
                || updates[0].dst_refname()? != destination_check
            {
                return Err(git2::Error::from_str("서버 참조가 변경되었습니다."));
            }
            Ok(())
        });
        callbacks.push_update_reference(move |_, status| {
            server_rejected.store(status.is_some(), std::sync::atomic::Ordering::Relaxed);
            Ok(())
        });
        let mut options = git2::PushOptions::new();
        options.remote_callbacks(callbacks);
        remote.push(&[format!(":{destination}")], Some(&mut options))
            .map_err(|_| anyhow::anyhow!("원격 삭제 처리 중 오류가 발생했습니다. 서버 반영 여부와 인증·권한·대상 변경을 확인하고 Fetch 후 다시 시도하세요."))?;
        if rejected.load(std::sync::atomic::Ordering::Relaxed) {
            bail!("서버가 참조 삭제를 거부했습니다.");
        }
        match repo.find_reference(&request.reference) {
            Ok(_) => remove_reference(&repo, &request)
                .context("서버 참조는 삭제됐지만 로컬 참조 정리에 실패했습니다.")?,
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => {
                return Err(error).context("서버 삭제 후 로컬 추적 정보 확인에 실패했습니다.");
            }
        }
        return Ok(());
    }
    remove_reference(&repo, &request)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Repository) {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/reference-delete-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repo = Repository::init(dir.path().join("repo")).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        std::fs::write(repo.workdir().unwrap().join("file"), "base").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        drop(tree);
        (dir, repo)
    }
    #[test]
    fn local_and_tag_deletion_reject_head_unmerged_and_changed_targets() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("merged", &head, false).unwrap();
        let prepared = prepare_reference_deletion(path, "refs/heads/merged").unwrap();
        delete_reference(path, prepared).unwrap();
        assert!(repo.find_reference("refs/heads/merged").is_err());
        let prepared = prepare_reference_deletion(path, "refs/heads/main").unwrap();
        assert!(delete_reference(path, prepared).is_err());
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let id = repo
            .commit(
                Some("refs/heads/unmerged"),
                &sig,
                &sig,
                "unmerged",
                &head.tree().unwrap(),
                &[&head],
            )
            .unwrap();
        let prepared = prepare_reference_deletion(path, "refs/heads/unmerged").unwrap();
        assert!(delete_reference(path, prepared).is_err());
        repo.tag_lightweight("tag", head.as_object(), false)
            .unwrap();
        let prepared = prepare_reference_deletion(path, "refs/tags/tag").unwrap();
        repo.reference("refs/tags/tag", id, true, "change").unwrap();
        assert!(delete_reference(path, prepared).is_err());
        let prepared = prepare_reference_deletion(path, "refs/tags/tag").unwrap();
        delete_reference(path, prepared).unwrap();
        assert!(repo.find_reference("refs/tags/tag").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(head.id()));
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "base");
    }
    #[test]
    fn branches_in_other_worktrees_and_locked_tags_are_preserved() {
        let (dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        let worktree = repo
            .worktree("linked", &dir.path().join("linked"), None)
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/heads/linked").unwrap();
        assert!(delete_reference(path, request).is_err());
        let linked = Repository::open_from_worktree(&worktree).unwrap();
        let request =
            prepare_reference_deletion(linked.workdir().unwrap(), "refs/heads/main").unwrap();
        assert!(delete_reference(linked.workdir().unwrap(), request).is_err());
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("locked", head.as_object(), false)
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/tags/locked").unwrap();
        std::fs::write(repo.path().join("refs/tags/locked.lock"), "locked").unwrap();
        assert!(delete_reference(path, request).is_err());
        assert_eq!(
            repo.find_reference("refs/tags/locked").unwrap().target(),
            Some(head.id())
        );
    }

    #[test]
    fn stash_deletion_preserves_other_entries_and_rejects_reordered_indices() {
        let (_dir, mut repo) = fixture();
        let path = repo.workdir().unwrap().to_path_buf();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        std::fs::write(path.join("file"), "first").unwrap();
        let first = repo.stash_save(&sig, "first", None).unwrap();
        let stale = prepare_reference_deletion(&path, "stash@{0}").unwrap();
        std::fs::write(path.join("file"), "second").unwrap();
        let second = repo.stash_save(&sig, "second", None).unwrap();
        assert!(delete_reference(&path, stale).is_err());
        let request = prepare_reference_deletion(&path, "stash@{1}").unwrap();
        assert_eq!(request.expected, first.to_string());
        delete_reference(&path, request).unwrap();
        assert_eq!(repo.reflog("refs/stash").unwrap().len(), 1);
        assert_eq!(
            repo.reflog("refs/stash").unwrap().get(0).unwrap().id_new(),
            second
        );
        let request = prepare_reference_deletion(&path, "stash@{0}").unwrap();
        delete_reference(&path, request).unwrap();
        assert!(
            super::super::panel_references(&path, true)
                .unwrap()
                .is_empty()
        );
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "base");
    }
    #[test]
    fn remote_tag_option_handles_matching_missing_and_changed_server_tags() {
        let (dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        let server = Repository::init_bare(dir.path().join("server")).unwrap();
        let mut remote = repo
            .remote("origin", server.path().to_str().unwrap())
            .unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("matching", head.as_object(), false)
            .unwrap();
        remote
            .push(&["refs/tags/matching:refs/tags/matching"], None)
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/tags/matching").unwrap();
        delete_reference_with_remote_tag(path, request, true).unwrap();
        assert!(server.find_reference("refs/tags/matching").is_err());
        assert!(repo.find_reference("refs/tags/matching").is_err());
        repo.tag_lightweight("missing-remote", head.as_object(), false)
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/tags/missing-remote").unwrap();
        delete_reference_with_remote_tag(path, request, true).unwrap();
        assert!(repo.find_reference("refs/tags/missing-remote").is_err());
        repo.tag_lightweight("changed", head.as_object(), false)
            .unwrap();
        remote
            .push(&["refs/tags/changed:refs/tags/changed"], None)
            .unwrap();
        let server_head = server.find_commit(head.id()).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let new = server
            .commit(
                None,
                &sig,
                &sig,
                "different",
                &server_head.tree().unwrap(),
                &[&server_head],
            )
            .unwrap();
        server
            .reference("refs/tags/changed", new, true, "change")
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/tags/changed").unwrap();
        assert!(delete_reference_with_remote_tag(path, request, true).is_err());
        assert_eq!(
            repo.find_reference("refs/tags/changed").unwrap().target(),
            Some(head.id())
        );
        assert_eq!(
            server.find_reference("refs/tags/changed").unwrap().target(),
            Some(new)
        );
    }

    #[test]
    fn remote_deletion_removes_server_branch_and_tracking_ref_but_rejects_changed_server() {
        let (dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        let server = Repository::init_bare(dir.path().join("server")).unwrap();
        let mut remote = repo
            .remote("origin", server.path().to_str().unwrap())
            .unwrap();
        remote
            .push(&["refs/heads/main:refs/heads/topic"], None)
            .unwrap();
        let id = repo.head().unwrap().target().unwrap();
        repo.reference("refs/remotes/origin/topic", id, true, "fixture")
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/remotes/origin/topic").unwrap();
        delete_reference(path, request).unwrap();
        assert!(server.find_reference("refs/heads/topic").is_err());
        assert!(repo.find_reference("refs/remotes/origin/topic").is_err());
        remote
            .push(&["refs/heads/main:refs/heads/topic"], None)
            .unwrap();
        repo.reference("refs/remotes/origin/topic", id, true, "fixture")
            .unwrap();
        let request = prepare_reference_deletion(path, "refs/remotes/origin/topic").unwrap();
        let head = server.find_commit(id).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let new = server
            .commit(
                Some("refs/heads/topic"),
                &sig,
                &sig,
                "server change",
                &head.tree().unwrap(),
                &[&head],
            )
            .unwrap();
        assert!(delete_reference(path, request).is_err());
        assert_eq!(
            server.find_reference("refs/heads/topic").unwrap().target(),
            Some(new)
        );
        assert_eq!(
            repo.find_reference("refs/remotes/origin/topic")
                .unwrap()
                .target(),
            Some(id)
        );
        assert_eq!(repo.head().unwrap().target(), Some(id));
    }
}
