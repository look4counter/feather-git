use anyhow::{Context, Result, bail};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const LINUX_TERMINALS: &[&str] = &[
    "x-terminal-emulator",
    "gnome-terminal",
    "konsole",
    "xfce4-terminal",
    "xterm",
];

const BASH_START: &str = "cd -- \"$1\" && exec \"$2\" -i";
const MAC_SCRIPT: &str = r#"on run argv
    set projectPath to item 1 of argv
    set bashPath to item 2 of argv
    tell application "Terminal"
        do script ("cd -- " & quoted form of projectPath & " && exec " & quoted form of bashPath & " -i")
        activate
    end tell
end run"#;

fn executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    true
}

fn search_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter(|p| p.is_absolute())
                .collect()
        })
        .unwrap_or_default()
}

fn find_in(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|path| executable(path))
}

fn git_bash_candidates(dirs: &[PathBuf], roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in dirs {
        for ancestor in dir.ancestors().take(3) {
            candidates.push(ancestor.join("git-bash.exe"));
        }
    }
    candidates.extend(roots.iter().map(|root| root.join("git-bash.exe")));
    candidates
}

fn windows_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for name in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Some(path) = std::env::var_os(name) {
            roots.push(PathBuf::from(path).join("Git"));
        }
    }
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(path).join("Programs").join("Git"));
    }
    roots
}

fn build_command(os: &str, launcher: &Path, bash: &Path, project: &Path) -> Result<Command> {
    let mut command = Command::new(launcher);
    command.current_dir(project);
    match os {
        "windows" => {
            let mut directory = OsString::from("--cd=");
            directory.push(project);
            command.arg(directory);
        }
        "linux" => {
            let name = launcher.file_name().and_then(|name| name.to_str());
            match name {
                Some("gnome-terminal") => {
                    command.arg("--");
                }
                Some("xfce4-terminal") => {
                    command.arg("--disable-server").arg("-x");
                }
                Some("x-terminal-emulator" | "konsole" | "xterm") => {
                    command.arg("-e");
                }
                _ => bail!("지원하지 않는 터미널입니다."),
            }
            // 프로젝트 경로는 스크립트에 삽입하지 않고 위치 인수로만 전달한다.
            command
                .arg(bash)
                .args(["--noprofile", "--norc", "-c", BASH_START, "bash"])
                .arg(project)
                .arg(bash);
        }
        "macos" => {
            command
                .arg("-e")
                .arg(MAC_SCRIPT)
                .arg("--")
                .arg(project)
                .arg(bash);
        }
        _ => bail!("이 운영체제에서는 Bash 터미널 열기를 지원하지 않습니다."),
    }
    Ok(command)
}

pub fn open(project: &Path) -> Result<()> {
    anyhow::ensure!(project.is_dir(), "프로젝트 폴더가 존재하지 않습니다.");
    let mut dirs = search_dirs();
    let os = std::env::consts::OS;
    let mut command = match os {
        "windows" => {
            let launcher = git_bash_candidates(&dirs, &windows_roots()).into_iter()
                .find(|path| executable(path))
                .context("Git Bash를 찾지 못했습니다. Git for Windows를 설치하거나 설치 경로를 PATH에 추가하세요.")?;
            build_command(os, &launcher, Path::new(""), project)?
        }
        "linux" | "macos" => {
            dirs.extend([PathBuf::from("/bin"), PathBuf::from("/usr/bin")]);
            let bash = find_in("bash", &dirs)
                .context("Bash를 찾지 못했습니다. Bash를 설치하고 PATH를 확인하세요.")?;
            let launcher = if os == "macos" {
                PathBuf::from("/usr/bin/osascript")
            } else {
                LINUX_TERMINALS.iter().find_map(|name| find_in(name, &dirs))
                    .context("터미널을 찾지 못했습니다. GNOME Terminal, Konsole, Xfce Terminal 또는 xterm을 설치하세요.")?
            };
            build_command(os, &launcher, &bash, project)?
        }
        _ => bail!("이 운영체제에서는 Bash 터미널 열기를 지원하지 않습니다."),
    };
    // 사용자가 요청한 대화식 터미널 창을 표시한다. Windows 보조 콘솔은 숨긴다.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Bash 터미널을 실행하지 못했습니다.")?;
    anyhow::ensure!(
        status.success(),
        "Bash 터미널 실행에 실패했습니다. 종료 상태: {status}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_commands_preserve_special_paths_as_data() {
        let project = Path::new("/projects/한글 a'\";$(touch nope)\\folder");
        let bash = Path::new("/custom path/bash");
        let command = build_command("windows", Path::new("git-bash.exe"), bash, project).unwrap();
        let mut expected = OsString::from("--cd=");
        expected.push(project);
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![expected.as_os_str()]
        );
        assert_eq!(command.get_current_dir(), Some(project));
        for launcher in LINUX_TERMINALS {
            let command = build_command("linux", Path::new(launcher), bash, project).unwrap();
            let args = command.get_args().collect::<Vec<_>>();
            assert_eq!(args[args.len() - 2], project.as_os_str());
            assert_eq!(args[args.len() - 1], bash.as_os_str());
            assert!(args.contains(&std::ffi::OsStr::new(BASH_START)));
            assert_eq!(command.get_current_dir(), Some(project));
        }
        let command =
            build_command("macos", Path::new("/usr/bin/osascript"), bash, project).unwrap();
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![
                std::ffi::OsStr::new("-e"),
                std::ffi::OsStr::new(MAC_SCRIPT),
                std::ffi::OsStr::new("--"),
                project.as_os_str(),
                bash.as_os_str(),
            ]
        );
        assert!(!MAC_SCRIPT.contains("touch nope"));
        assert!(build_command("other", Path::new("terminal"), bash, project).is_err());
    }

    #[test]
    fn detection_handles_git_cmd_usr_bin_and_missing_installations() {
        let candidates = git_bash_candidates(
            &[
                PathBuf::from("/custom/Git/cmd"),
                PathBuf::from("/custom/Git/usr/bin"),
            ],
            &[PathBuf::from("/default/Git")],
        );
        assert!(candidates.contains(&PathBuf::from("/custom/Git/git-bash.exe")));
        assert!(candidates.contains(&PathBuf::from("/default/Git/git-bash.exe")));
        assert!(find_in("bash", &[]).is_none());
        assert!(
            find_in(
                "nonexistent-terminal-executable",
                &[PathBuf::from("/nonexistent")]
            )
            .is_none()
        );
    }
}
