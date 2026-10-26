use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

pub fn create(path: &Path, source: Option<&str>) -> Result<()> {
    if source.is_some_and(|url| url.trim().is_empty() || url.starts_with('-')) {
        bail!("유효한 저장소 URL을 입력하세요.");
    }
    std::fs::create_dir(path)
        .context("새 대상 폴더를 만들 수 없습니다. 기존 경로는 사용할 수 없습니다.")?;
    let mut command = Command::new("git");
    command.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    if let Some(url) = source {
        command.args(["clone", "--", url]).arg(path);
    } else {
        command.args(["init", "--"]).arg(path);
    }
    let output = command
        .output()
        .context("Git을 실행할 수 없습니다. Git 설치와 PATH를 확인하세요.")?;
    if !output.status.success() {
        bail!("Git 작업 실패: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_clone_and_protect_existing_paths() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        create(&source, None).unwrap();
        assert!(source.join(".git").is_dir());
        std::fs::write(source.join("tracked.txt"), "clone content").unwrap();
        for args in [
            vec!["add", "tracked.txt"],
            vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "initial",
            ],
        ] {
            assert!(
                Command::new("git")
                    .current_dir(&source)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        let target = temp.path().join("clone");
        create(&target, Some(source.to_str().unwrap())).unwrap();
        assert!(target.join(".git").is_dir());
        assert_eq!(
            std::fs::read_to_string(target.join("tracked.txt")).unwrap(),
            "clone content"
        );
        std::fs::write(target.join("keep"), "preserved").unwrap();
        assert!(create(&target, None).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("keep")).unwrap(),
            "preserved"
        );
        let invalid = temp.path().join("invalid");
        assert!(create(&invalid, Some("")).is_err());
        assert!(!invalid.exists());
        assert!(
            create(
                &invalid,
                Some(temp.path().join("missing").to_str().unwrap())
            )
            .is_err()
        );
    }
}
