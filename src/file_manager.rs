use std::{path::Path, process::Command};

fn program(os: &str) -> anyhow::Result<&'static str> {
    match os {
        "windows" => Ok("explorer.exe"),
        "linux" => Ok("xdg-open"),
        "macos" => Ok("open"),
        _ => anyhow::bail!("이 운영체제에서는 파일 관리자 열기를 지원하지 않습니다."),
    }
}

fn command(os: &str, path: &Path) -> anyhow::Result<Command> {
    let mut command = Command::new(program(os)?);
    command.arg(path);
    Ok(command)
}

pub fn open(path: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(path.is_dir(), "프로젝트 폴더가 존재하지 않습니다.");
    let mut command = command(std::env::consts::OS, path)?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output()?;
    // Explorer는 기존 창에 요청을 전달한 경우에도 비정상 종료 코드를 반환할 수 있다.
    #[cfg(not(target_os = "windows"))]
    anyhow::ensure!(
        output.status.success(),
        "파일 관리자를 열지 못했습니다. 종료 상태: {}",
        output.status
    );
    #[cfg(target_os = "windows")]
    let _ = output;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_commands_keep_project_path_as_one_literal_argument() {
        let path = Path::new("/projects/a b;$(test)/저장소");
        for (os, expected) in [
            ("windows", "explorer.exe"),
            ("linux", "xdg-open"),
            ("macos", "open"),
        ] {
            let command = command(os, path).unwrap();
            assert_eq!(command.get_program(), expected);
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                vec![path.as_os_str()]
            );
        }
        assert!(command("unsupported", path).is_err());
    }
}
