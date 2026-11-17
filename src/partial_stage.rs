use crate::diff_model::{Content, DiffLine, FileChange};
use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
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
        self.content_for_selection(lines, selected, false)
    }

    pub fn discarded_content(&self, lines: &[DiffLine], selected: &[usize]) -> Result<String> {
        self.content_for_selection(lines, selected, true)
    }

    fn content_for_selection(
        &self,
        lines: &[DiffLine],
        selected: &[usize],
        discard: bool,
    ) -> Result<String> {
        let mut keys = BTreeSet::new();
        for row in selected {
            let line = lines.get(*row).context("선택한 줄을 다시 조회하세요.")?;
            if !matches!(line.kind, '+' | '-') {
                bail!("추가·삭제 줄만 선택할 수 있습니다.");
            }
            keys.insert((line.kind, line.old, line.new));
        }
        if keys.is_empty() {
            bail!("변경 줄을 선택하세요.");
        }
        use similar::DiffableStr;
        let comparison_before = crate::line_endings::comparison_text(&self.before);
        let comparison_after = crate::line_endings::comparison_text(&self.after);
        let raw_before = self.before.as_str().tokenize_lines();
        let raw_after = self.after.as_str().tokenize_lines();
        let ending = if discard {
            crate::line_endings::preferred(&self.after, &self.before)
        } else {
            crate::line_endings::preferred(&self.before, &self.after)
        };
        let diff = similar::TextDiff::configure()
            .timeout(Duration::from_millis(500))
            .diff_lines(comparison_before.as_ref(), comparison_after.as_ref());
        let mut result = String::new();
        let changes: Vec<_> = diff.iter_all_changes().collect();
        let mut position = 0;
        while position < changes.len() {
            if changes[position].tag() == similar::ChangeTag::Equal {
                let change = &changes[position];
                let raw = if discard {
                    change.new_index().and_then(|index| raw_after.get(index))
                } else {
                    change.old_index().and_then(|index| raw_before.get(index))
                }
                .context("원문 줄을 찾을 수 없습니다.")?;
                result.push_str(raw);
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
                    && keys.remove(&('-', change.old_index().map(|i| i + 1), None)) == discard
                {
                    let raw = change
                        .old_index()
                        .and_then(|index| raw_before.get(index))
                        .context("원문 삭제 줄을 찾을 수 없습니다.")?;
                    if discard {
                        crate::line_endings::push_line(&mut result, raw, ending);
                    } else {
                        result.push_str(raw);
                    }
                }
                if let Some(change) = added.get(offset)
                    && keys.remove(&('+', None, change.new_index().map(|i| i + 1))) != discard
                {
                    let raw = change
                        .new_index()
                        .and_then(|index| raw_after.get(index))
                        .context("원문 추가 줄을 찾을 수 없습니다.")?;
                    if discard {
                        result.push_str(raw);
                    } else {
                        crate::line_endings::push_line(&mut result, raw, ending);
                    }
                }
            }
        }
        if !keys.is_empty() || &result == if discard { &self.after } else { &self.before } {
            bail!("선택한 변경을 적용할 수 없습니다. 다시 조회하세요.");
        }
        Ok(result)
    }
}

fn validate_path(root: &Path, raw: &[u8]) -> Result<()> {
    if raw.is_empty() || raw.contains(&0) {
        bail!("잘못된 파일 경로입니다.");
    }
    let relative = gix::path::from_bstr(raw.as_bstr())?;
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            bail!("저장소 내부 파일만 Stage할 수 있습니다.");
        };
        if name.to_string_lossy().eq_ignore_ascii_case(".git") {
            bail!("Git 메타데이터는 Stage할 수 없습니다.");
        }
        path.push(name);
        if crate::repository::is_git_directory(&path) { bail!("Git 메타데이터는 Stage할 수 없습니다."); }
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

fn validate_preview(
    root: &Path,
    file: &FileChange,
    preview: &Preview,
) -> Result<(PathBuf, PathBuf)> {
    if !eligible(file) {
        bail!("일반 미스테이지 텍스트 파일만 부분 Stage할 수 있습니다.");
    }
    let repo = crate::diff_git::open(root)?;
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
    let path = gix::path::from_bstr(file.path.as_bstr())?;
    let mut attributes = repo.attributes_only(
        &index,
        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
    )?;
    let mut matches = attributes.selected_attribute_matches(["filter", "working-tree-encoding"]);
    attributes
        .at_path(path.as_ref(), None)?
        .matching_attributes(&mut matches);
    if matches
        .iter_selected()
        .any(|m| !m.assignment.state.is_unspecified() && !m.assignment.state.is_unset())
    {
        bail!("변환 필터가 있는 파일입니다. 파일 전체 Stage를 사용하세요.");
    }
    let before = crate::diff_git::read_content(&repo, file, &file.before)?;
    let after = crate::diff_git::read_content(&repo, file, &file.after)?;
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
    Ok((root.to_path_buf(), path.into_owned()))
}

pub fn apply(
    root: &Path,
    file: &FileChange,
    preview: &Preview,
    lines: &[DiffLine],
    selected: &[usize],
) -> Result<()> {
    let (root, _) = validate_preview(root, file, preview)?;
    let root = root.as_path();
    let selected = preview.selected_content(lines, selected)?;
    let removed = matches!(file.after, Content::Empty) && selected.is_empty();
    let mode = match file.before {
        Content::Object(_, mode) => u32::from(mode),
        _ => 0o100644,
    };
    crate::diff_git::set_index_content(
        root,
        &file.path,
        (!removed).then_some(selected.as_bytes()),
        mode,
        match file.before {
            Content::Object(id, mode) => Some((id, u32::from(mode))),
            _ => None,
        },
    )?;
    Ok(())
}

pub fn discard(
    root: &Path,
    file: &FileChange,
    preview: &Preview,
    lines: &[DiffLine],
    selected: &[usize],
) -> Result<()> {
    let (root, _) = validate_preview(root, file, preview)?;
    let replacement = preview.discarded_content(lines, selected)?;
    crate::diff_git::set_worktree_content(
        &root,
        file,
        preview.after.as_bytes(),
        replacement.as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn oracle_command(root: &Path) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        command.current_dir(root);
        command
    }

    fn git(root: &Path, args: &[&str]) -> Vec<u8> {
        let output = oracle_command(root).args(args).output().unwrap();
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

    fn preview(root: &Path, name: &str) -> (FileChange, crate::diff_model::Diff) {
        let file = crate::diff_git::load_stage(root)
            .unwrap()
            .into_iter()
            .find(|file| !file.staged && file.path == name.as_bytes())
            .unwrap();
        let diff = crate::diff_git::diff(root, &file).unwrap();
        assert!(diff.partial.is_some());
        (file, diff)
    }

    fn stage(root: &Path, file: &FileChange, diff: &crate::diff_model::Diff, rows: &[usize]) {
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
    fn cross_line_endings_partial_stage_and_discard_preserve_destination_bytes() {
        for (before, after, staged, discarded) in [
            (
                "one\r\ntwo\nthree\r\n",
                "ONE\nTWO\nthree\n",
                "ONE\r\ntwo\nthree\r\n",
                "ONE\ntwo\nthree\n",
            ),
            (
                "one\ntwo\nthree\n",
                "ONE\r\nTWO\r\nthree\r\n",
                "ONE\ntwo\nthree\n",
                "ONE\r\ntwo\r\nthree\r\n",
            ),
            (
                "one\r\ntwo\r\nthree\r\n",
                "ONE\nTWO\nthree\n",
                "ONE\r\ntwo\r\nthree\r\n",
                "ONE\ntwo\nthree\n",
            ),
            (
                "one\ntwo\nlast",
                "ONE\r\nTWO\r\nlast",
                "ONE\ntwo\nlast",
                "ONE\r\ntwo\r\nlast",
            ),
        ] {
            let dir = fixture("file.txt", before);
            let root = dir.path();
            fs::write(root.join("file.txt"), after).unwrap();
            let (file, diff) = preview(root, "file.txt");
            let rows = diff
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| {
                    line.old == Some(1) && line.kind == '-'
                        || line.new == Some(1) && line.kind == '+'
                })
                .map(|(row, _)| row)
                .collect::<Vec<_>>();
            stage(root, &file, &diff, &rows);
            assert_eq!(git(root, &["show", ":file.txt"]), staged.as_bytes());
            assert_eq!(fs::read(root.join("file.txt")).unwrap(), after.as_bytes());
            let (file, diff) = preview(root, "file.txt");
            let rows = diff
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| {
                    line.old == Some(2) && line.kind == '-'
                        || line.new == Some(2) && line.kind == '+'
                })
                .map(|(row, _)| row)
                .collect::<Vec<_>>();
            let index = fs::read(root.join(".git/index")).unwrap();
            discard(
                root,
                &file,
                diff.partial.as_ref().unwrap(),
                &diff.lines,
                &rows,
            )
            .unwrap();
            assert_eq!(
                fs::read(root.join("file.txt")).unwrap(),
                discarded.as_bytes()
            );
            assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        }
    }

    #[test]
    fn checked_line_discard_preserves_index_other_changes_and_other_files() {
        let original = (0..30)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        let dir = fixture("file.txt", &original);
        let root = dir.path();
        let indexed = original.replace("line 0\n", "already staged\n");
        fs::write(root.join("file.txt"), &indexed).unwrap();
        git(root, &["add", "file.txt"]);
        let after = indexed
            .replace("line 5\n", "first change\n")
            .replace("line 25\n", "second change\n");
        fs::write(root.join("file.txt"), &after).unwrap();
        fs::write(root.join("other.txt"), "preserve").unwrap();
        let (file, diff) = preview(root, "file.txt");
        let header = diff.lines.iter().position(|line| line.kind == '@').unwrap();
        let rows = hunk_rows(&diff.lines, header);
        let index = fs::read(root.join(".git/index")).unwrap();
        discard(
            root,
            &file,
            diff.partial.as_ref().unwrap(),
            &diff.lines,
            &rows,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            indexed.replace("line 25\n", "second change\n")
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
        assert_eq!(
            fs::read_to_string(root.join("other.txt")).unwrap(),
            "preserve"
        );
        assert!(!root.join(".git/index.lock").exists());
        assert!(!fs::read_dir(root).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".feather-discard-")
        }));
    }

    #[test]
    fn checked_line_discard_handles_new_deleted_crlf_and_eof() {
        let dir = fixture("한 글[1].txt", "first\r\nsecond\r\nlast");
        let root = dir.path();
        fs::write(root.join("new.txt"), "one\ntwo\n").unwrap();
        let (file, diff) = preview(root, "new.txt");
        let row = diff
            .lines
            .iter()
            .position(|line| line.kind == '+' && line.new == Some(1))
            .unwrap();
        discard(
            root,
            &file,
            diff.partial.as_ref().unwrap(),
            &diff.lines,
            &[row],
        )
        .unwrap();
        assert_eq!(fs::read_to_string(root.join("new.txt")).unwrap(), "two\n");
        let (file, diff) = preview(root, "new.txt");
        let rows = diff
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind == '+')
            .map(|(row, _)| row)
            .collect::<Vec<_>>();
        discard(
            root,
            &file,
            diff.partial.as_ref().unwrap(),
            &diff.lines,
            &rows,
        )
        .unwrap();
        assert!(!root.join("new.txt").exists());
        fs::remove_file(root.join("한 글[1].txt")).unwrap();
        let (file, diff) = preview(root, "한 글[1].txt");
        let row = diff
            .lines
            .iter()
            .position(|line| line.kind == '-' && line.old == Some(1))
            .unwrap();
        let index = fs::read(root.join(".git/index")).unwrap();
        discard(
            root,
            &file,
            diff.partial.as_ref().unwrap(),
            &diff.lines,
            &[row],
        )
        .unwrap();
        assert_eq!(fs::read(root.join("한 글[1].txt")).unwrap(), b"first\r\n");
        let (file, diff) = preview(root, "한 글[1].txt");
        let rows = diff
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind == '-')
            .map(|(row, _)| row)
            .collect::<Vec<_>>();
        discard(
            root,
            &file,
            diff.partial.as_ref().unwrap(),
            &diff.lines,
            &rows,
        )
        .unwrap();
        assert_eq!(
            fs::read(root.join("한 글[1].txt")).unwrap(),
            b"first\r\nsecond\r\nlast"
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    }

    #[test]
    fn checked_line_discard_rejects_stale_files_index_locks_filters_and_paths() {
        let dir = fixture("file.txt", "a\nb\n");
        let root = dir.path();
        fs::write(root.join("file.txt"), "a\nchanged\n").unwrap();
        let (file, diff) = preview(root, "file.txt");
        let rows = hunk_rows(
            &diff.lines,
            diff.lines.iter().position(|line| line.kind == '@').unwrap(),
        );
        let apply = |file: &FileChange| {
            discard(
                root,
                file,
                diff.partial.as_ref().unwrap(),
                &diff.lines,
                &rows,
            )
        };
        fs::write(root.join("file.txt"), "external\n").unwrap();
        assert!(apply(&file).is_err());
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "external\n"
        );
        fs::write(root.join("file.txt"), "a\nchanged\n").unwrap();
        git(root, &["add", "file.txt"]);
        assert!(apply(&file).is_err());
        git(root, &["reset", "HEAD", "--", "file.txt"]);
        fs::write(root.join(".git/index.lock"), "owner").unwrap();
        assert!(apply(&file).is_err());
        assert_eq!(
            fs::read_to_string(root.join(".git/index.lock")).unwrap(),
            "owner"
        );
        fs::remove_file(root.join(".git/index.lock")).unwrap();
        fs::write(root.join(".gitattributes"), "file.txt filter=test\n").unwrap();
        assert!(apply(&file).is_err());
        fs::remove_file(root.join(".gitattributes")).unwrap();
        let mut invalid = file.clone();
        invalid.path = b"../outside".to_vec();
        assert!(apply(&invalid).is_err());
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "a\nchanged\n"
        );
    }

    #[test]
    fn checked_line_discard_preserves_changes_missing_from_displayed_rows() {
        let preview = Preview {
            before: (0..6000).map(|index| format!("old {index}\n")).collect(),
            after: (0..6000).map(|index| format!("new {index}\n")).collect(),
        };
        let lines = vec![
            DiffLine {
                kind: '-',
                old: Some(1),
                new: None,
                text: "old 0".into(),
            },
            DiffLine {
                kind: '+',
                old: None,
                new: Some(1),
                text: "new 0".into(),
            },
        ];
        let restored = preview.discarded_content(&lines, &[0, 1]).unwrap();
        assert_eq!(restored, preview.after.replacen("new 0\n", "old 0\n", 1));
        assert!(restored.ends_with("new 5999\n"));
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
