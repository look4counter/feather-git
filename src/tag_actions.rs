use super::{committing::push_to, open_exact};
use anyhow::{Context, Result, bail};
use std::path::Path;

pub(super) fn tag_push_remote(repo: &git2::Repository) -> Result<String> {
    let remote = if let Ok(head) = repo.head()
        && head.is_branch()
    {
        let branch = head
            .name()?
            .strip_prefix("refs/heads/")
            .context("현재 브랜치를 확인할 수 없습니다.")?;
        match repo
            .config()?
            .get_string(&format!("branch.{branch}.remote"))
        {
            Ok(remote) => remote,
            Err(error) if error.code() == git2::ErrorCode::NotFound => "origin".into(),
            Err(error) => return Err(error.into()),
        }
    } else {
        "origin".into()
    };
    Ok(remote)
}

pub fn create_tag(path: &Path, id: &str, name: &str, push: bool) -> Result<Option<String>> {
    let name = name.trim();
    let reference = format!("refs/tags/{name}");
    if name.is_empty() || name.starts_with('-') || !git2::Reference::is_valid_name(&reference) {
        bail!("올바른 태그명을 입력하세요.");
    }
    if id.len() != 40 {
        bail!("선택한 커밋 ID를 확인하세요.");
    }
    let repo = open_exact(path)?;
    let commit = repo
        .find_commit(git2::Oid::from_str(id)?)
        .context("선택한 커밋을 찾을 수 없습니다.")?;
    repo.tag_lightweight(name, commit.as_object(), false)
        .context("태그를 만들지 못했습니다. 같은 이름의 태그·잠금·권한을 확인하세요.")?;
    if !push {
        return Ok(None);
    }
    let result = (|| -> Result<()> {
        let remote = tag_push_remote(&repo)?;
        push_to(&repo, &reference, &remote, &reference)
    })();
    Ok(result.err().map(|error| format!("로컬 태그 '{name}'는 생성됐지만 원격 Push에 실패했습니다. 로컬 태그는 유지됩니다.\n{error:#}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    fn fixture() -> (tempfile::TempDir, Repository, git2::Oid, git2::Oid) {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/tag-action-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repo = Repository::init(dir.path().join("repo")).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let tree = repo
            .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        let first = repo
            .commit(Some("HEAD"), &sig, &sig, "first", &tree, &[])
            .unwrap();
        let parent = repo.find_commit(first).unwrap();
        let second = repo
            .commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
            .unwrap();
        drop(parent);
        drop(tree);
        std::fs::write(repo.workdir().unwrap().join("local"), "keep").unwrap();
        (dir, repo, first, second)
    }
    #[test]
    fn tag_targets_selected_commit_without_changing_head_and_rejects_invalid_names_and_duplicates()
    {
        let (_dir, repo, first, second) = fixture();
        let path = repo.workdir().unwrap();
        assert!(
            create_tag(path, &first.to_string(), "v1", false)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            repo.find_reference("refs/tags/v1").unwrap().target(),
            Some(first)
        );
        assert!(create_tag(path, &second.to_string(), "v1", false).is_err());
        assert_eq!(
            repo.find_reference("refs/tags/v1").unwrap().target(),
            Some(first)
        );
        for name in ["", "a b", "-invalid", "x..y", "name/"] {
            assert!(create_tag(path, &first.to_string(), name, false).is_err());
        }
        assert!(create_tag(path, "HEAD", "invalid-target", false).is_err());
        assert!(
            create_tag(
                path,
                "0000000000000000000000000000000000000000",
                "missing",
                false
            )
            .is_err()
        );
        assert!(repo.find_reference("refs/tags/missing").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(second));
        assert_eq!(std::fs::read_to_string(path.join("local")).unwrap(), "keep");
    }
    #[test]
    fn tag_push_uses_configured_remote_and_sends_only_the_new_tag() {
        let (dir, repo, first, second) = fixture();
        let server = Repository::init_bare(dir.path().join("server")).unwrap();
        repo.remote("upstream", server.path().to_str().unwrap())
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("branch.main.remote", "upstream")
            .unwrap();
        repo.tag_lightweight(
            "other",
            repo.find_commit(second).unwrap().as_object(),
            false,
        )
        .unwrap();
        assert!(
            create_tag(repo.workdir().unwrap(), &first.to_string(), "v1", true)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            server.find_reference("refs/tags/v1").unwrap().target(),
            Some(first)
        );
        assert!(server.find_reference("refs/tags/other").is_err());
        assert!(server.find_reference("refs/heads/main").is_err());
    }
    #[test]
    fn failed_push_keeps_local_tag_and_never_overwrites_remote_tag() {
        let (dir, repo, first, second) = fixture();
        let path = repo.workdir().unwrap();
        assert!(
            create_tag(path, &first.to_string(), "no-remote", true)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            repo.find_reference("refs/tags/no-remote").unwrap().target(),
            Some(first)
        );
        let server = Repository::init_bare(dir.path().join("server")).unwrap();
        let mut remote = repo
            .remote("origin", server.path().to_str().unwrap())
            .unwrap();
        repo.set_head_detached(second).unwrap();
        assert!(
            create_tag(path, &first.to_string(), "origin-default", true)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            server
                .find_reference("refs/tags/origin-default")
                .unwrap()
                .target(),
            Some(first)
        );
        assert_eq!(repo.head().unwrap().target(), Some(second));
        repo.tag_lightweight(
            "conflict",
            repo.find_commit(first).unwrap().as_object(),
            false,
        )
        .unwrap();
        remote
            .push(&["refs/tags/conflict:refs/tags/conflict"], None)
            .unwrap();
        repo.find_reference("refs/tags/conflict")
            .unwrap()
            .delete()
            .unwrap();
        assert!(
            create_tag(path, &second.to_string(), "conflict", true)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            repo.find_reference("refs/tags/conflict").unwrap().target(),
            Some(second)
        );
        assert_eq!(
            server
                .find_reference("refs/tags/conflict")
                .unwrap()
                .target(),
            Some(first)
        );
    }
}
