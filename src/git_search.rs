use anyhow::{Context, Result};
use git2::{ObjectType, Repository, TreeWalkMode, TreeWalkResult};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

#[derive(Clone, Debug)]
pub enum Match {
    Commit { id: String, title: String },
    File { path: String, content: bool },
}

#[derive(Clone, Debug)]
pub struct FileCommit {
    pub id: String,
    pub description: String,
    pub author: String,
    pub author_email: String,
    pub time: i64,
    pub time_offset: i32,
}

pub fn search(root: &Path, query: &str) -> Result<Vec<Match>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let repo = Repository::open(root).context("검색할 저장소를 열지 못했습니다")?;
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
        let tree = commit.tree()?;
        tree.walk(TreeWalkMode::PreOrder, |prefix, entry| {
            if entry.kind() != Some(ObjectType::Blob) {
                return TreeWalkResult::Ok;
            }
            let path = format!("{}{}", prefix, entry.name().unwrap_or(""));
            if files.get(&path) == Some(&true) {
                return TreeWalkResult::Ok;
            }
            let path_match = path.to_lowercase().contains(&needle);
            let content_match = *content_matches.entry(entry.id()).or_insert_with(|| {
                repo.find_blob(entry.id()).ok().is_some_and(|blob| {
                    !blob.content().contains(&0)
                        && std::str::from_utf8(blob.content())
                            .ok()
                            .is_some_and(|text| text.to_lowercase().contains(&needle))
                })
            });
            if path_match || content_match {
                files.insert(path, content_match);
            }
            TreeWalkResult::Ok
        })?;
    }
    if let Some(workdir) = repo.workdir() {
        let mut inspect = |path: &str| {
            if path.is_empty()
                || Path::new(path).is_absolute()
                || path.split('/').any(|part| part == "..")
            {
                return;
            }
            let name_match = path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.to_lowercase().contains(&needle));
            let path_match = path.to_lowercase().contains(&needle) || name_match;
            let full_path = workdir.join(path);
            let content_match = std::fs::symlink_metadata(&full_path)
                .ok()
                .is_some_and(|metadata| metadata.file_type().is_file())
                && std::fs::read(&full_path).ok().is_some_and(|bytes| {
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

pub fn file_history(root: &Path, path: &str) -> Result<Vec<FileCommit>> {
    let repo = Repository::open(root)?;
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
    let mut result = Vec::new();
    for oid in walk {
        let commit = repo.find_commit(oid?)?;
        let tree = commit.tree()?;
        let current = tree.get_path(Path::new(path)).ok().map(|e| e.id());
        let previous = commit
            .parent(0)
            .ok()
            .and_then(|p| p.tree().ok())
            .and_then(|t| t.get_path(Path::new(path)).ok())
            .map(|e| e.id());
        if current != previous {
            let author = commit.author();
            result.push(FileCommit {
                id: commit.id().to_string(),
                description: commit.summary().ok().flatten().unwrap_or("").to_owned(),
                author: author.name().unwrap_or("").to_owned(),
                author_email: author.email().unwrap_or("").to_owned(),
                time: author.when().seconds(),
                time_offset: author.when().offset_minutes(),
            });
        }
    }
    Ok(result)
}

pub fn file_diff(root: &Path, path: &str, commit_id: &str) -> Result<String> {
    let repo = Repository::open(root)?;
    let commit = repo.find_commit(git2::Oid::from_str(commit_id)?)?;
    let tree = commit.tree()?;
    let parent = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let read = |tree: &git2::Tree<'_>| -> Vec<u8> {
        tree.get_path(Path::new(path))
            .ok()
            .and_then(|entry| repo.find_blob(entry.id()).ok())
            .map(|blob| blob.content().to_vec())
            .unwrap_or_default()
    };
    let before = parent.as_ref().map(read).unwrap_or_default();
    let after = read(&tree);
    if before.contains(&0) || after.contains(&0) {
        return Ok("바이너리 파일입니다.".into());
    }
    let before = String::from_utf8_lossy(&before);
    let after = String::from_utf8_lossy(&after);
    Ok(similar::TextDiff::from_lines(&before, &after)
        .unified_diff()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_commit_path_and_old_file_content_across_refs() {
        let root = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
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
        assert_eq!(
            file_history(root.path(), "feature.txt").unwrap()[0].id,
            feature.to_string()
        );
        let history = file_history(root.path(), "another.txt").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, second.to_string());
        assert_eq!(history[0].description, "second commit");
        assert_eq!(history[0].author, "Tester");
        assert_eq!(history[0].author_email, "test@example.com");
        assert!(history[0].time > 0);
        assert!(
            file_diff(root.path(), "another.txt", &second.to_string())
                .unwrap()
                .contains("+new content")
        );
    }
}
