use std::path::Path;

/// 파일 작업에 쓰는 원본 경로는 유지하고 화면에 표시할 문자열만 만든다.
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let Some(rest) = text.strip_prefix(r"\\?\") else {
        return text.into_owned();
    };
    if rest
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(r"UNC\"))
    {
        let network = &rest[4..];
        if !network.is_empty() {
            return format!(r"\\{network}");
        }
    }
    let bytes = rest.as_bytes();
    if bytes
        .first()
        .is_some_and(|letter| letter.is_ascii_alphabetic())
        && bytes.get(1) == Some(&b':')
        && bytes.get(2) == Some(&b'\\')
    {
        return rest.to_owned();
    }
    text.into_owned()
}

/// 존재하지 않는 파일도 복사할 수 있도록 파일시스템 조회 없이 결합합니다.
pub fn absolute_file_path(root: &Path, raw: &[u8]) -> Result<std::path::PathBuf, String> {
    use gix::bstr::ByteSlice;
    if !root.is_absolute() {
        return Err("저장소 경로가 절대경로가 아닙니다.".into());
    }
    let relative = gix::path::from_bstr(raw.as_bstr()).map_err(|error| error.to_string())?;
    let components: Vec<_> = relative.components().collect();
    if components.is_empty()
        || components
            .iter()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("올바른 저장소 파일 경로가 아닙니다.".into());
    }
    Ok(root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn absolute_file_paths_keep_literal_unicode_paths_and_reject_escape() {
        let root = std::env::current_dir().unwrap();
        assert_eq!(
            absolute_file_path(&root, "폴더/한 글[1].txt".as_bytes()).unwrap(),
            root.join("폴더/한 글[1].txt")
        );
        assert!(absolute_file_path(Path::new("relative"), b"file.txt").is_err());
        for raw in [b"../outside".as_slice(), b"/absolute", b"", b"dir/../file"] {
            assert!(absolute_file_path(&root, raw).is_err());
        }
    }

    #[test]
    fn verbatim_drive_paths_display_without_prefix() {
        for (source, expected) in [
            (r"\\?\C:\repo", r"C:\repo"),
            (r"\\?\d:\Projects\한글 저장소", r"d:\Projects\한글 저장소"),
            (r"\\?\Z:\", r"Z:\"),
        ] {
            assert_eq!(display_path(Path::new(source)), expected);
        }
    }

    #[test]
    fn verbatim_unc_paths_restore_the_network_prefix() {
        for (source, expected) in [
            (r"\\?\UNC\server\share\repo", r"\\server\share\repo"),
            (r"\\?\unc\서버\공유 폴더\repo", r"\\서버\공유 폴더\repo"),
        ] {
            assert_eq!(display_path(Path::new(source)), expected);
        }
    }

    #[test]
    fn normal_incomplete_and_device_paths_are_preserved() {
        for source in [
            r"C:\repo",
            r"\\server\share\repo",
            r"relative\repo",
            "/tmp/한글 repo",
            "",
            r"\\?",
            r"\\?\",
            r"\\?\C:",
            r"\\?\UNC\",
            r"\\?\Volume{guid}\repo",
            r"\\?\GLOBALROOT\Device\disk",
            r"\\.\COM1",
        ] {
            assert_eq!(display_path(Path::new(source)), source);
        }
    }

    #[test]
    fn display_does_not_mutate_original_path() {
        let source = PathBuf::from(r"\\?\C:\한글 저장소");
        let before = source.clone();
        let _ = display_path(&source);
        assert_eq!(source, before);
    }
}
