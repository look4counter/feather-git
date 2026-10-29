use crate::model::{Content, DiffLine, FileChange};
use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{
    collections::BTreeSet,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Preview {
    pub before: String,
    pub after: String,
}

fn regular(content: &Content) -> bool {
    match content {
        Content::Object(_, mode) => mode & 0o170000 == 0o100000,
        Content::Empty | Content::Worktree => true,
    }
}

pub fn eligible(file: &FileChange) -> bool {
    !file.staged
        && file.status != "U"
        && matches!(file.after, Content::Empty | Content::Worktree)
        && regular(&file.before)
}

pub fn hunk_rows(lines: &[DiffLine], header: usize) -> Vec<usize> {
    if lines.get(header).is_none_or(|line| line.kind != '@') {
        return Vec::new();
    }
    lines
        .iter()
        .enumerate()
        .skip(header + 1)
        .take_while(|(_, line)| line.kind != '@')
        .filter(|(_, line)| matches!(line.kind, '+' | '-'))
        .map(|(row, _)| row)
        .collect()
}

impl Preview {
    pub fn selected_content(&self, lines: &[DiffLine], selected: &[usize]) -> Result<String> {
        let mut keys = BTreeSet::new();
        for row in selected {
            let line = lines.get(*row).context("선택한 줄을 다시 조회하세요.")?;
            if !matches!(line.kind, '+' | '-') {
                bail!("추가·삭제 줄만 Stage할 수 있습니다.");
            }
            keys.insert((line.kind, line.old, line.new));
        }
        if keys.is_empty() {
            bail!("Stage할 변경 줄을 선택하세요.");
        }
        let diff = similar::TextDiff::configure()
            .timeout(Duration::from_millis(500))
            .diff_lines(&self.before, &self.after);
        let mut result = String::new();
        let changes: Vec<_> = diff.iter_all_changes().collect();
        let mut position = 0;
        while position < changes.len() {
            if changes[position].tag() == similar::ChangeTag::Equal {
                result.push_str(changes[position].value());
                position += 1;
                continue;
            }
            let start = position;
            while position < changes.len() && changes[position].tag() != similar::ChangeTag::Equal {
                position += 1;
            }
            let removed: Vec<_> = changes[start..position]
                .iter()
                .filter(|change| change.tag() == similar::ChangeTag::Delete)
                .collect();
            let added: Vec<_> = changes[start..position]
                .iter()
                .filter(|change| change.tag() == similar::ChangeTag::Insert)
                .collect();
            // 연속 교체를 줄 순서로 짝지어 일부 교체의 원래 위치를 유지한다.
            for offset in 0..removed.len().max(added.len()) {
                if let Some(change) = removed.get(offset)
                    && !keys.remove(&('-', change.old_index().map(|i| i + 1), None))
                {
                    result.push_str(change.value());
                }
                if let Some(change) = added.get(offset)
                    && keys.remove(&('+', None, change.new_index().map(|i| i + 1)))
                {
                    result.push_str(change.value());
                }
            }
        }
        if !keys.is_empty() || result == self.before {
            bail!("선택한 변경을 적용할 수 없습니다. 다시 조회하세요.");
        }
        Ok(result)
    }
}

fn command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root).env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn quoted(prefix: &str, raw: &[u8]) -> String {
    let mut result = format!("\"{prefix}");
    for byte in raw {
        match byte {
            b'"' => result.push_str("\\\""),
            b'\\' => result.push_str("\\\\"),
            b' '..=b'~' => result.push(*byte as char),
            _ => result.push_str(&format!("\\{byte:03o}")),
        }
    }
    result.push('"');
    result
}

fn patch_lines(patch: &mut String, source: &str, prefix: char) {
    for line in source.split_inclusive('\n') {
        patch.push(prefix);
        patch.push_str(line);
        if !line.ends_with('\n') {
            patch.push_str("\n\\ No newline at end of file\n");
        }
    }
}

fn patch(file: &FileChange, before: &str, selected: &str) -> String {
    let old = quoted("a/", &file.path);
    let new = quoted("b/", &file.path);
    let mut result = format!("diff --git {old} {new}\n");
    let remove = matches!(file.after, Content::Empty) && selected.is_empty();
    if matches!(file.before, Content::Empty) {
        result.push_str("new file mode 100644\n");
    } else if remove {
        let mode = match file.before {
            Content::Object(_, mode) => mode,
            _ => 0o100644,
        };
        result.push_str(&format!("deleted file mode {mode:o}\n"));
    }
    result.push_str(&format!(
        "--- {}\n+++ {}\n",
        if matches!(file.before, Content::Empty) {
            "/dev/null"
        } else {
            &old
        },
        if remove { "/dev/null" } else { &new }
    ));
    let old_count = before.split_inclusive('\n').count();
    let new_count = selected.split_inclusive('\n').count();
    result.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        usize::from(old_count > 0),
        old_count,
        usize::from(new_count > 0),
        new_count
    ));
    patch_lines(&mut result, before, '-');
    patch_lines(&mut result, selected, '+');
    result
}

fn validate_path(root: &Path, raw: &[u8]) -> Result<()> {
    if raw.is_empty() || raw.contains(&0) {
        bail!("잘못된 파일 경로입니다.");
    }
    let relative = gix::path::from_bstr(raw.as_bstr());
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            bail!("저장소 내부 파일만 Stage할 수 있습니다.");
        };
        if name.to_string_lossy().eq_ignore_ascii_case(".git") {
            bail!("Git 메타데이터는 Stage할 수 없습니다.");
        }
        path.push(name);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    bail!("링크 경로는 부분 Stage할 수 없습니다.");
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        bail!("링크 경로는 부분 Stage할 수 없습니다.");
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub fn apply(
    root: &Path,
    file: &FileChange,
    preview: &Preview,
    lines: &[DiffLine],
    selected: &[usize],
) -> Result<()> {
    if !eligible(file) {
        bail!("일반 미스테이지 텍스트 파일만 부분 Stage할 수 있습니다.");
    }
    let repo = crate::git::open(root)?;
    let root = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    validate_path(root, &file.path)?;
    let index = repo.index_or_empty()?;
    if index.entries().iter().any(|entry| {
        entry.path(&index).as_bytes() == file.path
            && entry.stage() != gix::index::entry::Stage::Unconflicted
    }) {
        bail!("충돌 파일은 먼저 해결한 뒤 전체 Stage하세요.");
    }
    let entry = index.entry_by_path(file.path.as_bstr());
    let unchanged = match (&file.before, entry) {
        (Content::Empty, None) => true,
        (Content::Object(id, mode), Some(entry)) => {
            entry.id == *id && entry.mode.bits() as u16 == *mode
        }
        _ => false,
    };
    if !unchanged {
        bail!("인덱스가 변경되었습니다. 다시 조회하세요.");
    }
    let path = gix::path::from_bstr(file.path.as_bstr());
    let attrs = command(root)
        .args(["check-attr", "-z", "filter", "working-tree-encoding", "--"])
        .arg(path.as_ref())
        .output()?;
    if !attrs.status.success() {
        bail!("파일 속성을 확인하지 못했습니다.");
    }
    for record in attrs
        .stdout
        .split(|byte| *byte == 0)
        .collect::<Vec<_>>()
        .chunks(3)
    {
        if record.len() == 3
            && ![b"unspecified".as_slice(), b"unset".as_slice()].contains(&record[2])
        {
            bail!("변환 필터가 있는 파일입니다. 파일 전체 Stage를 사용하세요.");
        }
    }
    let before = crate::git::read_content(&repo, file, &file.before)?;
    let after = crate::git::read_content(&repo, file, &file.after)?;
    // 삭제된 파일이 다시 만들어진 경우도 오래된 미리보기로 취급한다.
    if matches!(file.after, Content::Empty) && root.join(path.as_ref()).symlink_metadata().is_ok() {
        bail!("작업 파일이 변경되었습니다. 다시 조회하세요.");
    }
    if before != preview.before.as_bytes()
        || after != preview.after.as_bytes()
        || before.contains(&0)
        || after.contains(&0)
    {
        bail!("파일 내용이 변경되었습니다. 다시 조회하세요.");
    }
    std::str::from_utf8(&before)?;
    std::str::from_utf8(&after)?;
    let selected = preview.selected_content(lines, selected)?;
    let patch = patch(file, &preview.before, &selected);
    let mut child = command(root)
        .args([
            "apply",
            "--cached",
            "--unidiff-zero",
            "--whitespace=nowarn",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let written = child
        .stdin
        .take()
        .context("패치 입력을 열 수 없습니다.")?
        .write_all(patch.as_bytes());
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "부분 Stage 실패: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    written.context("패치를 전달하지 못했습니다.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git(root: &Path, args: &[&str]) -> Vec<u8> {
        let output = command(root).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn fixture(name: &str, source: &str) -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init", "-b", "main"]);
        for (key, value) in [
            ("user.name", "Test"),
            ("user.email", "test@example.invalid"),
            ("core.autocrlf", "false"),
            ("commit.gpgsign", "false"),
        ] {
            git(root, &["config", key, value]);
        }
        fs::write(root.join(name), source).unwrap();
        git(root, &["add", "--", name]);
        git(root, &["commit", "-m", "base"]);
        temp
    }

    fn preview(root: &Path, name: &str) -> (FileChange, crate::model::Diff) {
        let file = crate::git::load_stage(root)
            .unwrap()
            .into_iter()
            .find(|file| !file.staged && file.path == name.as_bytes())
            .unwrap();
        let diff = crate::git::diff(root, &file).unwrap();
        assert!(diff.partial.is_some());
        (file, diff)
    }

    fn stage(root: &Path, file: &FileChange, diff: &crate::model::Diff, rows: &[usize]) {
        apply(
            root,
            file,
            diff.partial.as_ref().unwrap(),
            &diff.lines,
            rows,
        )
        .unwrap();
    }

    #[test]
    fn hunk_staging_preserves_existing_stage_other_files_and_worktree() {
        let name = "file.txt";
        let original = (0..25)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        let temp = fixture(name, &original);
        let root = temp.path();
        let indexed = original.replace("line 0\n", "already staged\n");
        fs::write(root.join(name), &indexed).unwrap();
        fs::write(root.join("other.txt"), "other staged\n").unwrap();
        git(root, &["add", "."]);
        let worktree = indexed
            .replace("line 5\n", "first changed\n")
            .replace("line 20\n", "second changed\n");
        fs::write(root.join(name), &worktree).unwrap();
        let (file, diff) = preview(root, name);
        let headers: Vec<_> = diff
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind == '@')
            .map(|(row, _)| row)
            .collect();
        assert_eq!(headers.len(), 2);
        stage(root, &file, &diff, &hunk_rows(&diff.lines, headers[0]));
        assert_eq!(
            git(root, &["show", ":file.txt"]),
            indexed.replace("line 5\n", "first changed\n").as_bytes()
        );
        assert_eq!(git(root, &["show", ":other.txt"]), b"other staged\n");
        assert_eq!(fs::read(root.join(name)).unwrap(), worktree.as_bytes());
        let (file, diff) = preview(root, name);
        stage(root, &file, &diff, &hunk_rows(&diff.lines, 0));
        assert_eq!(git(root, &["show", ":file.txt"]), worktree.as_bytes());
        assert_eq!(fs::read(root.join(name)).unwrap(), worktree.as_bytes());
    }

    #[test]
    fn individual_added_and_deleted_lines_can_be_staged_independently() {
        for kind in ['+', '-'] {
            let temp = fixture("file.txt", "before\nold\nafter\n");
            let root = temp.path();
            fs::write(root.join("file.txt"), "before\nnew\nafter\n").unwrap();
            let (file, diff) = preview(root, "file.txt");
            let row = diff
                .lines
                .iter()
                .position(|line| line.kind == kind)
                .unwrap();
            stage(root, &file, &diff, &[row]);
            let expected = if kind == '+' {
                "before\nold\nnew\nafter\n"
            } else {
                "before\nafter\n"
            };
            assert_eq!(git(root, &["show", ":file.txt"]), expected.as_bytes());
            assert_eq!(
                fs::read(root.join("file.txt")).unwrap(),
                b"before\nnew\nafter\n"
            );
        }
    }

    #[test]
    fn unicode_literal_paths_crlf_and_missing_final_newline_preserve_raw_content() {
        let name = "-한글 [x].txt";
        for (before, after, expected) in [
            ("one\r\ntwo\r\n", "ONE\r\nTWO\r\n", "ONE\r\ntwo\r\n"),
            ("one\nlast", "ONE\nLAST", "ONE\nlast"),
        ] {
            let temp = fixture(name, before);
            let root = temp.path();
            fs::write(root.join(name), after).unwrap();
            let (file, diff) = preview(root, name);
            let rows: Vec<_> = diff
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| {
                    (line.kind == '-' && line.old == Some(1))
                        || (line.kind == '+' && line.new == Some(1))
                })
                .map(|(row, _)| row)
                .collect();
            stage(root, &file, &diff, &rows);
            assert_eq!(
                git(root, &["show", &format!(":{name}")]),
                expected.as_bytes()
            );
            assert_eq!(fs::read(root.join(name)).unwrap(), after.as_bytes());
        }
        assert_eq!(quoted("a/", b"x\t\"\\\xff"), "\"a/x\\011\\\"\\\\\\377\"");
    }

    #[test]
    fn new_and_deleted_files_support_partial_lines_and_complete_removal() {
        let temp = fixture("base.txt", "base\n");
        let root = temp.path();
        fs::write(root.join("new.txt"), "first\nsecond\n").unwrap();
        let (file, diff) = preview(root, "new.txt");
        let row = diff
            .lines
            .iter()
            .position(|line| line.new == Some(1))
            .unwrap();
        stage(root, &file, &diff, &[row]);
        assert_eq!(git(root, &["show", ":new.txt"]), b"first\n");
        assert_eq!(fs::read(root.join("new.txt")).unwrap(), b"first\nsecond\n");
        git(root, &["add", "new.txt"]);
        fs::remove_file(root.join("new.txt")).unwrap();
        let (file, diff) = preview(root, "new.txt");
        let row = diff
            .lines
            .iter()
            .position(|line| line.old == Some(1))
            .unwrap();
        stage(root, &file, &diff, &[row]);
        assert_eq!(git(root, &["show", ":new.txt"]), b"second\n");
        assert!(!root.join("new.txt").exists());
        let (file, diff) = preview(root, "new.txt");
        stage(root, &file, &diff, &hunk_rows(&diff.lines, 0));
        assert!(git(root, &["ls-files", "--", "new.txt"]).is_empty());
    }

    #[test]
    fn stale_files_index_filters_and_invalid_selection_are_rejected_without_writes() {
        let temp = fixture("file.txt", "old\n");
        let root = temp.path();
        fs::write(root.join("file.txt"), "new\n").unwrap();
        let (file, diff) = preview(root, "file.txt");
        let rows = hunk_rows(&diff.lines, 0);
        let original_index = fs::read(root.join(".git/index")).unwrap();
        for selected in [vec![], vec![0], vec![usize::MAX]] {
            assert!(
                apply(
                    root,
                    &file,
                    diff.partial.as_ref().unwrap(),
                    &diff.lines,
                    &selected
                )
                .is_err()
            );
            assert_eq!(fs::read(root.join(".git/index")).unwrap(), original_index);
        }
        fs::write(root.join("file.txt"), "external\n").unwrap();
        assert!(
            apply(
                root,
                &file,
                diff.partial.as_ref().unwrap(),
                &diff.lines,
                &rows
            )
            .is_err()
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), original_index);
        fs::write(root.join("file.txt"), "new\n").unwrap();
        fs::write(root.join(".gitattributes"), "file.txt filter=custom\n").unwrap();
        assert!(
            apply(
                root,
                &file,
                diff.partial.as_ref().unwrap(),
                &diff.lines,
                &rows
            )
            .unwrap_err()
            .to_string()
            .contains("필터")
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), original_index);
        fs::remove_file(root.join(".gitattributes")).unwrap();
        git(root, &["add", "file.txt"]);
        let changed_index = fs::read(root.join(".git/index")).unwrap();
        assert!(
            apply(
                root,
                &file,
                diff.partial.as_ref().unwrap(),
                &diff.lines,
                &rows
            )
            .unwrap_err()
            .to_string()
            .contains("인덱스")
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), changed_index);
        assert_eq!(fs::read(root.join("file.txt")).unwrap(), b"new\n");
    }
}
