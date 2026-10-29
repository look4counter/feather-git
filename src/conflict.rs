use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{fs, io::Read, ops::Range, path::Path};

#[derive(Clone, Copy, Debug)]
pub enum Choice {
    Current,
    Incoming,
    Both,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub line: usize,
    pub end_line: usize,
    whole: Range<usize>,
    current: Range<usize>,
    incoming: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct Preview {
    pub source: String,
    pub regions: Vec<Region>,
}

fn marker(line: &str) -> Option<(u8, usize)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let first = *line.as_bytes().first()?;
    if !b"<|=>".contains(&first) {
        return None;
    }
    let width = line.bytes().take_while(|b| *b == first).count();
    (width >= 7 && (width == line.len() || line.as_bytes()[width] == b' '))
        .then_some((first, width))
}

pub fn parse(source: &str) -> Result<Preview> {
    if source.len() as u64 > crate::git::MAX_BYTES || source.contains('\0') {
        bail!("선택할 수 없는 텍스트 파일입니다.");
    }
    let mut regions = Vec::new();
    let mut active: Option<(usize, usize, usize, usize)> = None;
    let mut base = None;
    let mut separator = None;
    let mut offset = 0;
    for (index, line) in source.split_inclusive('\n').enumerate() {
        if let Some((kind, width)) = marker(line) {
            match (kind, active) {
                (b'<', None) => {
                    active = Some((offset, offset + line.len(), index + 1, width));
                    base = None;
                    separator = None;
                }
                (b'|', Some((_, _, _, expected)))
                    if width == expected && base.is_none() && separator.is_none() =>
                {
                    base = Some(offset);
                }
                (b'=', Some((_, _, _, expected))) if width == expected && separator.is_none() => {
                    separator = Some((offset, offset + line.len()));
                }
                (b'>', Some((start, content, line_number, expected)))
                    if width == expected && separator.is_some() =>
                {
                    let (middle, incoming) = separator.unwrap();
                    regions.push(Region {
                        line: line_number,
                        end_line: index + 1,
                        whole: start..offset + line.len(),
                        current: content..base.unwrap_or(middle),
                        incoming: incoming..offset,
                    });
                    active = None;
                }
                _ => bail!("불완전하거나 중첩된 충돌 마커입니다. 외부 편집기로 확인하세요."),
            }
        }
        offset += line.len();
    }
    if active.is_some() {
        bail!("끝나지 않은 충돌 구간입니다. 외부 편집기로 확인하세요.");
    }
    Ok(Preview {
        source: source.into(),
        regions,
    })
}

impl Preview {
    pub fn region_for_line(&self, line: usize) -> Option<usize> {
        self.regions
            .iter()
            .position(|region| (region.line..=region.end_line).contains(&line))
    }

    pub fn resolve(&self, index: usize, choice: Choice) -> Result<String> {
        let region = self
            .regions
            .get(index)
            .context("충돌 구간을 다시 조회하세요.")?;
        let mut result = self.source[..region.whole.start].to_owned();
        if matches!(choice, Choice::Current | Choice::Both) {
            result.push_str(&self.source[region.current.clone()]);
        }
        if matches!(choice, Choice::Incoming | Choice::Both) {
            result.push_str(&self.source[region.incoming.clone()]);
        }
        result.push_str(&self.source[region.whole.end..]);
        Ok(result)
    }
}

fn checked_file(root: &Path, raw: &[u8]) -> Result<std::path::PathBuf> {
    let relative = gix::path::from_bstr(raw.as_bstr());
    let mut path = root.to_path_buf();
    if relative.as_os_str().is_empty() {
        bail!("빈 파일 경로입니다.");
    }
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            bail!("잘못된 파일 경로입니다.");
        }
        path.push(component);
        let metadata = fs::symlink_metadata(&path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                bail!("링크 경로는 수정할 수 없습니다.");
            }
        }
        if metadata.file_type().is_symlink() {
            bail!("링크 경로는 수정할 수 없습니다.");
        }
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.len() > crate::git::MAX_BYTES {
        bail!("1 MiB 이하의 일반 파일만 수정할 수 있습니다.");
    }
    Ok(path)
}

pub fn apply(
    root: &Path,
    raw: &[u8],
    preview: &Preview,
    index: usize,
    choice: Choice,
) -> Result<()> {
    let repo = crate::git::open(root)?;
    let root = repo.workdir().context("작업 디렉터리가 없습니다.")?;
    let entries = repo.index()?;
    if !entries.entries().iter().any(|entry| {
        entry.path(&entries).as_bytes() == raw
            && entry.stage() != gix::index::entry::Stage::Unconflicted
    }) {
        bail!("미병합 파일이 아닙니다. 다시 조회하세요.");
    }
    let path = checked_file(root, raw)?;
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(crate::git::MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes != preview.source.as_bytes() {
        bail!("파일 내용이 변경되었습니다. 다시 조회한 뒤 선택하세요.");
    }
    // 공개 미리보기 값도 다시 파싱하여 임의 범위 치환을 막는다.
    let current = parse(std::str::from_utf8(&bytes)?)?;
    let resolved = current.resolve(index, choice)?;
    fs::write(path, resolved)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_regions_and_all_choices_preserve_surrounding_bytes() {
        let text = "앞\r\n<<<<<<< HEAD\r\n우리\r\n||||||| base\r\n조상\r\n=======\r\n상대\r\n>>>>>>> other\r\n중간\r\n<<<<<<< HEAD\r\n=======\r\n둘\r\n>>>>>>> other\r\n끝";
        let p = parse(text).unwrap();
        assert_eq!(p.regions.len(), 2);
        for (choice, expected) in [
            (Choice::Current, "우리\r\n"),
            (Choice::Incoming, "상대\r\n"),
            (Choice::Both, "우리\r\n상대\r\n"),
        ] {
            let result = p.resolve(0, choice).unwrap();
            assert!(result.starts_with(&format!("앞\r\n{expected}중간\r\n")));
            assert!(result.ends_with("둘\r\n>>>>>>> other\r\n끝"));
            let next = parse(&result).unwrap();
            assert_eq!(next.regions.len(), 1);
            assert_eq!(
                next.resolve(0, Choice::Current).unwrap(),
                format!("앞\r\n{expected}중간\r\n끝")
            );
        }
    }

    #[test]
    fn malformed_markers_fail_closed_and_custom_width_is_supported() {
        for text in [
            "<<<<<<< a\nx\n",
            "=======\n",
            "<<<<<<< a\n<<<<<<< b\n=======\n>>>>>>> b\n>>>>>>> a\n",
            "<<<<<<< a\n=======\n||||||| b\n>>>>>>> b\n",
            "<<<<<<< a\n========\n>>>>>>> b\n",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        let p = parse("<<<<<<<< a\na\n========\nb\n>>>>>>>> b").unwrap();
        assert_eq!(p.resolve(0, Choice::Both).unwrap(), "a\nb\n");
        assert!(parse("plain\ntext").unwrap().regions.is_empty());
    }

    fn git(root: &Path, args: &[&str]) -> std::process::Output {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "core.autocrlf=false",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap()
    }

    #[test]
    fn real_conflict_selection_refreshes_preview_and_preserves_index() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        assert!(git(root, &["init", "-b", "main"]).status.success());
        fs::write(root.join("file.txt"), "base\n").unwrap();
        assert!(git(root, &["add", "."]).status.success());
        assert!(git(root, &["commit", "-m", "base"]).status.success());
        assert!(git(root, &["checkout", "-b", "incoming"]).status.success());
        fs::write(root.join("file.txt"), "incoming\n").unwrap();
        assert!(git(root, &["commit", "-am", "incoming"]).status.success());
        assert!(git(root, &["checkout", "main"]).status.success());
        fs::write(root.join("file.txt"), "current\n").unwrap();
        assert!(git(root, &["commit", "-am", "current"]).status.success());
        assert!(!git(root, &["merge", "incoming"]).status.success());
        let raw_index = fs::read(root.join(".git/index")).unwrap();
        let original = fs::read_to_string(root.join("file.txt")).unwrap();
        let file = crate::git::load_stage(root)
            .unwrap()
            .into_iter()
            .find(|f| f.status == "U")
            .unwrap();
        let diff = crate::git::diff(root, &file).unwrap();
        let marker_row = diff
            .lines
            .iter()
            .position(|line| line.text == "<<<<<<< HEAD")
            .unwrap();
        let marker = &diff.lines[marker_row];
        assert_eq!(marker.text, "<<<<<<< HEAD");
        assert_eq!(marker.new, Some(1));
        assert_eq!(diff.conflict_at_row(marker_row), Some(0));
        assert_eq!(diff.conflict_at_row(0), None);
        assert!(diff.lines.iter().all(|line| line.kind != '!'));
        assert!(diff.partial.is_none());
        let preview = diff.conflict.unwrap();
        for (choice, expected) in [
            (Choice::Current, "current\n"),
            (Choice::Incoming, "incoming\n"),
            (Choice::Both, "current\nincoming\n"),
        ] {
            fs::write(root.join("file.txt"), &original).unwrap();
            apply(root, b"file.txt", &preview, 0, choice).unwrap();
            assert_eq!(fs::read_to_string(root.join("file.txt")).unwrap(), expected);
            assert_eq!(fs::read(root.join(".git/index")).unwrap(), raw_index);
            let refreshed = crate::git::diff(root, &file).unwrap();
            assert!(refreshed.conflict.is_none());
            assert!(refreshed.note.contains("Stage"));
        }
        let two = "before\n<<<<<<< HEAD\nfirst current\n=======\nfirst incoming\n>>>>>>> incoming\nbetween\n<<<<<<< HEAD\nsecond current\n=======\nsecond incoming\n>>>>>>> incoming\nafter";
        fs::write(root.join("file.txt"), two).unwrap();
        let first_diff = crate::git::diff(root, &file).unwrap();
        let target_row = first_diff
            .lines
            .iter()
            .position(|line| line.text == "second incoming")
            .unwrap();
        let target_region = first_diff.conflict_at_row(target_row).unwrap();
        assert_eq!(target_region, 1);
        let first = first_diff.conflict.unwrap();
        apply(root, b"file.txt", &first, target_region, Choice::Incoming).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            two.replace(
                "<<<<<<< HEAD\nsecond current\n=======\nsecond incoming\n>>>>>>> incoming\n",
                "second incoming\n"
            )
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), raw_index);
        fs::write(root.join("file.txt"), two).unwrap();
        assert_eq!(first.regions.len(), 2);
        apply(root, b"file.txt", &first, 0, Choice::Current).unwrap();
        let second = crate::git::diff(root, &file).unwrap().conflict.unwrap();
        assert_eq!(second.regions.len(), 1);
        assert!(apply(root, b"file.txt", &first, 1, Choice::Both).is_err());
        apply(root, b"file.txt", &second, 0, Choice::Both).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("file.txt")).unwrap(),
            "before\nfirst current\nbetween\nsecond current\nsecond incoming\nafter"
        );
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), raw_index);
        for unsupported in [
            b"binary\0".as_slice(),
            b"invalid\xff".as_slice(),
            b"<<<<<<< unfinished\n".as_slice(),
        ] {
            fs::write(root.join("file.txt"), unsupported).unwrap();
            assert!(crate::git::diff(root, &file).unwrap().conflict.is_none());
            assert!(apply(root, b"file.txt", &preview, 0, Choice::Current).is_err());
            assert_eq!(fs::read(root.join("file.txt")).unwrap(), unsupported);
        }
        fs::write(root.join("file.txt"), "external edit\n").unwrap();
        assert!(apply(root, b"file.txt", &preview, 0, Choice::Current).is_err());
        assert_eq!(fs::read(root.join("file.txt")).unwrap(), b"external edit\n");
        assert_eq!(fs::read(root.join(".git/index")).unwrap(), raw_index);
        assert!(apply(root, b"../file.txt", &preview, 0, Choice::Current).is_err());
        fs::write(root.join("file.txt"), &original).unwrap();
        assert!(git(root, &["add", "file.txt"]).status.success());
        assert!(apply(root, b"file.txt", &preview, 0, Choice::Current).is_err());
        assert_eq!(fs::read_to_string(root.join("file.txt")).unwrap(), original);
        let staged = crate::git::load_stage(root)
            .unwrap()
            .into_iter()
            .find(|f| f.staged)
            .unwrap();
        assert!(crate::git::diff(root, &staged).unwrap().conflict.is_none());
    }

    #[test]
    fn cursor_regions_include_all_markers_and_exclude_headers_deletions_and_surroundings() {
        use crate::model::{Diff, DiffLine};
        for newline in ["\n", "\r\n"] {
            let source = [
                "before",
                "<<<<<<< HEAD",
                "current",
                "||||||| base",
                "base",
                "=======",
                "incoming",
                ">>>>>>> other",
                "between",
                "<<<<<<< HEAD",
                "=======",
                "second",
                ">>>>>>> other",
                "after",
            ]
            .join(newline);
            let preview = parse(&source).unwrap();
            assert_eq!(
                (preview.regions[0].line, preview.regions[0].end_line),
                (2, 8)
            );
            assert_eq!(
                (preview.regions[1].line, preview.regions[1].end_line),
                (10, 13)
            );
            let mut diff = Diff {
                conflict: Some(preview),
                ..Diff::default()
            };
            diff.lines.push(DiffLine {
                old: None,
                new: None,
                kind: '@',
                text: "@@".into(),
            });
            for (index, text) in source.lines().enumerate() {
                diff.lines.push(DiffLine {
                    old: None,
                    new: Some(index + 1),
                    kind: '+',
                    text: text.into(),
                });
            }
            diff.lines.push(DiffLine {
                old: Some(4),
                new: None,
                kind: '-',
                text: "deleted base".into(),
            });
            for row in 0..diff.lines.len() {
                let expected = match diff.lines[row].new {
                    Some(2..=8) => Some(0),
                    Some(10..=13) => Some(1),
                    _ => None,
                };
                assert_eq!(diff.conflict_at_row(row), expected, "row {row}");
            }
            assert_eq!(diff.conflict_at_row(usize::MAX), None);
            diff.conflict = None;
            assert_eq!(diff.conflict_at_row(3), None);
        }
    }
}
