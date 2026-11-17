use anyhow::{Context, Result};
use git2::{ObjectType, Repository};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match {
    Commit { id: String, title: String },
    File { path: String, content: bool },
}
fn safe_worktree_file(root: &Path, path: &str) -> Option<PathBuf> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let file = root.join(path);
    if !std::fs::symlink_metadata(&file).ok()?.file_type().is_file() {
        return None;
    }
    let resolved = file.canonicalize().ok()?;
    resolved.starts_with(root).then_some(resolved)
}

pub fn search(root: &Path, query: &str) -> Result<Vec<Match>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let repo = Repository::open_ext(
        root,
        git2::RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )
    .context("검색할 저장소를 열지 못했습니다")?;
    let workdir = repo.workdir().map(Path::canonicalize).transpose()?;
    let metadata_dirs = workdir.as_deref().map(crate::repository::git_metadata_directories)
        .transpose()?.unwrap_or_default();
    let is_metadata = |path: &str| {
        Path::new(path).components().any(|part| {
            matches!(part, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case(".git"))
        }) || workdir.as_ref().is_some_and(|root| {
            let absolute = root.join(path);
            metadata_dirs.iter().any(|directory| absolute.starts_with(directory))
        })
    };
    let mut walk = repo.revwalk()?;
    for reference in repo.references()? {
        let reference = reference?;
        let name = reference.name().unwrap_or("");
        if (name.starts_with("refs/heads/")
            || name.starts_with("refs/remotes/")
            || name.starts_with("refs/tags/"))
            && let Ok(object) = reference.peel(ObjectType::Commit)
        {
            walk.push(object.id())?;
        }
    }
    if let Ok(head) = repo.head()
        && let Some(id) = head.target()
    {
        walk.push(id)?;
    }
    let mut commits = Vec::new();
    let mut files: BTreeMap<String, bool> = BTreeMap::new();
    let mut content_matches = HashMap::new();
    for oid in walk {
        crate::backend::checkpoint();
        let commit = repo.find_commit(oid?)?;
        let message = String::from_utf8_lossy(commit.message_bytes());
        if message.to_lowercase().contains(&needle) {
            let matched_line = message
                .lines()
                .find(|line| line.to_lowercase().contains(&needle));
            commits.push(Match::Commit {
                id: commit.id().to_string(),
                title: matched_line.unwrap_or_default().to_owned(),
            });
        }
        let mut trees = vec![(String::new(), commit.tree()?)];
        while let Some((prefix, tree)) = trees.pop() {
            for entry in tree.iter() {
                crate::backend::checkpoint();
                let path = format!("{}{}", prefix, entry.name().unwrap_or(""));
                if is_metadata(&path) { continue; }
                if entry.kind() == Some(ObjectType::Tree) {
                    trees.push((format!("{path}/"), repo.find_tree(entry.id())?));
                    continue;
                }
                if entry.kind() != Some(ObjectType::Blob) || files.get(&path) == Some(&true) {
                    continue;
                }
                let path_match = path.to_lowercase().contains(&needle);
                let content_match = if let Some(found) = content_matches.get(&entry.id()) {
                    *found
                } else {
                    let blob = repo.find_blob(entry.id())?;
                    let found = !blob.content().contains(&0)
                        && std::str::from_utf8(blob.content())
                            .ok()
                            .is_some_and(|text| text.to_lowercase().contains(&needle));
                    content_matches.insert(entry.id(), found);
                    found
                };
                if path_match || content_match {
                    files.insert(path, content_match);
                }
            }
        }
    }
    if let Some(workdir) = repo.workdir() {
        let canonical_root = workdir.canonicalize()?;
        let mut inspect = |path: &str| {
            crate::backend::checkpoint();
            if path.is_empty()
                || Path::new(path).is_absolute()
                || path.split('/').any(|part| part == "..")
                || is_metadata(path)
            {
                return;
            }
            let name_match = path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.to_lowercase().contains(&needle));
            let path_match = path.to_lowercase().contains(&needle) || name_match;
            let content_match = safe_worktree_file(&canonical_root, path)
                .and_then(|file| std::fs::read(file).ok())
                .is_some_and(|bytes| {
                    !bytes.contains(&0)
                        && std::str::from_utf8(&bytes)
                            .ok()
                            .is_some_and(|text| text.to_lowercase().contains(&needle))
                });
            if path_match || content_match {
                files
                    .entry(path.to_owned())
                    .and_modify(|found| *found |= content_match)
                    .or_insert(content_match);
            }
        };
        for entry in repo.index()?.iter() {
            inspect(&String::from_utf8_lossy(&entry.path));
        }
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        for entry in repo.statuses(Some(&mut options))?.iter() {
            if entry.status().is_wt_new()
                && let Ok(path) = entry.path()
            {
                inspect(path);
            }
        }
    }
    let mut result = commits;
    result.extend(
        files
            .into_iter()
            .map(|(path, content)| Match::File { path, content }),
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_excludes_actual_metadata_but_keeps_similarly_named_files_and_commits() {
        let root = tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        let repo = Repository::init(root.path()).unwrap();
        std::fs::create_dir(root.path().join(".git2")).unwrap();
        std::fs::create_dir(root.path().join(".git2-notes")).unwrap();
        std::fs::write(root.path().join(".git2/config"), "[audit]\n marker = internal-marker\n").unwrap();
        std::fs::write(root.path().join(".git2-notes/readme.txt"), "ordinary-marker\n").unwrap();
        std::fs::write(root.path().join(".gitignore"), "ordinary-marker\n").unwrap();
        let mut index = repo.index().unwrap();
        for path in [".git2/config", ".git2-notes/readme.txt", ".gitignore"] {
            index.add_path(Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("QA", "qa@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "ordinary-marker commit", &tree, &[]).unwrap();
        Repository::init_bare(root.path().join(".git2")).unwrap();
        let other = Repository::init_bare(root.path().join("metadata-store")).unwrap();
        other.config().unwrap().set_str("audit.marker", "internal-marker").unwrap();
        assert!(search(repo.path(), "internal-marker").unwrap().is_empty());
        let matches = search(repo.path(), ".git2").unwrap();
        assert_eq!(matches, [Match::File { path: ".git2-notes/readme.txt".into(), content: false }]);
        let normal = search(repo.path(), "ordinary-marker").unwrap();
        assert!(normal.iter().any(|entry| matches!(entry, Match::Commit { .. })));
        for path in [".git2-notes/readme.txt", ".gitignore"] {
            assert!(normal.contains(&Match::File { path: path.into(), content: true }));
        }
    }

    #[test]
    fn searches_commit_path_and_old_file_content_across_refs() {
        let root =
            tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        let repo = Repository::init(root.path()).unwrap();
        let signature = git2::Signature::now("Tester", "test@example.com").unwrap();
        let add = |name: &str, content: &str, message: &str| {
            std::fs::write(root.path().join(name), content).unwrap();
            let mut index = repo.index().unwrap();
            let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
            if let Some(parent) = &parent {
                index.read_tree(&parent.tree().unwrap()).unwrap();
            }
            index.add_path(Path::new(name)).unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let parents: Vec<_> = parent.iter().collect();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parents,
            )
            .unwrap()
        };
        let first = add(
            "notes.txt",
            "old unique content",
            "first unique commit\n\n본문 검색어 한글",
        );
        repo.tag_lightweight("old", &repo.find_object(first, None).unwrap(), false)
            .unwrap();
        repo.branch("feature", &repo.find_commit(first).unwrap(), false)
            .unwrap();
        let second = add("another.txt", "new content", "second commit");
        let main_branch = repo.head().unwrap().name().unwrap().to_owned();
        repo.set_head("refs/heads/feature").unwrap();
        let feature = add("feature.txt", "only on feature branch", "feature commit");
        repo.set_head(&main_branch).unwrap();
        std::fs::write(root.path().join("untracked-filename.txt"), "working text").unwrap();
        assert!(
            search(root.path(), "first unique")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::Commit { id, .. } if id == &first.to_string()))
        );
        assert!(
            search(root.path(), "second commit")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::Commit { id, .. } if id == &second.to_string()))
        );
        assert!(
            !search(root.path(), "second commit")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::Commit { id, .. } if id == &first.to_string()))
        );
        assert!(search(root.path(), "본문 검색어").unwrap().iter().any(|item| matches!(item,
            Match::Commit { id, title } if id == &first.to_string() && title.contains("본문 검색어"))));
        assert!(
            search(root.path(), "notes.txt")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::File { path, .. } if path == "notes.txt"))
        );
        assert!(search(root.path(), "old unique content").unwrap().iter().any(|item| matches!(item, Match::File { path, content: true } if path == "notes.txt")));
        assert!(
            search(root.path(), "untracked-filename")
                .unwrap()
                .iter()
                .any(|item| matches!(item,
            Match::File { path, .. } if path == "untracked-filename.txt"))
        );
        assert!(search(root.path(), "only on feature branch").unwrap().iter().any(|item| matches!(item, Match::File { path, content: true } if path == "feature.txt")));
        assert!(
            search(root.path(), "FIRST UNIQUE")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::Commit { id, .. } if id == &first.to_string()))
        );
        assert!(search(root.path(), "   ").unwrap().is_empty());
        std::fs::write(root.path().join("binary.dat"), b"secret\0binary-content").unwrap();
        assert!(!search(root.path(), "binary-content").unwrap().iter().any(
            |item| matches!(item, Match::File { path, content: true } if path == "binary.dat")
        ));
        assert!(
            search(root.path(), "feature commit")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::Commit { id, .. } if id == &feature.to_string()))
        );
    }
    #[test]
    fn search_includes_tag_only_and_detached_commits_without_changing_head() {
        let root =
            tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        let repo = Repository::init(root.path()).unwrap();
        let signature = git2::Signature::now("test", "test@example.invalid").unwrap();
        let tree_id = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let tagged = repo
            .commit(None, &signature, &signature, "tag-only-query", &tree, &[])
            .unwrap();
        repo.tag_lightweight("only-tag", &repo.find_object(tagged, None).unwrap(), false)
            .unwrap();
        let detached = repo
            .commit(None, &signature, &signature, "detached-query", &tree, &[])
            .unwrap();
        repo.set_head_detached(detached).unwrap();
        assert!(
            search(root.path(), "TAG-ONLY-QUERY")
                .unwrap()
                .iter()
                .any(|item| matches!(item, Match::Commit { id, .. } if id == &tagged.to_string()))
        );
        assert!(
            search(root.path(), "detached-query").unwrap().iter().any(
                |item| matches!(item, Match::Commit { id, .. } if id == &detached.to_string())
            )
        );
        assert_eq!(repo.head().unwrap().target(), Some(detached));
        assert!(repo.head_detached().unwrap());
    }
    #[test]
    fn worktree_content_paths_reject_parent_absolute_missing_and_directory_targets() {
        let dir =
            tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        let root = dir.path().join("repo");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("inside.txt"), "inside").unwrap();
        std::fs::write(dir.path().join("outside.txt"), "outside").unwrap();
        let root = root.canonicalize().unwrap();
        assert!(safe_worktree_file(&root, "inside.txt").is_some());
        assert!(safe_worktree_file(&root, "../outside.txt").is_none());
        assert!(
            safe_worktree_file(&root, &dir.path().join("outside.txt").to_string_lossy()).is_none()
        );
        assert!(safe_worktree_file(&root, "missing.txt").is_none());
        assert!(safe_worktree_file(&root, "").is_none());
        std::fs::create_dir(root.join("folder")).unwrap();
        assert!(safe_worktree_file(&root, "folder").is_none());
    }
}
