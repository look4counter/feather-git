use feather_git::{git, model::*};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

fn imported_history(count: usize) -> TempDir {
    use std::io::Write;
    let dir = init();
    let mut input = b"blob\nmark :1\ndata 6\nhello\n\n".to_vec();
    for index in 0..count {
        let message = format!("commit {index}\n");
        input.extend_from_slice(format!("commit refs/heads/main\nmark :{}\ncommitter Test <test@example.invalid> {} +0000\ndata {}\n{}\n", index + 2, 1700000000 + index, message.len(), message).as_bytes());
        if index > 0 {
            input.extend_from_slice(format!("from :{}\n", index + 1).as_bytes());
        } else {
            input.extend_from_slice(b"M 100644 :1 file.txt\n");
        }
        input.push(b'\n');
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["fast-import", "--quiet"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    dir
}

fn run(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args([
            "-c",
            "core.autocrlf=false",
            "-c",
            "user.name=테스트",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .expect("테스트 준비에 Git이 필요합니다");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn init() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), &["init", "-b", "main"]);
    dir
}

fn write(path: &Path, name: &str, value: impl AsRef<[u8]>) {
    fs::write(path.join(name), value).unwrap();
}

fn commit(path: &Path, message: &str) {
    run(path, &["add", "."]);
    run(path, &["commit", "-m", message]);
}

#[test]
fn empty_repository_and_invalid_path() {
    let dir = init();
    let snapshot = git::load(dir.path(), None, None).unwrap();
    assert!(snapshot.commits.is_empty());
    assert!(snapshot.commit_detail.is_none());
    assert!(snapshot.history.is_empty());
    assert_eq!(snapshot.head, "main");
    write(dir.path(), "새 파일.txt", "초기 파일\n");
    let snapshot = git::load(dir.path(), None, None).unwrap();
    assert_eq!(snapshot.stage.len(), 1);
    assert_eq!(snapshot.stage[0].status, "A");
    assert!(!snapshot.stage[0].staged);
    run(dir.path(), &["add", "."]);
    let snapshot = git::load(dir.path(), None, None).unwrap();
    assert!(snapshot.stage[0].staged);
    let invalid = tempfile::tempdir().unwrap();
    assert!(git::load(invalid.path(), None, None).is_err());
}

#[test]
fn references_history_stage_and_read_only() {
    let dir = init();
    let path = dir.path();
    write(path, "sample.txt", "first\n");
    write(path, "deleted.txt", "delete me\n");
    commit(path, "첫 커밋");
    let first = run(path, &["rev-parse", "HEAD"]);
    run(path, &["branch", "feature"]);
    run(path, &["tag", "v0.1"]);
    run(path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    write(path, "sample.txt", "second\n");
    commit(path, "두 번째 커밋");
    let second = run(path, &["rev-parse", "HEAD"]);
    write(path, "sample.txt", "staged\n");
    run(path, &["add", "sample.txt"]);
    write(path, "sample.txt", "worktree\n");
    fs::remove_file(path.join("deleted.txt")).unwrap();
    write(path, "untracked.txt", "new\n");
    write(path, ".gitignore", "ignored.txt\n");
    write(path, "ignored.txt", "ignore\n");
    let before = run(path, &["status", "--porcelain=v1"]);
    let index_before = fs::read(path.join(".git/index")).unwrap();
    let snapshot = git::load(path, None, None).unwrap();
    assert_eq!(snapshot.commits.len(), 2);
    assert_eq!(snapshot.commits[0].id, second);
    for tab in [RefTab::Local, RefTab::Remote, RefTab::Tag] {
        assert!(snapshot.references.iter().any(|r| r.kind == tab));
    }
    let staged = snapshot
        .stage
        .iter()
        .find(|f| f.label == "sample.txt" && f.staged)
        .unwrap();
    let working = snapshot
        .stage
        .iter()
        .find(|f| f.label == "sample.txt" && !f.staged)
        .unwrap();
    let diff = git::diff(path, staged).unwrap();
    assert!(
        diff.lines
            .iter()
            .any(|l| l.kind == '-' && l.text == "second")
    );
    assert!(
        diff.lines
            .iter()
            .any(|l| l.kind == '+' && l.text == "staged")
    );
    let diff = git::diff(path, working).unwrap();
    assert!(
        diff.lines
            .iter()
            .any(|l| l.kind == '-' && l.text == "staged")
    );
    assert!(
        diff.lines
            .iter()
            .any(|l| l.kind == '+' && l.text == "worktree")
    );
    assert!(
        snapshot
            .stage
            .iter()
            .any(|f| f.label == "deleted.txt" && f.status == "D")
    );
    assert!(!snapshot.stage.iter().any(|f| f.label == "ignored.txt"));
    let root = git::load(path, Some("refs/heads/feature"), Some(&first)).unwrap();
    assert_eq!(root.commits.len(), 2);
    assert_eq!(root.history.len(), 2);
    assert!(root.history.iter().all(|f| f.status == "A"));
    assert_eq!(before, run(path, &["status", "--porcelain=v1"]));
    assert_eq!(index_before, fs::read(path.join(".git/index")).unwrap());
    assert_eq!(run(path, &["rev-parse", "HEAD"]), second);
}

#[test]
fn stash_reflog_packed_refs_and_missing_reference_refresh() {
    let dir = init();
    let path = dir.path();
    write(path, "file.txt", "base\n");
    commit(path, "기준");
    run(path, &["tag", "-a", "release", "-m", "릴리스"]);
    for value in ["one\n", "two\n"] {
        write(path, "file.txt", value);
        run(path, &["stash", "push", "-m", value.trim()]);
    }
    run(path, &["branch", "temporary"]);
    run(path, &["pack-refs", "--all"]);
    let snapshot = git::load(path, None, None).unwrap();
    let stash: Vec<_> = snapshot
        .references
        .iter()
        .filter(|r| r.kind == RefTab::Stash)
        .collect();
    assert_eq!(stash.len(), 2);
    assert!(stash[0].name.contains("stash@{0}") && stash[0].name.contains("two"));
    let snapshot = git::load(path, Some(&stash[0].revision), None).unwrap();
    assert!(snapshot.commits.iter().any(|c| c.title.contains("two")));
    run(path, &["branch", "-D", "temporary"]);
    let snapshot = git::load(
        path,
        Some("refs/heads/temporary"),
        Some("0000000000000000000000000000000000000000"),
    )
    .unwrap();
    assert!(snapshot.commits.iter().any(|c| c.title == "기준"));
}

#[test]
fn binary_large_file_and_diff_limits() {
    let dir = init();
    let path = dir.path();
    write(path, "binary.dat", [1, 0, 2]);
    write(path, "large.txt", vec![b'x'; git::MAX_BYTES as usize + 1]);
    write(path, "many.txt", "line\n".repeat(5100));
    let snapshot = git::load(path, None, None).unwrap();
    for (name, note) in [
        ("binary.dat", "바이너리"),
        ("large.txt", "1 MiB"),
        ("many.txt", "5,000"),
    ] {
        let file = snapshot.stage.iter().find(|f| f.label == name).unwrap();
        let diff = git::diff(path, file).unwrap();
        assert!(diff.note.contains(note), "{name}: {}", diff.note);
        assert!(diff.lines.len() <= 5000);
    }
}

#[test]
fn detached_head_bare_repository_and_mode_change() {
    let dir = init();
    let path = dir.path();
    write(path, "script.sh", "echo hello\n");
    commit(path, "기준");
    run(path, &["update-index", "--chmod=+x", "script.sh"]);
    run(path, &["commit", "-m", "실행 모드"]);
    let snapshot = git::load(path, None, None).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].label, "script.sh");
    run(path, &["checkout", "--detach"]);
    assert_eq!(git::load(path, None, None).unwrap().head, "Detached HEAD");
    let bare = tempfile::tempdir().unwrap();
    run(
        bare.path(),
        &["clone", "--bare", path.to_str().unwrap(), "."],
    );
    let snapshot = git::load(bare.path(), None, None).unwrap();
    assert_eq!(snapshot.commits.len(), 2);
    assert!(snapshot.stage.is_empty());
}

#[test]
fn merge_history_uses_first_parent() {
    let dir = init();
    let path = dir.path();
    write(path, "base.txt", "base\n");
    commit(path, "기준");
    run(path, &["checkout", "-b", "feature"]);
    write(path, "feature.txt", "feature\n");
    commit(path, "기능");
    run(path, &["checkout", "main"]);
    write(path, "main.txt", "main\n");
    commit(path, "메인");
    run(path, &["merge", "--no-ff", "feature", "-m", "병합"]);
    let snapshot = git::load(path, None, None).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].label, "feature.txt");
    let detail = snapshot.commit_detail.as_ref().unwrap();
    assert_eq!(detail.parent_ids.len(), 2);
    assert_eq!(detail.id, run(path, &["rev-parse", "HEAD"]));
    assert_eq!(detail.parent_ids[0], run(path, &["rev-parse", "HEAD^1"]));
    assert_eq!(detail.parent_ids[1], run(path, &["rev-parse", "HEAD^2"]));
    assert_eq!(snapshot.commits[0].parent_ids, detail.parent_ids);
    assert_eq!(snapshot.commits[0].output_edges.len(), 2);
}

#[test]
fn commit_details_keep_full_message_signatures_timezones_and_selection() {
    let dir = init();
    let path = dir.path();
    write(path, "detail.txt", "first\n");
    run(path, &["add", "."]);
    let message = format!(
        "제목\n\n본문의 빈 줄과 한글을 보존합니다.\n{}\n",
        "긴 본문\n".repeat(150)
    );
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args([
            "-c",
            "user.name=커미터",
            "-c",
            "user.email=committer@example.invalid",
            "commit",
            "--author=작성자 <author@example.invalid>",
            "--cleanup=verbatim",
            "-m",
            &message,
        ])
        .env("GIT_AUTHOR_DATE", "2026-10-27T09:12:34+0900")
        .env("GIT_COMMITTER_DATE", "2026-10-27T10:23:45-0400")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = run(path, &["rev-parse", "HEAD"]);
    let detail = git::load(path, None, None).unwrap().commit_detail.unwrap();
    assert_eq!(detail.id, first);
    assert!(detail.parent_ids.is_empty());
    assert_eq!(detail.author, "작성자");
    assert_eq!(detail.author_email, "author@example.invalid");
    assert_eq!(detail.committer, "커미터");
    assert_eq!(detail.committer_email, "committer@example.invalid");
    assert_eq!(detail.author_offset, 9 * 3600);
    assert_eq!(detail.commit_offset, -4 * 3600);
    assert_ne!(detail.author_time, detail.commit_time);
    assert_eq!(detail.message, message);
    write(path, "detail.txt", "second\n");
    commit(path, "후속 커밋");
    let second = run(path, &["rev-parse", "HEAD"]);
    let selected = git::load(path, None, Some(&first)).unwrap();
    assert_eq!(selected.commit_detail.unwrap().id, first);
    let fallback = git::load(path, None, Some("invalid")).unwrap();
    assert_eq!(fallback.commit_detail.unwrap().id, second);
}

#[test]
fn history_includes_more_than_200_commits() {
    let dir = init();
    let path = dir.path();
    write(path, "file.txt", "base\n");
    commit(path, "시작");
    let tree = run(path, &["rev-parse", "HEAD^{tree}"]);
    let mut parent = run(path, &["rev-parse", "HEAD"]);
    for i in 0..204 {
        parent = run(
            path,
            &[
                "commit-tree",
                &tree,
                "-p",
                &parent,
                "-m",
                &format!("커밋 {i}"),
            ],
        );
    }
    run(path, &["update-ref", "refs/heads/main", &parent]);
    let snapshot = git::load(path, None, None).unwrap();
    assert_eq!(snapshot.commits.len(), 205);
    assert_eq!(snapshot.commits[0].id, parent);
    assert!(snapshot.commits.last().unwrap().parent_ids.is_empty());
    assert!(snapshot.commits.last().unwrap().output_edges.is_empty());
    assert_eq!(snapshot.commit_detail.unwrap().id, parent);
}

#[test]
fn all_refs_and_detached_head_are_visible_without_branch_filtering() {
    let dir = init();
    let path = dir.path();
    write(path, "base.txt", "base\n");
    commit(path, "base");
    let base = run(path, &["rev-parse", "HEAD"]);
    run(path, &["checkout", "-b", "feature"]);
    write(path, "feature.txt", "feature\n");
    commit(path, "feature");
    let feature = run(path, &["rev-parse", "HEAD"]);
    run(path, &["checkout", "main"]);
    write(path, "main.txt", "main\n");
    commit(path, "main");
    let main = run(path, &["rev-parse", "HEAD"]);
    let tree = run(path, &["rev-parse", "HEAD^{tree}"]);
    let remote = run(path, &["commit-tree", &tree, "-m", "remote only"]);
    let tag = run(path, &["commit-tree", &tree, "-m", "tag only"]);
    let detached = run(path, &["commit-tree", &tree, "-m", "detached"]);
    let unreachable = run(path, &["commit-tree", &tree, "-m", "unreachable"]);
    run(
        path,
        &["update-ref", "refs/remotes/origin/remote-only", &remote],
    );
    run(path, &["update-ref", "refs/remotes/origin/main", &main]);
    run(path, &["tag", "-a", "tag-only", &tag, "-m", "tag"]);
    run(path, &["checkout", "--detach", &detached]);
    let snapshot = git::load(path, None, None).unwrap();
    let ids: std::collections::HashSet<_> =
        snapshot.commits.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), 6);
    for id in [&base, &main, &feature, &remote, &tag, &detached] {
        assert!(ids.contains(id.as_str()));
    }
    assert!(!ids.contains(unreachable.as_str()));
    let main_row = snapshot.commits.iter().find(|c| c.id == main).unwrap();
    assert_eq!(main_row.branches, ["main", "origin/main"]);
    assert!(
        snapshot
            .commits
            .iter()
            .find(|c| c.id == remote)
            .unwrap()
            .branches
            .contains(&"origin/remote-only".into())
    );
    for revision in [
        "refs/heads/feature",
        "refs/remotes/origin/main",
        "refs/tags/tag-only",
    ] {
        let selected = git::load(path, Some(revision), Some(&feature)).unwrap();
        let selected_ids: std::collections::HashSet<_> =
            selected.commits.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, selected_ids);
        assert_eq!(selected.commit_detail.unwrap().id, feature);
    }
    let selection = git::load_commit(path, &feature).unwrap();
    assert_eq!(selection.detail.id, feature);
    assert!(selection.history.iter().any(|f| f.label == "feature.txt"));
    assert!(git::load_commit(path, "invalid").is_err());
}

#[test]
fn initial_and_incremental_pages_are_bounded_and_complete() {
    for count in [0, 200, 400, 450] {
        let dir = imported_history(count);
        let mut snapshot = git::load_initial(dir.path(), None).unwrap();
        assert_eq!(snapshot.commits.len(), count.min(200));
        assert_eq!(snapshot.has_more, count > 200);
        assert_eq!(snapshot.current_branch.as_deref(), Some("refs/heads/main"));
        let session = snapshot.session.clone().unwrap();
        while snapshot.has_more {
            let page = session.next_page().unwrap();
            assert!(page.commits.len() <= 200);
            assert!(!page.commits.is_empty());
            snapshot.commits.extend(page.commits);
            snapshot.has_more = page.has_more;
        }
        assert_eq!(snapshot.commits.len(), count);
        let unique: std::collections::HashSet<_> = snapshot.commits.iter().map(|c| &c.id).collect();
        assert_eq!(unique.len(), count);
        let finished = session.next_page().unwrap();
        assert!(finished.commits.is_empty() && !finished.has_more);
        for rows in snapshot.commits.windows(2) {
            assert_eq!(rows[0].parent_ids[0], rows[1].id);
            assert_eq!(
                rows[0].output_edges[0].to_lane,
                rows[1].input_edges[0].from_lane
            );
            assert_eq!(rows[0].output_edges[0].color, rows[1].input_edges[0].color);
        }
    }
}

#[test]
fn checkout_markers_and_stage_virtual_indices_preserve_identity() {
    let dir = init();
    let path = dir.path();
    write(path, "a.txt", "one\n");
    commit(path, "base");
    run(path, &["branch", "feature"]);
    write(path, "a.txt", "staged\n");
    run(path, &["add", "."]);
    write(path, "b.txt", "untracked\n");
    let snapshot = git::load_initial(path, None).unwrap();
    assert_eq!(snapshot.current_branch.as_deref(), Some("refs/heads/main"));
    let current: Vec<_> = snapshot
        .references
        .iter()
        .filter(|r| Some(&r.revision) == snapshot.current_branch.as_ref())
        .collect();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].name, "main");
    assert_eq!(
        file_rows(&snapshot.stage, true),
        [
            FileRow::Group(true),
            FileRow::File(0),
            FileRow::Group(false),
            FileRow::File(1)
        ]
    );
    assert_eq!(
        file_rows(&snapshot.stage, false),
        [FileRow::File(0), FileRow::File(1)]
    );
    run(path, &["checkout", "--detach"]);
    assert!(
        git::load_initial(path, None)
            .unwrap()
            .current_branch
            .is_none()
    );
    let invalid =
        feather_git::paging::HistorySession::start(path.join("missing"), Vec::new(), &[]).unwrap();
    assert!(invalid.next_page().is_err());
}

#[test]
fn multiple_diff_groups_never_exceed_line_limit() {
    let dir = init();
    let path = dir.path();
    let common = (0..30).map(|i| format!("common {i}\n")).collect::<String>();
    write(path, "file.txt", format!("{common}old tail\n"));
    commit(path, "기준");
    write(
        path,
        "file.txt",
        format!("{}{common}new tail\n", "added\n".repeat(4996)),
    );
    let snapshot = git::load(path, None, None).unwrap();
    let diff = git::diff(path, &snapshot.stage[0]).unwrap();
    assert_eq!(diff.lines.len(), 5000);
    assert!(diff.note.contains("5,000"));
}

#[test]
fn special_objects_and_large_staged_blob() {
    let dir = init();
    let path = dir.path();
    write(path, "file.txt", "target\n");
    commit(path, "기준");
    let blob = run(path, &["rev-parse", "HEAD:file.txt"]);
    let head = run(path, &["rev-parse", "HEAD"]);
    run(
        path,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("120000,{blob},link"),
        ],
    );
    run(
        path,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{head},module"),
        ],
    );
    run(path, &["commit", "-m", "특수 파일"]);
    let snapshot = git::load(path, None, None).unwrap();
    for name in ["link", "module"] {
        let file = snapshot.history.iter().find(|f| f.label == name).unwrap();
        assert!(git::diff(path, file).unwrap().note.contains("서브모듈"));
    }
    write(path, "large.txt", vec![b'x'; git::MAX_BYTES as usize + 1]);
    run(path, &["add", "large.txt"]);
    let snapshot = git::load(path, None, None).unwrap();
    let file = snapshot
        .stage
        .iter()
        .find(|f| f.label == "large.txt")
        .unwrap();
    assert!(file.staged);
    assert!(git::diff(path, file).unwrap().note.contains("1 MiB"));
}

#[test]
fn conflicts_are_identified() {
    let dir = init();
    let path = dir.path();
    write(path, "file.txt", "base\n");
    commit(path, "기준");
    run(path, &["checkout", "-b", "feature"]);
    write(path, "file.txt", "feature\n");
    commit(path, "기능");
    run(path, &["checkout", "main"]);
    write(path, "file.txt", "main\n");
    commit(path, "메인");
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["merge", "feature"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let snapshot = git::load(path, None, None).unwrap();
    assert!(
        snapshot
            .stage
            .iter()
            .any(|f| f.label == "file.txt" && f.status == "U")
    );
}

#[test]
fn external_filters_are_not_executed() {
    let dir = init();
    let path = dir.path();
    write(path, ".gitattributes", "file.txt filter=custom\n");
    write(path, "file.txt", "base\n");
    commit(path, "기준");
    run(
        path,
        &[
            "config",
            "filter.custom.clean",
            "echo called > filter-called.txt; cat",
        ],
    );
    run(path, &["config", "filter.custom.required", "true"]);
    write(path, "file.txt", "edit\n");
    let snapshot = git::load(path, None, None).unwrap();
    assert!(snapshot.stage.iter().any(|f| f.label == "file.txt"));
    assert!(!path.join("filter-called.txt").exists());
}
