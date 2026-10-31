use anyhow::{Context, Result, bail};
use git2::{Oid, Repository, build::CheckoutBuilder};
use std::path::Path;

pub(crate) fn open_repository(path: &Path) -> Result<Repository> {
    let repo = Repository::discover(crate::native_git::ordinary_path(path))?;
    if let Err(error) = repo.index()
        && error
            .message()
            .contains("unsupported mandatory extension: 'link'")
    {
        crate::native_local::expand_index(path)?;
        return Ok(Repository::discover(crate::native_git::ordinary_path(
            path,
        ))?);
    }
    Ok(repo)
}

pub fn checkout(path: &Path, a: &[&str]) -> Result<()> {
    let repo = open_repository(path)?;
    let target = *a.last().context("체크아웃 대상이 없습니다.")?;
    if let Some(i) = a.iter().position(|s| *s == "-c") {
        if repo.find_branch(a[i + 1], git2::BranchType::Local).is_ok() {
            bail!("같은 이름의 로컬 브랜치가 있습니다.");
        }
    } else if !a.contains(&"--detach") {
        let expected = format!("branch refs/heads/{target}");
        if String::from_utf8(crate::native_refs::worktrees(path)?)?
            .lines()
            .any(|s| s == expected)
        {
            bail!("다른 작업 트리에서 사용 중인 브랜치입니다.");
        }
    }
    let object = repo.revparse_single(target)?;
    let mut options = CheckoutBuilder::new();
    options.safe();
    repo.checkout_tree(&object, Some(&mut options))?;
    if a.contains(&"--detach") {
        repo.set_head_detached(object.peel_to_commit()?.id())?;
    } else if let Some(i) = a.iter().position(|s| *s == "-c") {
        let name = a[i + 1];
        let branch = repo.branch(name, &object.peel_to_commit()?, false)?;
        repo.set_head(branch.get().name().context("브랜치명이 없습니다.")?)?;
        crate::native_refs::branch(
            path,
            &["branch", &format!("--set-upstream-to={target}"), "--", name],
        )?;
    } else {
        repo.set_head(&format!("refs/heads/{target}"))?;
    }
    Ok(())
}
pub fn reset(path: &Path, a: &[&str]) -> Result<()> {
    let repo = open_repository(path)?;
    let target = repo.revparse_single(a.last().context("Reset 대상이 없습니다.")?)?;
    let mode = if a.contains(&"--hard") {
        git2::ResetType::Hard
    } else if a.contains(&"--soft") {
        git2::ResetType::Soft
    } else {
        git2::ResetType::Mixed
    };
    repo.reset(&target, mode, None)?;
    Ok(())
}
pub fn restore(path: &Path, a: &[&str]) -> Result<()> {
    let repo = open_repository(path)?;
    let target = repo.head()?.peel_to_tree()?;
    let pos = a
        .iter()
        .position(|s| *s == "--")
        .context("복원 경로가 없습니다.")?;
    let mut options = CheckoutBuilder::new();
    options.force().disable_pathspec_match(true);
    for p in &a[pos + 1..] {
        options.path(p);
    }
    repo.checkout_tree(target.as_object(), Some(&mut options))?;
    Ok(())
}
pub fn stash(path: &Path, a: &[&str]) -> Result<Vec<u8>> {
    let mut repo = open_repository(path)?;
    match a.get(1).copied().unwrap_or_default() {
        "list" => {
            let mut ids = Vec::new();
            repo.stash_foreach(|_, _, id| {
                ids.push(id.to_string());
                true
            })?;
            Ok(ids.join("\n").into_bytes())
        }
        "push" => {
            let signature = repo.signature()?;
            let message = a
                .iter()
                .position(|s| *s == "--message")
                .and_then(|i| a.get(i + 1))
                .copied();
            repo.stash_save2(
                &signature,
                message,
                Some(git2::StashFlags::INCLUDE_UNTRACKED),
            )?;
            Ok(Vec::new())
        }
        "drop" => {
            let n = a
                .last()
                .unwrap()
                .strip_prefix("stash@{")
                .and_then(|s| s.strip_suffix('}'))
                .context("잘못된 Stash입니다.")?
                .parse()?;
            repo.stash_drop(n)?;
            Ok(Vec::new())
        }
        "apply" => {
            apply_stash(&repo, Oid::from_str(a.last().unwrap())?)?;
            Ok(Vec::new())
        }
        _ => bail!("지원하지 않는 Stash 요청입니다."),
    }
}

fn changed_paths(
    repo: &Repository,
    old: &git2::Tree<'_>,
    new: &git2::Tree<'_>,
) -> Result<Vec<std::path::PathBuf>> {
    let diff = repo.diff_tree_to_tree(Some(old), Some(new), None)?;
    let mut paths = std::collections::BTreeSet::new();
    for delta in diff.deltas() {
        for file in [delta.old_file(), delta.new_file()] {
            if let Some(path) = file.path() {
                paths.insert(path.to_owned());
            }
        }
    }
    Ok(paths.into_iter().collect())
}
fn copy_index(repo: &Repository, source: &git2::Index) -> Result<()> {
    let entries = source.iter().collect::<Vec<_>>();
    let mut index = repo.index()?;
    index.clear()?;
    for entry in entries {
        index.add(&entry)?;
    }
    index.write()?;
    Ok(())
}
fn preflight_paths(repo: &Repository, paths: &[std::path::PathBuf]) -> Result<()> {
    for path in paths {
        match repo.status_file(path) {
            Ok(status)
                if status.intersects(
                    git2::Status::WT_MODIFIED
                        | git2::Status::WT_DELETED
                        | git2::Status::WT_NEW
                        | git2::Status::CONFLICTED,
                ) =>
            {
                bail!("변경 사항이 덮어써질 수 있습니다: {}", path.display())
            }
            Err(e) if e.code() != git2::ErrorCode::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    Ok(())
}
fn checkout_index_paths(
    repo: &Repository,
    index: &mut git2::Index,
    paths: &[std::path::PathBuf],
    force: bool,
) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut options = CheckoutBuilder::new();
    if force {
        options.force();
    } else {
        options.safe();
    }
    options
        .allow_conflicts(true)
        .conflict_style_merge(true)
        .disable_pathspec_match(true)
        .update_index(false);
    for path in paths {
        options.path(path);
    }
    repo.checkout_index(Some(index), Some(&mut options))?;
    if force {
        let root = repo.workdir().context("No worktree")?;
        for path in paths {
            let raw = path.to_string_lossy().replace('\\', "/");
            if index.get_path(Path::new(&raw), 0).is_none() {
                match std::fs::remove_file(root.join(path)) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
    Ok(())
}
fn apply_stash(repo: &Repository, id: Oid) -> Result<()> {
    let stash = repo.find_commit(id)?;
    if stash.parent_count() < 2 {
        bail!("올바른 Stash가 아닙니다.");
    }
    let base = stash.parent(0)?.tree()?;
    let staged = stash.parent(1)?.tree()?;
    let work = stash.tree()?;
    let mut original = repo.index()?;
    let original_tree = repo.find_tree(original.write_tree()?)?;
    let paths = changed_paths(repo, &base, &work)?;
    preflight_paths(repo, &paths)?;
    let mut merged = repo.merge_trees(&base, &original_tree, &work, None)?;
    let restored_index = repo.merge_trees(&base, &original_tree, &staged, None)?;
    if stash.parent_count() > 2 {
        let untracked = stash.parent(2)?.tree()?;
        let mut options = CheckoutBuilder::new();
        options
            .safe()
            .update_index(false)
            .disable_pathspec_match(true);
        let mut any = false;
        untracked.walk(git2::TreeWalkMode::PreOrder, |prefix, entry| {
            if entry.kind() != Some(git2::ObjectType::Tree)
                && let Ok(name) = entry.name()
            {
                options.path(format!("{prefix}{name}"));
                any = true;
            }
            git2::TreeWalkResult::Ok
        })?;
        if any {
            repo.checkout_tree(untracked.as_object(), Some(&mut options))?;
        }
    }
    checkout_index_paths(repo, &mut merged, &paths, false)?;
    if merged.has_conflicts() {
        copy_index(repo, &merged)?;
        bail!("Stash 충돌 파일을 해결한 뒤 Stage하세요.");
    }
    if restored_index.has_conflicts() {
        copy_index(repo, &restored_index)?;
        bail!("Stash의 Stage 복원에 충돌이 있습니다.");
    }
    copy_index(repo, &restored_index)?;
    Ok(())
}

fn commit_index(
    repo: &Repository,
    message: &str,
    parents: &[git2::Commit<'_>],
    author: Option<&git2::Signature<'_>>,
) -> Result<Oid> {
    let mut index = repo.index()?;
    if index.has_conflicts() {
        bail!("미해결 충돌 파일이 남아 있습니다.");
    }
    let tree = index.write_tree()?;
    let path = repo.workdir().unwrap_or(repo.path());
    let gix_repo = crate::native_local::repo(path)?;
    let parents = parents
        .iter()
        .map(|c| gix::ObjectId::from_bytes_or_panic(c.id().as_bytes()))
        .collect::<Vec<_>>();
    let author = author.map(|sig| gix::actor::Signature {
        name: sig.name_bytes().into(),
        email: sig.email_bytes().into(),
        time: gix::date::Time::new(sig.when().seconds(), sig.when().offset_minutes() * 60),
    });
    let id = crate::native_local::create_commit(
        &gix_repo,
        gix::ObjectId::from_bytes_or_panic(tree.as_bytes()),
        &parents,
        message,
        author,
    )?;
    Ok(Oid::from_bytes(id.as_bytes())?)
}
pub fn merge(path: &Path, a: &[&str]) -> Result<()> {
    let repo = open_repository(path)?;
    if a.contains(&"--abort") {
        let original = std::fs::read_to_string(repo.path().join("ORIG_HEAD"))?;
        let object = repo.find_object(Oid::from_str(original.trim())?, None)?;
        let mut index = repo.index()?;
        let target = object.peel_to_tree()?;
        let before = repo.head()?.peel_to_tree()?;
        let mut paths = changed_paths(&repo, &before, &target)?;
        for entry in index.iter() {
            if (entry.flags >> 12) & 3 != 0 {
                paths.push(std::path::PathBuf::from(
                    String::from_utf8_lossy(&entry.path).into_owned(),
                ));
            }
        }
        if let Ok(heads) = std::fs::read_to_string(repo.path().join("MERGE_HEAD")) {
            for id in heads.lines() {
                let tree = repo.find_commit(Oid::from_str(id)?)?.tree()?;
                paths.extend(changed_paths(&repo, &before, &tree)?);
            }
        }
        index.read_tree(&target)?;
        checkout_index_paths(&repo, &mut index, &paths, true)?;
        copy_index(&repo, &index)?;
        repo.cleanup_state()?;
        return Ok(());
    }
    if a.contains(&"--continue") {
        return finish_merge(&repo);
    }
    let target = repo.revparse_single(a.last().unwrap())?.peel_to_commit()?;
    let annotated = repo.find_annotated_commit(target.id())?;
    let (analysis, _) = repo.merge_analysis(&[&annotated])?;
    if analysis.is_up_to_date() {
        return Ok(());
    }
    if analysis.is_fast_forward() {
        let head = repo.head()?;
        let mut options = CheckoutBuilder::new();
        options.safe();
        repo.checkout_tree(target.as_object(), Some(&mut options))?;
        repo.find_reference(head.name().unwrap())?
            .set_target(target.id(), "merge: fast-forward")?;
        return Ok(());
    }
    let mut options = CheckoutBuilder::new();
    options
        .safe()
        .allow_conflicts(true)
        .conflict_style_merge(true);
    repo.merge(&[&annotated], None, Some(&mut options))?;
    if repo.index()?.has_conflicts() {
        bail!("병합 충돌 파일을 수정한 뒤 Stage하세요.");
    }
    finish_merge(&repo)
}
fn finish_merge(repo: &Repository) -> Result<()> {
    let mut parents = vec![repo.head()?.peel_to_commit()?];
    for id in std::fs::read_to_string(repo.path().join("MERGE_HEAD"))?.lines() {
        parents.push(repo.find_commit(Oid::from_str(id)?)?);
    }
    let message = repo.message().unwrap_or_else(|_| "Merge commit".into());
    commit_index(repo, &message, &parents, None)?;
    repo.cleanup_state()?;
    Ok(())
}

fn rebase_dir(repo: &Repository) -> Result<std::path::PathBuf> {
    for name in ["rebase-merge", "rebase-apply"] {
        let dir = repo.path().join(name);
        if dir.is_dir() {
            return Ok(dir);
        }
    }
    bail!("진행 중인 Rebase가 없습니다.")
}
fn clear_rebase(repo: &Repository) -> Result<()> {
    for name in ["rebase-merge", "rebase-apply"] {
        let dir = repo.path().join(name);
        if dir.is_dir() {
            std::fs::remove_dir_all(dir)?;
        }
    }
    for name in ["CHERRY_PICK_HEAD", "MERGE_MSG", "AUTO_MERGE"] {
        let _ = std::fs::remove_file(repo.path().join(name));
    }
    Ok(())
}
fn replay_commits(repo: &Repository, original: Oid, onto: Oid) -> Result<Vec<Oid>> {
    let base = repo.merge_base(original, onto)?;
    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;
    walk.push(original)?;
    walk.hide(base)?;
    Ok(walk.collect::<std::result::Result<Vec<_>, _>>()?)
}
pub fn rebase(path: &Path, a: &[&str]) -> Result<()> {
    let repo = open_repository(path)?;
    if a.iter()
        .any(|s| ["--abort", "--continue", "--skip"].contains(s))
    {
        let dir = rebase_dir(&repo)?;
        let read = |name: &str| -> Result<String> {
            Ok(std::fs::read_to_string(dir.join(name))?.trim().to_owned())
        };
        let original = Oid::from_str(&read("orig-head")?)?;
        let onto = Oid::from_str(&read("onto")?)?;
        let branch = read("head-name")?;
        if a.contains(&"--abort") {
            let object = repo.find_object(original, None)?;
            repo.reset(&object, git2::ResetType::Hard, None)?;
            if branch.starts_with("refs/heads/") {
                repo.set_head(&branch)?;
            }
            clear_rebase(&repo)?;
            return Ok(());
        }
        let backend_apply = dir.ends_with("rebase-apply");
        let current = read(if backend_apply {
            "original-commit"
        } else {
            "stopped-sha"
        })?;
        let current = Oid::from_str(&current)?;
        if a.contains(&"--continue") {
            let commit = repo.find_commit(current)?;
            let parent = repo.head()?.peel_to_commit()?;
            commit_index(
                &repo,
                commit.message().unwrap_or("Rebased commit"),
                &[parent],
                Some(&commit.author()),
            )?;
        } else {
            let head = repo.head()?.peel_to_commit()?;
            let mut checkout = CheckoutBuilder::new();
            checkout.force();
            repo.checkout_tree(head.as_object(), Some(&mut checkout))?;
            let mut index = repo.index()?;
            index.read_tree(&head.tree()?)?;
            index.write()?;
        }
        let commits = replay_commits(&repo, original, onto)?;
        let start = commits
            .iter()
            .position(|id| *id == current)
            .context("Rebase 순서를 확인하지 못했습니다.")?
            + 1;
        return replay(&repo, &dir, &commits, start, &branch, original);
    }
    let mut status = git2::StatusOptions::new();
    status.include_untracked(false);
    if !repo.statuses(Some(&mut status))?.is_empty() {
        bail!("로컬 변경을 먼저 커밋하거나 스태시하세요.");
    }
    let original = repo.head()?.peel_to_commit()?.id();
    let branch = if repo.head_detached()? {
        "detached HEAD".to_owned()
    } else {
        repo.head()?
            .name()
            .context("브랜치가 없습니다.")?
            .to_owned()
    };
    let onto = repo.revparse_single(a.last().unwrap())?.peel_to_commit()?;
    let commits = replay_commits(&repo, original, onto.id())?;
    let apply = repo
        .config()?
        .get_string("rebase.backend")
        .is_ok_and(|s| s == "apply");
    let dir = repo.path().join(if apply {
        "rebase-apply"
    } else {
        "rebase-merge"
    });
    std::fs::create_dir(&dir)?;
    if apply {
        std::fs::write(dir.join("rebasing"), b"")?;
    }
    for (name, value) in [
        ("orig-head", original.to_string()),
        ("onto", onto.id().to_string()),
        ("head-name", branch.clone()),
        (
            if apply { "last" } else { "end" },
            commits.len().to_string(),
        ),
        (if apply { "next" } else { "msgnum" }, "0".into()),
    ] {
        std::fs::write(dir.join(name), format!("{value}\n"))?;
    }
    let mut options = CheckoutBuilder::new();
    options.safe();
    if let Err(error) = repo.checkout_tree(onto.as_object(), Some(&mut options)) {
        clear_rebase(&repo)?;
        return Err(error.into());
    }
    repo.set_head_detached(onto.id())?;
    replay(&repo, &dir, &commits, 0, &branch, original)
}
fn replay(
    repo: &Repository,
    dir: &Path,
    commits: &[Oid],
    start: usize,
    branch: &str,
    original: Oid,
) -> Result<()> {
    for (i, id) in commits.iter().enumerate().skip(start) {
        let commit = repo.find_commit(*id)?;
        let apply = dir.ends_with("rebase-apply");
        std::fs::write(
            dir.join(if apply {
                "original-commit"
            } else {
                "stopped-sha"
            }),
            format!("{id}\n"),
        )?;
        std::fs::write(
            dir.join(if apply { "next" } else { "msgnum" }),
            format!("{}\n", i + 1),
        )?;
        let mut options = git2::CherrypickOptions::new();
        let mut checkout = CheckoutBuilder::new();
        checkout
            .safe()
            .allow_conflicts(true)
            .conflict_style_merge(true);
        options.checkout_builder(checkout);
        repo.cherrypick(&commit, Some(&mut options))?;
        if repo.index()?.has_conflicts() {
            bail!("Rebase 충돌 파일을 해결한 뒤 Stage하세요.");
        }
        let parent = repo.head()?.peel_to_commit()?;
        commit_index(
            repo,
            commit.message().unwrap_or("Rebased commit"),
            &[parent],
            Some(&commit.author()),
        )?;
        let _ = std::fs::remove_file(repo.path().join("CHERRY_PICK_HEAD"));
    }
    if branch.starts_with("refs/heads/") {
        let head = repo.head()?.target().context("HEAD가 없습니다.")?;
        repo.reference_matching(branch, head, true, original, "rebase: finished")?;
        repo.set_head(branch)?;
    }
    clear_rebase(repo)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RevertBackup {
    index: Vec<u8>,
    tree: String,
    paths: Vec<std::path::PathBuf>,
}
fn clear_revert(repo: &Repository) -> Result<()> {
    for name in [
        "REVERT_HEAD",
        "MERGE_MSG",
        "AUTO_MERGE",
        "feather-revert-backup.json",
    ] {
        let _ = std::fs::remove_file(repo.path().join(name));
    }
    let dir = repo.path().join("sequencer");
    if dir.is_dir() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}
pub fn revert(path: &Path, a: &[&str]) -> Result<()> {
    let repo = open_repository(path)?;
    if a.contains(&"--quit") {
        return clear_revert(&repo);
    }
    if a.contains(&"--abort") {
        let backup = repo.path().join("feather-revert-backup.json");
        if backup.exists() {
            let backup: RevertBackup = serde_json::from_slice(&std::fs::read(&backup)?)?;
            let tree = repo.find_tree(Oid::from_str(&backup.tree)?)?;
            let mut index = repo.index()?;
            index.read_tree(&tree)?;
            checkout_index_paths(&repo, &mut index, &backup.paths, true)?;
            std::fs::write(repo.path().join("index"), backup.index)?;
        } else {
            let head = std::fs::read_to_string(repo.path().join("sequencer/head"))
                .or_else(|_| std::fs::read_to_string(repo.path().join("ORIG_HEAD")))?;
            let object = repo.find_object(Oid::from_str(head.trim())?, None)?;
            repo.reset(&object, git2::ResetType::Hard, None)?;
        }
        return clear_revert(&repo);
    }
    if a.contains(&"--continue") || a.contains(&"--skip") {
        let marker = std::fs::read_to_string(repo.path().join("REVERT_HEAD"))?;
        let target = repo.find_commit(Oid::from_str(marker.trim())?)?;
        if a.contains(&"--continue") {
            let message = repo.message().unwrap_or_else(|_| {
                format!(
                    "Revert \"{}\"\n\nThis reverts commit {}.\n",
                    target.summary().ok().flatten().unwrap_or("commit"),
                    target.id()
                )
            });
            let parent = repo.head()?.peel_to_commit()?;
            commit_index(&repo, &message, &[parent], None)?;
        } else {
            let old = target.tree()?;
            let new = if target.parent_count() == 0 {
                repo.find_tree(repo.treebuilder(None)?.write()?)?
            } else {
                target.parent(0)?.tree()?
            };
            let paths = changed_paths(&repo, &old, &new)?;
            restore_head_paths(&repo, &paths)?;
        }
        let todo_path = repo.path().join("sequencer/todo");
        let todo = std::fs::read_to_string(&todo_path).unwrap_or_default();
        let remaining = todo
            .lines()
            .filter(|s| !s.trim().is_empty() && !s.starts_with('#'))
            .skip(1)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            return clear_revert(&repo);
        }
        std::fs::write(&todo_path, format!("{}\n", remaining.join("\n")))?;
        let no_commit = std::fs::read_to_string(repo.path().join("sequencer/opts"))
            .unwrap_or_default()
            .contains("no-commit = true");
        for (i, line) in remaining.iter().enumerate() {
            let revision = line
                .split_whitespace()
                .nth(1)
                .context("Revert 목록이 올바르지 않습니다.")?;
            let target = repo.revparse_single(revision)?.peel_to_commit()?;
            std::fs::write(&todo_path, format!("{}\n", remaining[i..].join("\n")))?;
            apply_revert(&repo, &target, no_commit, false)?;
        }
        return clear_revert(&repo);
    }
    let revision = a
        .last()
        .context("Revert 대상이 없습니다.")?
        .trim_end_matches("^!");
    let target = repo.revparse_single(revision)?.peel_to_commit()?;
    let no_commit = a.contains(&"--no-commit");
    apply_revert(&repo, &target, no_commit, true)
}
fn restore_head_paths(repo: &Repository, paths: &[std::path::PathBuf]) -> Result<()> {
    let tree = repo.head()?.peel_to_tree()?;
    let mut target = git2::Index::new()?;
    target.read_tree(&tree)?;
    checkout_index_paths(repo, &mut target, paths, true)?;
    let mut index = repo.index()?;
    for path in paths {
        let _ = index.remove_path(path);
        let _ = index.conflict_remove(path);
        if let Ok(entry) = tree.get_path(path) {
            let raw = path.to_string_lossy().replace('\\', "/");
            let mut e = git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: entry.filemode() as u32,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: entry.id(),
                flags: 0,
                flags_extended: 0,
                path: raw.into_bytes(),
            };
            e.flags = (e.path.len().min(0xfff)) as u16;
            index.add(&e)?;
        }
    }
    index.write()?;
    Ok(())
}
fn apply_revert(
    repo: &Repository,
    target: &git2::Commit<'_>,
    no_commit: bool,
    initial: bool,
) -> Result<()> {
    if target.parent_count() > 1 {
        bail!("병합 커밋은 mainline 지정 없이 Revert할 수 없습니다.");
    }
    let old = target.tree()?;
    let new = if target.parent_count() == 0 {
        repo.find_tree(repo.treebuilder(None)?.write()?)?
    } else {
        target.parent(0)?.tree()?
    };
    let paths = changed_paths(repo, &old, &new)?;
    preflight_paths(repo, &paths)?;
    let mut index = repo.index()?;
    let current_tree = repo.find_tree(index.write_tree()?)?;
    if !no_commit && current_tree.id() != repo.head()?.peel_to_tree()?.id() {
        bail!("Stage 변경사항을 먼저 커밋하거나 스태시하세요.");
    }
    let mut merged = repo.merge_trees(&old, &current_tree, &new, None)?;
    if initial {
        let backup = RevertBackup {
            index: std::fs::read(repo.path().join("index"))?,
            tree: current_tree.id().to_string(),
            paths: paths.clone(),
        };
        std::fs::write(
            repo.path().join("feather-revert-backup.json"),
            serde_json::to_vec(&backup)?,
        )?;
        let sequence = repo.path().join("sequencer");
        std::fs::create_dir_all(&sequence)?;
        std::fs::write(
            sequence.join("head"),
            format!("{}\n", repo.head()?.target().context("HEAD가 없습니다.")?),
        )?;
        std::fs::write(
            sequence.join("todo"),
            format!(
                "revert {} {}\n",
                target.id(),
                target.summary().ok().flatten().unwrap_or("commit")
            ),
        )?;
        std::fs::write(
            sequence.join("opts"),
            format!("[options]\n\tno-commit = {no_commit}\n"),
        )?;
    }
    std::fs::write(
        repo.path().join("REVERT_HEAD"),
        format!("{}\n", target.id()),
    )?;
    let message = format!(
        "Revert \"{}\"\n\nThis reverts commit {}.\n",
        target.summary().ok().flatten().unwrap_or("commit"),
        target.id()
    );
    std::fs::write(repo.path().join("MERGE_MSG"), &message)?;
    checkout_index_paths(repo, &mut merged, &paths, false)?;
    copy_index(repo, &merged)?;
    if merged.has_conflicts() {
        bail!("Revert 충돌 파일을 해결한 뒤 Stage하세요.");
    }
    if !no_commit {
        let parent = repo.head()?.peel_to_commit()?;
        commit_index(repo, &message, &[parent], None)?;
    }
    if initial {
        clear_revert(repo)?;
    }
    Ok(())
}
