use crate::{
    diff_model::{Content, FileChange},
    repository::{Commit, CommitReference},
};
use anyhow::{Context, Result};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Clone)]
pub struct FileHistoryEntry {
    pub commit: Commit,
    pub file: FileChange,
}

fn content(tree: Option<&git2::Tree<'_>>, raw: &[u8]) -> Result<Content> {
    use gix::bstr::ByteSlice;
    let Some(tree) = tree else {
        return Ok(Content::Empty);
    };
    let path = gix::path::from_bstr(raw.as_bstr())?;
    match tree.get_path(&path) {
        Ok(entry) => Ok(Content::Object(
            gix::ObjectId::from_hex(entry.id().to_string().as_bytes())?,
            entry.filemode() as u16,
        )),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(Content::Empty),
        Err(error) => Err(error.into()),
    }
}

/// 모든 브랜치의 Git 객체만 읽고 작업 파일과 인덱스는 변경하지 않습니다.
pub fn load(root: &Path, raw: &[u8]) -> Result<Vec<FileHistoryEntry>> {
    crate::path_display::absolute_file_path(root, raw).map_err(anyhow::Error::msg)?;
    let repo = git2::Repository::open(root)?;
    let mut tips = HashSet::new();
    let mut references: HashMap<git2::Oid, Vec<CommitReference>> = HashMap::new();
    for reference in repo.references()? {
        let reference = reference?;
        if !(reference.is_branch() || reference.is_remote() || reference.is_tag()) {
            continue;
        }
        let commit = match reference.peel_to_commit() {
            Ok(commit) => commit,
            Err(_) if reference.is_tag() => continue,
            Err(error) => return Err(error.into()),
        };
        if reference.is_branch() || reference.is_remote() {
            tips.insert(commit.id());
        }
        let full_name = reference
            .name()
            .context("참조 이름을 읽을 수 없습니다.")?
            .to_owned();
        let name = full_name
            .strip_prefix("refs/heads/")
            .or_else(|| full_name.strip_prefix("refs/remotes/"))
            .or_else(|| full_name.strip_prefix("refs/tags/"))
            .unwrap_or(&full_name)
            .to_owned();
        let label = if reference.is_remote() {
            name.split_once('/')
                .map_or(name.as_str(), |(_, branch)| branch)
                .to_owned()
        } else {
            name.clone()
        };
        references
            .entry(commit.id())
            .or_default()
            .push(CommitReference {
                full_name,
                name,
                label,
                is_remote: reference.is_remote(),
            });
    }
    match repo.head() {
        Ok(head) => {
            tips.insert(head.peel_to_commit()?.id());
        }
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
            ) => {}
        Err(error) => return Err(error.into()),
    }
    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    for tip in tips {
        walk.push(tip)?;
    }
    let mut entries = Vec::new();
    for id in walk {
        crate::backend::checkpoint();
        let id = id?;
        let commit = repo.find_commit(id)?;
        let tree = commit.tree()?;
        let parent = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let before = content(parent.as_ref(), raw)?;
        let after = content(Some(&tree), raw)?;
        if before == after {
            continue;
        }
        let status = if matches!(before, Content::Empty) {
            "A"
        } else if matches!(after, Content::Empty) {
            "D"
        } else {
            "M"
        };
        let file = FileChange {
            path: raw.to_vec(),
            label: String::from_utf8_lossy(raw).into_owned(),
            status: status.into(),
            staged: false,
            before,
            after,
        };
        let time = commit.time();
        let date = gix::date::Time::new(time.seconds(), time.offset_minutes() * 60)
            .format(gix::date::time::format::ISO8601)
            .map(|value| value.chars().take(16).collect::<String>().replace('T', " "))
            .unwrap_or_else(|_| time.seconds().to_string());
        let commit_references = references.remove(&id).unwrap_or_default();
        entries.push(FileHistoryEntry {
            file,
            commit: Commit {
                id: id.to_string(),
                parents: commit.parent_ids().map(|id| id.to_string()).collect(),
                message: commit.summary()?.unwrap_or_default().to_owned(),
                author: commit.author().name().unwrap_or_default().to_owned(),
                date,
                reference: commit_references
                    .iter()
                    .map(|reference| reference.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                references: commit_references,
            },
        });
    }
    Ok(entries)
}

struct ExportCleanup {
    path: std::path::PathBuf,
    directory: std::path::PathBuf,
    keep: bool,
}
impl Drop for ExportCleanup {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_dir(&self.directory);
        }
    }
}

pub fn export_revision(
    root: &Path,
    id: &str,
    raw: &[u8],
    session: &crate::history_temp::Session,
) -> Result<std::path::PathBuf> {
    let path = export_revision_to(
        root,
        id,
        raw,
        &std::env::temp_dir().join("feather-git/file-history"),
    )?;
    session.track(path.clone());
    Ok(path)
}

fn export_revision_to(
    root: &Path,
    id: &str,
    raw: &[u8],
    cache: &Path,
) -> Result<std::path::PathBuf> {
    use std::io::Write;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = crate::path_display::absolute_file_path(root, raw).map_err(anyhow::Error::msg)?;
    let repo = git2::Repository::open(root)?;
    let commit = repo.find_commit(git2::Oid::from_str(id)?)?;
    let tree = commit.tree()?;
    let relative = path.strip_prefix(root)?;
    let entry = tree
        .get_path(relative)
        .context("해당 커밋에 파일이 없습니다.")?;
    anyhow::ensure!(
        entry.filemode() & 0o170000 == 0o100000,
        "일반 파일만 열 수 있습니다."
    );
    let blob = repo.find_blob(entry.id())?;
    let name = path
        .file_name()
        .context("파일 이름이 없습니다.")?
        .to_string_lossy();
    std::fs::create_dir_all(cache)?;
    let number = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let commit_id = commit.id().to_string();
    let directory = cache.join(format!(
        "{}-{}-{number}",
        &commit_id[..12],
        std::process::id()
    ));
    std::fs::create_dir(&directory)?;
    let target = directory.join(format!("{}-{name}", &commit_id[..12]));
    let mut output = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
    {
        Ok(output) => output,
        Err(error) => {
            let _ = std::fs::remove_dir(&directory);
            return Err(error.into());
        }
    };
    let mut cleanup = ExportCleanup {
        path: target.clone(),
        directory,
        keep: false,
    };
    output.write_all(blob.content())?;
    output.flush()?;
    output.sync_all()?;
    drop(output);
    let mut permissions = std::fs::metadata(&target)?.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&target, permissions)?;
    cleanup.keep = true;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, git2::Repository) {
        let parent = std::env::current_dir()
            .unwrap()
            .join("target/file-history-tests");
        std::fs::create_dir_all(&parent).unwrap();
        let dir = tempfile::tempdir_in(parent).unwrap();
        let repo = git2::Repository::init_opts(
            dir.path(),
            git2::RepositoryInitOptions::new().initial_head("main"),
        )
        .unwrap();
        (dir, repo)
    }
    fn commit(
        repo: &git2::Repository,
        message: &str,
        changes: &[(&str, Option<&str>)],
    ) -> git2::Oid {
        let root = repo.workdir().unwrap();
        let mut index = repo.index().unwrap();
        for (name, text) in changes {
            if let Some(text) = text {
                if let Some(parent) = root.join(name).parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(root.join(name), text).unwrap();
                index.add_path(Path::new(name)).unwrap();
            } else {
                std::fs::remove_file(root.join(name)).unwrap();
                index.remove_path(Path::new(name)).unwrap();
            }
        }
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let sig = git2::Signature::now("History Test", "history@example.invalid").unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
    }
    fn checkout(repo: &git2::Repository, name: &str) {
        repo.set_head(name).unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
    }

    #[test]
    fn revision_export_opens_exact_blob_in_unique_readonly_file_and_preserves_worktree() {
        let (dir, repo) = fixture();
        repo.config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        let root = dir.path();
        let old = commit(
            &repo,
            "old",
            &[("한 글[1].txt", Some("old\r\nwithout final newline"))],
        );
        let new = commit(&repo, "new", &[("한 글[1].txt", Some("new content\n"))]);
        let cache = root.join("export-cache");
        let index = std::fs::read(root.join(".git/index")).unwrap();
        let first =
            export_revision_to(root, &old.to_string(), "한 글[1].txt".as_bytes(), &cache).unwrap();
        let second =
            export_revision_to(root, &new.to_string(), "한 글[1].txt".as_bytes(), &cache).unwrap();
        let again =
            export_revision_to(root, &old.to_string(), "한 글[1].txt".as_bytes(), &cache).unwrap();
        assert_ne!(first, second);
        assert_ne!(first, again);
        assert_eq!(first.extension().unwrap(), "txt");
        assert_eq!(
            std::fs::read(&first).unwrap(),
            b"old\r\nwithout final newline"
        );
        assert_eq!(std::fs::read(&second).unwrap(), b"new content\n");
        assert!(std::fs::metadata(&first).unwrap().permissions().readonly());
        assert_eq!(
            std::fs::read(root.join("한 글[1].txt")).unwrap(),
            b"new content\n"
        );
        assert_eq!(std::fs::read(root.join(".git/index")).unwrap(), index);
        let deleted = commit(&repo, "delete", &[("한 글[1].txt", None)]);
        assert!(
            export_revision_to(
                root,
                &deleted.to_string(),
                "한 글[1].txt".as_bytes(),
                &cache
            )
            .is_err()
        );
        assert!(export_revision_to(root, &old.to_string(), b"../outside", &cache).is_err());
        assert!(export_revision_to(root, "invalid", b"file.txt", &cache).is_err());
        for path in [first, second, again] {
            let mut permissions = std::fs::metadata(&path).unwrap().permissions();
            #[cfg(windows)]
            #[allow(
                clippy::permissions_set_readonly_false,
                reason = "Windows 테스트 파일의 읽기 전용 속성만 해제한다"
            )]
            permissions.set_readonly(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                permissions.set_mode(0o600);
            }
            std::fs::set_permissions(&path, permissions).unwrap();
        }
    }

    #[test]
    fn all_branches_match_current_path_without_following_renames_or_writing_files() {
        let (dir, repo) = fixture();
        let root = dir.path();
        let base = commit(&repo, "create", &[("old name.txt", Some("one\ntwo\n"))]);
        repo.branch("side", &repo.find_commit(base).unwrap(), false)
            .unwrap();
        let renamed = commit(
            &repo,
            "rename",
            &[("old name.txt", None), ("한 글[1].txt", Some("one\ntwo\n"))],
        );
        let modified = commit(&repo, "modify", &[("한 글[1].txt", Some("main change\n"))]);
        let unrelated = commit(&repo, "unrelated", &[("other.txt", Some("other"))]);
        checkout(&repo, "refs/heads/side");
        let remote_only = commit(
            &repo,
            "remote path before rename",
            &[("old name.txt", Some("side change\n"))],
        );
        let remote_current = commit(
            &repo,
            "remote current path",
            &[("한 글[1].txt", Some("remote current\n"))],
        );
        repo.reference(
            "refs/remotes/origin/archive",
            remote_current,
            true,
            "fixture",
        )
        .unwrap();
        checkout(&repo, "refs/heads/main");
        repo.find_branch("side", git2::BranchType::Local)
            .unwrap()
            .delete()
            .unwrap();
        let deleted = commit(&repo, "delete", &[("한 글[1].txt", None)]);
        let index_before = std::fs::read(root.join(".git/index")).unwrap();
        let rows = load(root, "한 글[1].txt".as_bytes()).unwrap();
        let ids = rows
            .iter()
            .map(|entry| entry.commit.id.clone())
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), rows.len());
        assert_eq!(rows.len(), 4);
        for id in [renamed, modified, remote_current, deleted] {
            assert!(ids.contains(&id.to_string()));
        }
        assert!(!ids.contains(&unrelated.to_string()));
        assert!(!ids.contains(&base.to_string()));
        assert!(!ids.contains(&remote_only.to_string()));
        assert!(
            rows.iter()
                .all(|entry| entry.file.path == "한 글[1].txt".as_bytes())
        );
        let rename = rows
            .iter()
            .find(|entry| entry.commit.id == renamed.to_string())
            .unwrap();
        assert_eq!(rename.file.status, "A");
        assert!(matches!(rename.file.before, Content::Empty));
        assert_eq!(rename.file.path, "한 글[1].txt".as_bytes());
        let old_rows = load(root, b"old name.txt").unwrap();
        let old = old_rows
            .iter()
            .find(|entry| entry.commit.id == base.to_string())
            .unwrap();
        assert_eq!(old.file.path, b"old name.txt");
        assert_eq!(old.file.status, "A");
        assert!(matches!(old.file.before, Content::Empty));
        assert!(
            old_rows
                .iter()
                .any(|entry| entry.commit.id == remote_only.to_string())
        );
        assert!(
            !old_rows
                .iter()
                .any(|entry| entry.commit.id == modified.to_string())
        );
        let deleted = rows
            .iter()
            .find(|entry| entry.commit.id == deleted.to_string())
            .unwrap();
        assert_eq!(deleted.file.status, "D");
        assert!(matches!(deleted.file.after, Content::Empty));
        for entry in &rows {
            let diff = crate::diff_git::diff(root, &entry.file).unwrap();
            assert!(diff.worktree.is_none());
        }
        assert_eq!(
            std::fs::read(root.join(".git/index")).unwrap(),
            index_before
        );
        assert_eq!(
            std::fs::read_to_string(root.join("other.txt")).unwrap(),
            "other"
        );
        assert!(!root.join("한 글[1].txt").exists());
        assert!(load(root, b"never-committed.txt").unwrap().is_empty());
    }

    #[test]
    fn history_has_no_200_commit_limit_and_handles_unborn_and_merge_first_parent() {
        let (dir, repo) = fixture();
        let root = dir.path();
        assert!(load(root, b"file.txt").unwrap().is_empty());
        for number in 0..205 {
            commit(
                &repo,
                &format!("change {number}"),
                &[("file.txt", Some(&format!("value {number}")))],
            );
        }
        let rows = load(root, b"file.txt").unwrap();
        assert_eq!(rows.len(), 205);
        let first_parent = repo.head().unwrap().peel_to_commit().unwrap();
        let base = repo
            .find_commit(git2::Oid::from_str(&rows.last().unwrap().commit.id).unwrap())
            .unwrap();
        let mut index = repo.index().unwrap();
        std::fs::write(root.join("file.txt"), "merged").unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("History Test", "history@example.invalid").unwrap();
        let merged = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "merge",
                &tree,
                &[&first_parent, &base],
            )
            .unwrap();
        let rows = load(root, b"file.txt").unwrap();
        assert_eq!(rows.len(), 206);
        assert_eq!(rows[0].commit.id, merged.to_string());
        let before = rows[0].file.before.clone();
        let expected = content(Some(&first_parent.tree().unwrap()), b"file.txt").unwrap();
        assert_eq!(before, expected);
        assert!(load(root, b"../outside").is_err());
    }
}
