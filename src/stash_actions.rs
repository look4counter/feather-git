use super::{open_exact, index_conflict_paths};
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct StashApplyFailure {
    pub conflict_paths: Vec<PathBuf>,
    message: String,
}
impl std::fmt::Display for StashApplyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for StashApplyFailure {}

pub fn create_stash(path: &Path, name: &str, keep_index: bool) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        bail!("스태시명을 한 줄로 입력하세요.");
    }
    let mut repo = open_exact(path)?;
    if repo.is_bare()
        || repo.state() != git2::RepositoryState::Clean
        || repo.index()?.has_conflicts()
    {
        bail!("작업 폴더에서 진행 중인 병합·리베이스와 충돌을 먼저 해결하세요.");
    }
    repo.head()
        .context("초기 커밋을 만든 뒤 Stash를 실행하세요.")?;
    for lock in [
        repo.path().join("index.lock"),
        repo.commondir().join("refs/stash.lock"),
    ] {
        match std::fs::symlink_metadata(lock) {
            Ok(_) => bail!("인덱스 또는 스태시가 잠겨 있어 저장할 수 없습니다."),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("잠금 상태를 확인하지 못했습니다."),
        }
    }
    let signature = repo
        .signature()
        .context("Stash에 사용할 사용자 이름과 이메일을 설정하세요.")?;
    let flags = if keep_index {
        git2::StashFlags::KEEP_INDEX
    } else {
        git2::StashFlags::empty()
    };
    repo.stash_save(&signature, name, Some(flags)).context(
        "Stash를 저장하지 못했습니다. 저장할 추적 파일 변경사항과 잠금·권한을 확인하세요.",
    )?;
    Ok(())
}

pub fn apply_stash(path: &Path, request: super::ReferenceDeletion) -> Result<()> {
    let index: usize = request
        .reference
        .strip_prefix("stash@{")
        .and_then(|name| name.strip_suffix('}'))
        .context("적용할 스태시를 확인하세요.")?
        .parse()
        .context("스태시 순번을 확인하세요.")?;
    let guard = open_exact(path)?;
    let mut transaction = guard.transaction()?;
    transaction
        .lock_ref("refs/stash")
        .context("스태시가 잠겨 있어 적용할 수 없습니다.")?;
    let mut repo = open_exact(path)?;
    let existing = index_conflict_paths(&repo)?;
    if !existing.is_empty() {
        return Err(StashApplyFailure { conflict_paths: existing.into_iter().collect(), message: "이미 미해결 충돌이 있습니다. 아래 파일의 충돌을 해결한 뒤 스태시를 적용하세요. 스태시는 유지됩니다.".into() }.into());
    }
    if repo.is_bare() || repo.state() != git2::RepositoryState::Clean {
        bail!("스태시 적용 전에 진행 중인 병합·리베이스를 해결하세요.");
    }
    repo.head()
        .context("초기 커밋을 만든 뒤 스태시를 적용하세요.")?;
    match std::fs::symlink_metadata(repo.path().join("index.lock")) {
        Ok(_) => bail!("인덱스가 잠겨 있어 스태시를 적용할 수 없습니다."),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("인덱스 잠금 상태를 확인하지 못했습니다."),
    }
    if super::prepare_reference_deletion(path, &request.reference)? != request {
        bail!("스태시 순서 또는 대상이 변경되었습니다. 목록을 갱신한 뒤 다시 적용하세요.");
    }
    let mut paths = BTreeSet::new();
    let mut options = git2::StashApplyOptions::new();
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout
        .safe()
        .notify_on(git2::CheckoutNotificationType::CONFLICT)
        .notify(|_, path, _, _, _| {
            if let Some(path) = path {
                paths.insert(path.to_path_buf());
            }
            true
        });
    options.checkout_options(checkout);
    let result = repo.stash_apply(index, Some(&mut options));
    drop(options);
    paths.extend(index_conflict_paths(&repo)?);
    if !paths.is_empty() {
        let reason = result
            .err()
            .map(|error| format!("\n{error}"))
            .unwrap_or_default();
        return Err(StashApplyFailure {
            conflict_paths: paths.into_iter().collect(),
            message: format!("스태시 적용 중 충돌이 발생했습니다. 스태시는 유지됩니다. 아래 경로와 현재 파일 상태를 확인하세요.{reason}"),
        }.into());
    }
    result.context("스태시 적용에 실패했습니다. 스태시는 유지됩니다. 로컬 변경·잠금·권한과 현재 파일 상태를 확인하세요.")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    fn fixture() -> (tempfile::TempDir, Repository) {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/stash-action-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repo = Repository::init(dir.path().join("repo")).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Tester").unwrap();
        config
            .set_str("user.email", "test@example.invalid")
            .unwrap();
        std::fs::write(repo.workdir().unwrap().join("file"), "base").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        drop(tree);
        (dir, repo)
    }
    #[test]
    fn multiple_checkout_conflicts_keep_sorted_paths_through_toolbar_results() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        std::fs::create_dir(path.join("folder")).unwrap();
        std::fs::write(path.join("folder/second.txt"), "base").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("folder/second.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = Signature::now("Tester", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "second file", &tree, &[&parent])
            .unwrap();
        for file in ["file", "folder/second.txt"] {
            std::fs::write(path.join(file), "saved").unwrap();
        }
        create_stash(path, "both", false).unwrap();
        let request = super::super::prepare_reference_deletion(path, "stash@{0}").unwrap();
        for file in ["file", "folder/second.txt"] {
            std::fs::write(path.join(file), "local").unwrap();
        }
        let error = apply_stash(path, request.clone()).unwrap_err();
        let expected = vec![PathBuf::from("file"), PathBuf::from("folder/second.txt")];
        assert_eq!(
            error
                .downcast_ref::<StashApplyFailure>()
                .unwrap()
                .conflict_paths,
            expected
        );
        let result = crate::workspace_backend::ToolbarRequest {
            revision: 1,
            project: 0,
            path: path.to_owned(),
            name: "fixture".into(),
            action: crate::workspace_backend::ToolbarAction::ApplyStash { request },
        }
        .execute();
        assert!(result.outcome.is_err());
        assert_eq!(result.conflict_paths, expected);
        for file in ["file", "folder/second.txt"] {
            assert_eq!(std::fs::read_to_string(path.join(file)).unwrap(), "local");
        }
        assert_eq!(super::super::panel_references(path, true).unwrap().len(), 1);
    }

    #[test]
    fn existing_index_conflicts_are_reported_once_per_path() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("file"), "saved").unwrap();
        create_stash(path, "saved", false).unwrap();
        let request = super::super::prepare_reference_deletion(path, "stash@{0}").unwrap();
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        let mut entry = index.get_path(Path::new("file"), 0).unwrap();
        index.remove_path(Path::new("file")).unwrap();
        for stage in 1..=3 {
            entry.flags = (entry.flags & !0x3000) | (stage << 12);
            index.add(&entry).unwrap();
        }
        index.write().unwrap();
        let error = apply_stash(path, request).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<StashApplyFailure>()
                .unwrap()
                .conflict_paths,
            vec![PathBuf::from("file")]
        );
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "base");
    }

    #[test]
    fn apply_selected_stash_preserves_stashes_head_and_untracked_files() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        let head = repo.head().unwrap().target();
        std::fs::write(path.join("file"), "first").unwrap();
        create_stash(path, "first", false).unwrap();
        std::fs::write(path.join("file"), "second").unwrap();
        create_stash(path, "second", false).unwrap();
        std::fs::write(path.join("untracked"), "keep").unwrap();
        let request = super::super::prepare_reference_deletion(path, "stash@{1}").unwrap();
        let rows = super::super::panel_references(path, true).unwrap();
        assert_eq!(
            rows[1],
            super::super::stash_display_label(
                1,
                request.description.as_deref().unwrap_or_default()
            )
        );
        apply_stash(path, request).unwrap();
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "first");
        assert_eq!(
            std::fs::read_to_string(path.join("untracked")).unwrap(),
            "keep"
        );
        assert_eq!(super::super::panel_references(path, true).unwrap(), rows);
        assert_eq!(repo.head().unwrap().target(), head);
    }

    #[test]
    fn apply_stash_rejects_reordered_indices_local_conflicts_and_invalid_targets() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("file"), "first").unwrap();
        create_stash(path, "first", false).unwrap();
        let stale = super::super::prepare_reference_deletion(path, "stash@{0}").unwrap();
        std::fs::write(path.join("file"), "second").unwrap();
        create_stash(path, "second", false).unwrap();
        assert!(apply_stash(path, stale).is_err());
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "base");
        let request = super::super::prepare_reference_deletion(path, "stash@{0}").unwrap();
        std::fs::write(path.join("file"), "local").unwrap();
        assert!(apply_stash(path, request).is_err());
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "local");
        let invalid = super::super::prepare_reference_deletion(path, "refs/heads/main").unwrap();
        assert!(apply_stash(path, invalid).is_err());
        assert_eq!(super::super::panel_references(path, true).unwrap().len(), 2);
    }

    #[test]
    fn apply_stash_rejects_locks_without_applying_or_removing_the_stash() {
        for lock in ["index.lock", "refs/stash.lock"] {
            let (_dir, repo) = fixture();
            let path = repo.workdir().unwrap();
            std::fs::write(path.join("file"), "saved").unwrap();
            create_stash(path, "saved", false).unwrap();
            let request = super::super::prepare_reference_deletion(path, "stash@{0}").unwrap();
            std::fs::write(repo.path().join(lock), "locked").unwrap();
            assert!(apply_stash(path, request).is_err());
            assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "base");
            assert_eq!(super::super::panel_references(path, true).unwrap().len(), 1);
        }
    }

    #[test]
    fn named_stash_saves_staged_and_unstaged_changes_and_honors_keep_index() {
        for keep in [false, true] {
            let (_dir, repo) = fixture();
            let path = repo.workdir().unwrap();
            let head = repo.head().unwrap().target();
            std::fs::write(path.join("file"), "staged").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file")).unwrap();
            index.write().unwrap();
            std::fs::write(path.join("file"), "unstaged").unwrap();
            std::fs::write(path.join("untracked"), "keep").unwrap();
            create_stash(path, "named stash", keep).unwrap();
            let id = repo.reflog("refs/stash").unwrap().get(0).unwrap().id_new();
            let stash = repo.find_commit(id).unwrap();
            assert!(stash.message().unwrap().contains("named stash"));
            let tree = stash.tree().unwrap();
            let entry = tree.get_name("file").unwrap();
            assert_eq!(repo.find_blob(entry.id()).unwrap().content(), b"unstaged");
            let staged = stash.parent(1).unwrap().tree().unwrap();
            assert_eq!(
                repo.find_blob(staged.get_name("file").unwrap().id())
                    .unwrap()
                    .content(),
                b"staged"
            );
            assert_eq!(
                std::fs::read_to_string(path.join("file")).unwrap(),
                if keep { "staged" } else { "base" }
            );
            let mut actual_index = repo.index().unwrap();
            actual_index.read(true).unwrap();
            let entry = actual_index.get_path(Path::new("file"), 0).unwrap();
            assert_eq!(
                repo.find_blob(entry.id).unwrap().content(),
                if keep {
                    b"staged".as_slice()
                } else {
                    b"base".as_slice()
                }
            );
            assert_eq!(
                std::fs::read_to_string(path.join("untracked")).unwrap(),
                "keep"
            );
            assert_eq!(repo.head().unwrap().target(), head);
        }
    }
    #[test]
    fn stash_rejects_unborn_and_merge_state_without_changing_worktree() {
        let (dir, repo) = fixture();
        let unborn = Repository::init(dir.path().join("unborn")).unwrap();
        assert!(create_stash(unborn.workdir().unwrap(), "unborn", false).is_err());
        let path = repo.workdir().unwrap();
        std::fs::write(path.join("file"), "local").unwrap();
        std::fs::write(
            repo.path().join("MERGE_HEAD"),
            repo.head().unwrap().target().unwrap().to_string(),
        )
        .unwrap();
        assert!(create_stash(path, "merge", false).is_err());
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "local");
        assert!(repo.find_reference("refs/stash").is_err());
    }

    #[test]
    fn stash_rejects_empty_names_no_changes_and_locks_without_changing_files() {
        let (_dir, repo) = fixture();
        let path = repo.workdir().unwrap();
        assert!(create_stash(path, "", false).is_err());
        assert!(create_stash(path, "clean", false).is_err());
        std::fs::write(path.join("file"), "local").unwrap();
        std::fs::write(repo.path().join("refs/stash.lock"), "locked").unwrap();
        assert!(create_stash(path, "locked", false).is_err());
        assert_eq!(std::fs::read_to_string(path.join("file")).unwrap(), "local");
        assert!(repo.find_reference("refs/stash").is_err());
    }
}
