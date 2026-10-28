use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

fn git_command(path: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(path).env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

#[derive(Clone, Copy)]
pub enum RemoteOperation {
    Fetch,
    Pull,
    Push,
}

impl RemoteOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fetch => "패치 중…",
            Self::Pull => "풀 중…",
            Self::Push => "푸시 중…",
        }
    }
    pub fn run(self, path: &Path) -> Result<()> {
        let remotes = git_command(path)
            .arg("remote")
            .output()
            .context("Git을 실행할 수 없습니다.")?;
        if !remotes.status.success() {
            bail!(
                "원격 저장소를 확인하지 못했습니다: {}",
                String::from_utf8_lossy(&remotes.stderr)
            );
        }
        if remotes.stdout.iter().all(u8::is_ascii_whitespace) {
            bail!("등록된 원격 저장소가 없습니다.");
        }
        let args: &[&str] = match self {
            Self::Fetch => &["fetch", "--all"],
            Self::Pull => &["pull", "--ff-only"],
            Self::Push => &["push"],
        };
        let output = git_command(path)
            .args(args)
            .output()
            .context("Git fetch를 실행하지 못했습니다.")?;
        if !output.status.success() {
            bail!(
                "원격 작업 실패: {}{}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
pub fn fetch(path: &Path) -> Result<()> {
    RemoteOperation::Fetch.run(path)
}

pub fn stash_all(repository: &Path, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        bail!("스태시 이름을 한 줄로 입력하세요.");
    }
    let root = feather_git::git::repository_root(repository)?;
    let status = git_command(&root)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .context("변경 사항을 확인하지 못했습니다.")?;
    if !status.status.success() {
        bail!(
            "변경 사항 확인 실패: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    if status.stdout.is_empty() {
        bail!("스태시에 저장할 변경 사항이 없습니다.");
    }
    let output = git_command(&root)
        .args(["stash", "push", "--include-untracked", "--message", name])
        .output()
        .context("스태시를 실행하지 못했습니다.")?;
    if !output.status.success() {
        bail!(
            "스태시 저장 실패: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

pub fn delete_stash(repository: &Path, selector: &str, expected: &str) -> Result<()> {
    let index = selector
        .strip_prefix("stash@{")
        .and_then(|s| s.strip_suffix('}'));
    if !index.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        || ![40, 64].contains(&expected.len())
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("올바른 스태시를 선택하세요.");
    }
    let root = feather_git::git::repository_root(repository)?;
    let current = git_command(&root)
        .args(["rev-parse", "--verify", selector])
        .output()?;
    if !current.status.success()
        || !String::from_utf8_lossy(&current.stdout)
            .trim()
            .eq_ignore_ascii_case(expected)
    {
        bail!("스태시 목록이 변경되었습니다. 새로고침 후 다시 선택하세요.");
    }
    let output = git_command(&root)
        .args(["stash", "drop", selector])
        .output()
        .context("스태시 삭제를 실행하지 못했습니다.")?;
    if !output.status.success() {
        bail!(
            "스태시 삭제 실패: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

pub fn restore_stash(repository: &Path, commit: &str) -> Result<()> {
    if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("올바른 스태시를 선택하세요.");
    }
    let root = feather_git::git::repository_root(repository)?;
    let entries = git_command(&root)
        .args(["stash", "list", "--format=%H"])
        .output()?;
    if !entries.status.success() {
        bail!(
            "스태시 목록 조회 실패: {}",
            String::from_utf8_lossy(&entries.stderr)
        );
    }
    if !String::from_utf8_lossy(&entries.stdout)
        .lines()
        .any(|entry| entry.eq_ignore_ascii_case(commit))
    {
        bail!("선택한 스태시가 더 이상 목록에 없습니다.");
    }
    let output = git_command(&root)
        .args(["stash", "apply", "--index", commit])
        .output()
        .context("스태시 복원을 실행하지 못했습니다.")?;
    if !output.status.success() {
        bail!(
            "스태시 복원 실패. 원본 스태시는 유지됩니다: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

pub fn commit_then_push(repository: &Path, message: &str, push: bool) -> Result<Option<String>> {
    commit(repository, message)?;
    Ok(if push {
        RemoteOperation::Push
            .run(repository)
            .err()
            .map(|error| format!("커밋은 완료되었지만 푸시에 실패했습니다: {error:#}"))
    } else {
        None
    })
}

#[cfg(test)]
pub fn stage_file(repository: &Path, path: &[u8]) -> Result<()> {
    update_index(repository, path, true)
}

#[cfg(test)]
pub fn unstage_file(repository: &Path, path: &[u8]) -> Result<()> {
    update_index(repository, path, false)
}

pub fn commit(repository: &Path, message: &str) -> Result<()> {
    use std::{io::Write, process::Stdio};
    if message.trim().is_empty() || message.contains('\0') {
        bail!("커밋 메시지를 입력하세요.");
    }
    let root = feather_git::git::repository_root(repository)?;
    let staged = git_command(&root)
        .args(["diff", "--cached", "--quiet", "--no-ext-diff"])
        .output()?;
    match staged.status.code() {
        Some(0) => bail!("staged 변경사항이 없습니다."),
        Some(1) => {}
        _ => bail!(
            "staged 변경사항을 확인하지 못했습니다: {}",
            String::from_utf8_lossy(&staged.stderr)
        ),
    }
    let mut child = git_command(&root)
        .args(["commit", "--file=-", "--cleanup=verbatim"])
        .env("GIT_EDITOR", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Git 커밋을 실행하지 못했습니다.")?;
    let written = child
        .stdin
        .take()
        .context("커밋 메시지 입력을 열지 못했습니다.")?
        .write_all(message.as_bytes());
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(
            "커밋 실패: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    written.context("커밋 메시지를 전달하지 못했습니다.")?;
    Ok(())
}

#[cfg(test)]
fn update_index(repository: &Path, path: &[u8], staged: bool) -> Result<()> {
    update_index_many(repository, &[path.to_vec()], staged)
}

fn checked_paths(paths: &[Vec<u8>]) -> Result<Vec<std::path::PathBuf>> {
    use gix::bstr::ByteSlice;
    use std::path::Component;
    let mut result = Vec::new();
    for path in paths {
        let relative = gix::path::from_bstr(path.as_bstr());
        if path.is_empty()
            || path.contains(&0)
            || relative.components().any(|component| match component {
                Component::Normal(name) => name.to_string_lossy().eq_ignore_ascii_case(".git"),
                _ => true,
            })
        {
            bail!("저장소 내부의 파일 경로만 인덱스에서 변경할 수 있습니다.");
        }
        result.push(relative.into_owned());
    }
    Ok(result)
}

fn update_index_many(repository: &Path, paths: &[Vec<u8>], staged: bool) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let paths = checked_paths(paths)?;
    let root = feather_git::git::repository_root(repository)?;
    let mut command = git_command(&root);
    command.arg("--literal-pathspecs");
    if staged {
        command.args(["add", "--all", "--"]);
    } else if gix::discover(&root)?.head()?.is_unborn() {
        command.args(["rm", "--cached", "--force", "--ignore-unmatch", "--"]);
    } else {
        command.args(["reset", "--quiet", "HEAD", "--"]);
    }
    let output = command
        .args(paths)
        .output()
        .context("Git 인덱스 변경을 실행하지 못했습니다.")?;
    if !output.status.success() {
        bail!(
            "인덱스 변경 실패: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn toggle_files(
    repository: &Path,
    changes: &std::collections::BTreeMap<Vec<u8>, bool>,
) -> Result<()> {
    checked_paths(&changes.keys().cloned().collect::<Vec<_>>())?;
    let stage = changes
        .iter()
        .filter(|(_, staged)| **staged)
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    let unstage = changes
        .iter()
        .filter(|(_, staged)| !**staged)
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    update_index_many(repository, &stage, true)?;
    update_index_many(repository, &unstage, false)
}

pub fn discard_files(repository: &Path, paths: &[Vec<u8>]) -> Result<()> {
    let paths: Vec<_> = paths
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let relative = checked_paths(&paths)?;
    if paths.is_empty() {
        return Ok(());
    }
    let root = feather_git::git::repository_root(repository)?;
    let mut tracked = Vec::new();
    let mut added = Vec::new();
    {
        let repo = gix::discover(&root)?;
        let head = if repo.head()?.is_unborn() {
            None
        } else {
            Some(repo.head_commit()?.tree()?)
        };
        for (path, relative) in paths.iter().zip(&relative) {
            let mut ancestor = root.clone();
            let count = relative.components().count();
            for (index, component) in relative.components().enumerate() {
                ancestor.push(component.as_os_str());
                match std::fs::symlink_metadata(&ancestor) {
                    Ok(metadata) => {
                        let mut linked = metadata.file_type().is_symlink();
                        #[cfg(windows)]
                        {
                            use std::os::windows::fs::MetadataExt;
                            linked |= metadata.file_attributes() & 0x400 != 0;
                        }
                        if index + 1 < count && linked {
                            bail!("연결된 상위 폴더의 파일은 폐기할 수 없습니다.");
                        }
                        if index + 1 == count && metadata.is_dir() {
                            bail!("디렉터리나 서브모듈은 파일 폐기로 처리할 수 없습니다.");
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let entry = head
                .as_ref()
                .map(|tree| tree.lookup_entry(path.split(|byte| *byte == b'/')))
                .transpose()?
                .flatten();
            if let Some(entry) = entry {
                if entry.mode().is_tree() || entry.mode().is_commit() {
                    bail!("디렉터리나 서브모듈은 파일 폐기로 처리할 수 없습니다.");
                }
                tracked.push(relative.clone());
            } else {
                added.push(relative.clone());
            }
        }
    }
    if !tracked.is_empty() {
        let output = git_command(&root)
            .args([
                "--literal-pathspecs",
                "restore",
                "--source=HEAD",
                "--staged",
                "--worktree",
                "--",
            ])
            .args(&tracked)
            .output()?;
        if !output.status.success() {
            bail!(
                "변경사항 폐기 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    if !added.is_empty() {
        let output = git_command(&root)
            .args([
                "--literal-pathspecs",
                "rm",
                "--cached",
                "--force",
                "--ignore-unmatch",
                "--",
            ])
            .args(&added)
            .output()?;
        if !output.status.success() {
            bail!(
                "신규 파일 인덱스 제거 실패: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        for relative in added {
            let target = root.join(relative);
            match std::fs::remove_file(&target) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("선택한 신규 파일을 삭제하지 못했습니다."),
            }
        }
    }
    Ok(())
}

pub fn deletion_target(path: &Path) -> Result<std::path::PathBuf> {
    let metadata = std::fs::symlink_metadata(path).context("저장소 폴더를 찾을 수 없습니다.")?;
    if metadata.file_type().is_symlink() {
        bail!("링크 폴더는 삭제할 수 없습니다.");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            bail!("연결된 폴더는 삭제할 수 없습니다.");
        }
    }
    let root = path.canonicalize()?;
    if root.parent().is_none() || !root.join(".git").is_dir() || root.join(".git").is_symlink() {
        bail!(
            "일반 작업 저장소의 루트 폴더만 삭제할 수 있습니다. 연결된 worktree는 지원하지 않습니다."
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if std::fs::symlink_metadata(root.join(".git"))?.file_attributes() & 0x400 != 0 {
            bail!("연결된 Git 메타데이터 폴더가 있는 저장소는 삭제할 수 없습니다.");
        }
    }
    if feather_git::git::repository_root(&root)? != root {
        bail!("저장소 루트가 일치하지 않습니다.");
    }
    if root
        .join(".git/worktrees")
        .read_dir()
        .is_ok_and(|mut entries| entries.next().is_some())
    {
        bail!("연결된 worktree가 있는 저장소는 삭제할 수 없습니다.");
    }
    Ok(root)
}

#[cfg(windows)]
pub fn recycle_repository(path: &Path) -> Result<()> {
    let root = deletion_target(path)?;
    std::thread::Builder::new()
        .name("recycle-repository".into())
        .spawn(move || recycle_target(&root))?
        .join()
        .map_err(|_| anyhow::anyhow!("휴지통 작업이 중단되었습니다."))?
}

#[cfg(windows)]
fn recycle_target(path: &Path) -> Result<()> {
    use windows::{
        Win32::{System::Com::*, UI::Shell::*},
        core::HSTRING,
    };
    let root = deletion_target(path)?;
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        struct ComGuard;
        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _guard = ComGuard;
        let operation: IFileOperation =
            CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)?;
        operation.SetOperationFlags(
            FOF_ALLOWUNDO
                | FOF_NOCONFIRMATION
                | FOF_NOERRORUI
                | FOF_SILENT
                | FOFX_RECYCLEONDELETE
                | FOFX_EARLYFAILURE,
        )?;
        let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(root.as_os_str()), None)?;
        operation.DeleteItem(&item, None)?;
        operation
            .PerformOperations()
            .context("저장소를 휴지통으로 이동하지 못했습니다.")?;
        if operation.GetAnyOperationsAborted()?.as_bool() || root.exists() {
            bail!("저장소 삭제가 완료되지 않았습니다.");
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn recycle_repository(_path: &Path) -> Result<()> {
    bail!("이 환경에서는 휴지통 이동을 지원하지 않습니다.")
}

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

    fn configured_repository() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        create(&repo, None).unwrap();
        for args in [
            ["config", "user.name", "Test"],
            ["config", "user.email", "test@example.invalid"],
            ["config", "commit.gpgsign", "false"],
        ] {
            assert!(
                git_command(&repo)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        (temp, repo)
    }
    fn save_commit(repo: &Path, text: &str) {
        std::fs::write(repo.join("file.txt"), text).unwrap();
        stage_file(repo, b"file.txt").unwrap();
        commit(repo, text).unwrap();
    }
    #[test]
    fn batch_toggle_groups_mixed_states_and_validates_every_path_before_writing() {
        let (_temp, repo) = configured_repository();
        for name in ["a.txt", "b.txt", "[chosen].txt", "c.txt"] {
            std::fs::write(repo.join(name), "base").unwrap();
        }
        assert!(
            git_command(&repo)
                .args(["add", "."])
                .output()
                .unwrap()
                .status
                .success()
        );
        commit(&repo, "base").unwrap();
        for name in ["a.txt", "b.txt", "[chosen].txt", "c.txt"] {
            std::fs::write(repo.join(name), "changed").unwrap();
        }
        stage_file(&repo, b"b.txt").unwrap();
        let changes = [
            (b"a.txt".to_vec(), true),
            (b"b.txt".to_vec(), false),
            (b"[chosen].txt".to_vec(), true),
        ]
        .into_iter()
        .collect();
        toggle_files(&repo, &changes).unwrap();
        let files = feather_git::git::load_stage(&repo).unwrap();
        for (name, state) in [
            ("a.txt", "M "),
            ("b.txt", " M"),
            ("[chosen].txt", "M "),
            ("c.txt", " M"),
        ] {
            assert!(
                files
                    .iter()
                    .any(|file| file.label == name && file.stage_status() == state)
            );
            assert_eq!(std::fs::read_to_string(repo.join(name)).unwrap(), "changed");
        }
        let before = std::fs::read(repo.join(".git/index")).unwrap();
        let invalid = [(b"c.txt".to_vec(), true), (b"../outside".to_vec(), false)]
            .into_iter()
            .collect();
        assert!(toggle_files(&repo, &invalid).is_err());
        assert_eq!(std::fs::read(repo.join(".git/index")).unwrap(), before);
        let changes = [
            (b"a.txt".to_vec(), false),
            (b"[chosen].txt".to_vec(), false),
        ]
        .into_iter()
        .collect();
        toggle_files(&repo, &changes).unwrap();
        assert!(
            feather_git::git::load_stage(&repo)
                .unwrap()
                .iter()
                .all(|file| !file.staged)
        );
    }
    #[test]
    fn discard_restores_head_removes_only_selected_new_files_and_rejects_directories() {
        let (_temp, repo) = configured_repository();
        for name in ["modified.txt", "deleted.txt", "partial.txt", "keep.txt"] {
            std::fs::write(repo.join(name), "base").unwrap();
        }
        assert!(
            git_command(&repo)
                .args(["add", "."])
                .output()
                .unwrap()
                .status
                .success()
        );
        commit(&repo, "base").unwrap();
        for name in ["modified.txt", "partial.txt", "keep.txt"] {
            std::fs::write(repo.join(name), "indexed").unwrap();
            stage_file(&repo, name.as_bytes()).unwrap();
        }
        std::fs::write(repo.join("partial.txt"), "worktree").unwrap();
        std::fs::remove_file(repo.join("deleted.txt")).unwrap();
        stage_file(&repo, b"deleted.txt").unwrap();
        std::fs::write(repo.join("new [one].txt"), "new").unwrap();
        stage_file(&repo, b"new [one].txt").unwrap();
        std::fs::write(repo.join("untracked.txt"), "new").unwrap();
        std::fs::create_dir(repo.join("directory")).unwrap();
        std::fs::write(repo.join("directory/keep"), "keep").unwrap();
        assert!(discard_files(&repo, &[b"modified.txt".to_vec(), b"directory".to_vec()]).is_err());
        assert_eq!(
            std::fs::read_to_string(repo.join("modified.txt")).unwrap(),
            "indexed"
        );
        assert!(discard_files(&repo, &[b".git/config".to_vec()]).is_err());
        assert!(discard_files(&repo, &[b"../outside".to_vec()]).is_err());
        let paths = [
            "modified.txt",
            "deleted.txt",
            "partial.txt",
            "new [one].txt",
            "untracked.txt",
        ]
        .iter()
        .map(|name| name.as_bytes().to_vec())
        .collect::<Vec<_>>();
        discard_files(&repo, &paths).unwrap();
        for name in ["modified.txt", "deleted.txt", "partial.txt"] {
            assert_eq!(std::fs::read_to_string(repo.join(name)).unwrap(), "base");
        }
        assert!(!repo.join("new [one].txt").exists());
        assert!(!repo.join("untracked.txt").exists());
        assert_eq!(
            std::fs::read_to_string(repo.join("directory/keep")).unwrap(),
            "keep"
        );
        let files = feather_git::git::load_stage(&repo).unwrap();
        assert!(
            files
                .iter()
                .any(|file| file.label == "keep.txt" && file.staged)
        );
        assert!(
            !files
                .iter()
                .any(|file| paths.iter().any(|path| path == &file.path))
        );
        let (_temp, repo) = configured_repository();
        std::fs::write(repo.join("new.txt"), "new").unwrap();
        stage_file(&repo, b"new.txt").unwrap();
        discard_files(&repo, &[b"new.txt".to_vec()]).unwrap();
        assert!(!repo.join("new.txt").exists());
        assert!(feather_git::git::load_stage(&repo).unwrap().is_empty());
    }
    #[test]
    fn pull_push_and_commit_push_use_local_upstream_and_reject_divergence() {
        let (temp, source) = configured_repository();
        save_commit(&source, "first");
        let bare = temp.path().join("remote.git");
        assert!(
            git_command(temp.path())
                .args(["clone", "--bare"])
                .arg(&source)
                .arg(&bare)
                .output()
                .unwrap()
                .status
                .success()
        );
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        for repo in [&a, &b] {
            create(repo, Some(bare.to_str().unwrap())).unwrap();
            for args in [
                ["config", "user.name", "Test"],
                ["config", "user.email", "test@example.invalid"],
                ["config", "commit.gpgsign", "false"],
            ] {
                assert!(
                    git_command(repo)
                        .args(args)
                        .output()
                        .unwrap()
                        .status
                        .success()
                );
            }
        }
        std::fs::write(a.join("file.txt"), "second").unwrap();
        stage_file(&a, b"file.txt").unwrap();
        assert!(commit_then_push(&a, "second", true).unwrap().is_none());
        RemoteOperation::Pull.run(&b).unwrap();
        assert_eq!(
            std::fs::read_to_string(b.join("file.txt")).unwrap(),
            "second"
        );
        save_commit(&a, "third");
        RemoteOperation::Push.run(&a).unwrap();
        save_commit(&b, "local divergent");
        let head = git_command(&b)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        assert!(RemoteOperation::Pull.run(&b).is_err());
        assert_eq!(
            git_command(&b)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
            head
        );
        assert_eq!(
            std::fs::read_to_string(b.join("file.txt")).unwrap(),
            "local divergent"
        );
        let remote_head = git_command(&bare)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        assert!(commit_then_push(&a, "no staged changes", true).is_err());
        assert_eq!(
            git_command(&bare)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
            remote_head
        );
        assert!(
            git_command(&a)
                .args(["remote", "remove", "origin"])
                .output()
                .unwrap()
                .status
                .success()
        );
        std::fs::write(a.join("file.txt"), "committed without remote").unwrap();
        stage_file(&a, b"file.txt").unwrap();
        assert!(
            commit_then_push(&a, "local success", true)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            git_command(&a)
                .args(["show", "HEAD:file.txt"])
                .output()
                .unwrap()
                .stdout,
            b"committed without remote"
        );
    }

    #[test]
    fn commit_uses_only_index_and_preserves_unicode_multiline_message_and_worktree() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repo");
        create(&repository, None).unwrap();
        for args in [
            ["config", "user.name", "Test"],
            ["config", "user.email", "test@example.invalid"],
            ["config", "commit.gpgsign", "false"],
        ] {
            assert!(
                git_command(&repository)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        assert!(commit(&repository, "\n \t").is_err());
        assert!(
            commit(&repository, "nothing staged")
                .unwrap_err()
                .to_string()
                .contains("staged 변경사항이 없습니다")
        );
        std::fs::write(repository.join("tracked.txt"), "first").unwrap();
        stage_file(&repository, b"tracked.txt").unwrap();
        let message = "첫 커밋 😀\n\n본문 한 줄\n둘째 줄\n";
        commit(&repository, message).unwrap();
        let log = git_command(&repository)
            .args(["log", "-1", "--format=%B"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(log.stdout).unwrap().trim_end(),
            message.trim_end()
        );
        std::fs::write(repository.join("tracked.txt"), "indexed").unwrap();
        stage_file(&repository, b"tracked.txt").unwrap();
        std::fs::write(repository.join("tracked.txt"), "unstaged").unwrap();
        std::fs::write(repository.join("outside.txt"), "leave untracked").unwrap();
        commit(&repository, "second").unwrap();
        assert_eq!(
            git_command(&repository)
                .args(["show", "HEAD:tracked.txt"])
                .output()
                .unwrap()
                .stdout,
            b"indexed"
        );
        assert!(
            !git_command(&repository)
                .args(["cat-file", "-e", "HEAD:outside.txt"])
                .output()
                .unwrap()
                .status
                .success()
        );
        assert_eq!(
            std::fs::read_to_string(repository.join("tracked.txt")).unwrap(),
            "unstaged"
        );
        assert_eq!(
            std::fs::read_to_string(repository.join("outside.txt")).unwrap(),
            "leave untracked"
        );
        let head = git_command(&repository)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        assert!(
            commit(&repository, "no staged changes")
                .unwrap_err()
                .to_string()
                .contains("staged 변경사항이 없습니다")
        );
        assert!(commit(&repository, "bad\0message").is_err());
        assert_eq!(
            git_command(&repository)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
            head
        );
    }

    #[test]
    fn staging_is_literal_path_scoped_and_updates_two_column_statuses() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repo");
        create(&repository, None).unwrap();
        for name in [
            "modified.txt",
            "deleted.txt",
            "partial.txt",
            "[chosen].txt",
            "c.txt",
        ] {
            std::fs::write(repository.join(name), "base\n").unwrap();
        }
        assert!(
            git_command(&repository)
                .args(["add", "."])
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(
            git_command(&repository)
                .args([
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "commit",
                    "-m",
                    "base"
                ])
                .output()
                .unwrap()
                .status
                .success()
        );
        for name in ["modified.txt", "partial.txt", "[chosen].txt", "c.txt"] {
            std::fs::write(repository.join(name), "changed\n").unwrap();
        }
        std::fs::remove_file(repository.join("deleted.txt")).unwrap();
        std::fs::write(repository.join("new file.txt"), "new\n").unwrap();
        stage_file(&repository, b"partial.txt").unwrap();
        std::fs::write(repository.join("partial.txt"), "second change\n").unwrap();
        let files = feather_git::git::load_stage(&repository).unwrap();
        for (name, expected) in [
            ("new file.txt", "??"),
            ("modified.txt", " M"),
            ("deleted.txt", " D"),
        ] {
            assert!(
                files
                    .iter()
                    .any(|file| file.label == name && file.stage_status() == expected)
            );
        }
        let partial: Vec<_> = files
            .iter()
            .filter(|file| file.label == "partial.txt")
            .map(|file| file.stage_status())
            .collect();
        assert_eq!(partial, ["M ", " M"]);
        let rows = feather_git::model::file_rows(&files, false);
        assert_eq!(rows.len(), files.len());
        assert!(
            rows.iter()
                .all(|row| matches!(row, feather_git::model::FileRow::File(_)))
        );
        unstage_file(&repository, b"partial.txt").unwrap();
        let files = feather_git::git::load_stage(&repository).unwrap();
        let partial: Vec<_> = files
            .iter()
            .filter(|file| file.label == "partial.txt")
            .map(|file| file.stage_status())
            .collect();
        assert_eq!(partial, [" M"]);
        assert_eq!(
            std::fs::read_to_string(repository.join("partial.txt")).unwrap(),
            "second change\n"
        );
        for (name, expected) in [
            ("new file.txt", "A "),
            ("modified.txt", "M "),
            ("deleted.txt", "D "),
            ("[chosen].txt", "M "),
            ("partial.txt", "M "),
        ] {
            stage_file(&repository, name.as_bytes()).unwrap();
            let files = feather_git::git::load_stage(&repository).unwrap();
            assert!(
                files
                    .iter()
                    .any(|file| file.label == name && file.stage_status() == expected)
            );
            assert!(!files.iter().any(|file| file.label == name && !file.staged));
            assert!(
                files
                    .iter()
                    .any(|file| file.label == "c.txt" && file.stage_status() == " M")
            );
        }
        assert_eq!(
            std::fs::read_to_string(repository.join("partial.txt")).unwrap(),
            "second change\n"
        );
        assert_eq!(
            std::fs::read_to_string(repository.join("c.txt")).unwrap(),
            "changed\n"
        );
        assert!(!repository.join("deleted.txt").exists());
        stage_file(&repository, b"c.txt").unwrap();
        for (name, expected) in [
            ("new file.txt", "??"),
            ("modified.txt", " M"),
            ("deleted.txt", " D"),
            ("[chosen].txt", " M"),
            ("partial.txt", " M"),
        ] {
            unstage_file(&repository, name.as_bytes()).unwrap();
            let files = feather_git::git::load_stage(&repository).unwrap();
            assert!(
                files
                    .iter()
                    .any(|file| file.label == name && file.stage_status() == expected)
            );
            assert!(!files.iter().any(|file| file.label == name && file.staged));
            assert!(
                files
                    .iter()
                    .any(|file| file.label == "c.txt" && file.stage_status() == "M ")
            );
        }
        assert_eq!(
            std::fs::read_to_string(repository.join("partial.txt")).unwrap(),
            "second change\n"
        );
        assert_eq!(
            std::fs::read_to_string(repository.join("new file.txt")).unwrap(),
            "new\n"
        );
        assert!(!repository.join("deleted.txt").exists());
        stage_file(&repository, b"modified.txt").unwrap();
        std::fs::write(repository.join("modified.txt"), "base\n").unwrap();
        unstage_file(&repository, b"modified.txt").unwrap();
        assert!(
            !feather_git::git::load_stage(&repository)
                .unwrap()
                .iter()
                .any(|file| file.label == "modified.txt")
        );
        let before = std::fs::read(repository.join(".git/index")).unwrap();
        for path in [b"".as_slice(), b"../outside", b".", b"C:/outside"] {
            assert!(stage_file(&repository, path).is_err());
            assert!(unstage_file(&repository, path).is_err());
        }
        assert_eq!(
            std::fs::read(repository.join(".git/index")).unwrap(),
            before
        );
    }

    #[test]
    fn staging_new_file_works_before_first_commit() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repo");
        create(&repository, None).unwrap();
        std::fs::write(repository.join("새 파일.txt"), "content").unwrap();
        stage_file(&repository, "새 파일.txt".as_bytes()).unwrap();
        let files = feather_git::git::load_stage(&repository).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].stage_status(), "A ");
        assert_eq!(
            std::fs::read_to_string(repository.join("새 파일.txt")).unwrap(),
            "content"
        );
        std::fs::write(repository.join("새 파일.txt"), "newer content").unwrap();
        unstage_file(&repository, "새 파일.txt".as_bytes()).unwrap();
        let files = feather_git::git::load_stage(&repository).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].stage_status(), "??");
        assert_eq!(
            std::fs::read_to_string(repository.join("새 파일.txt")).unwrap(),
            "newer content"
        );
    }

    #[test]
    fn deletion_rejects_non_roots_and_linked_worktrees() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repo");
        create(&repository, None).unwrap();
        assert_eq!(
            deletion_target(&repository).unwrap(),
            repository.canonicalize().unwrap()
        );
        let child = repository.join("child");
        std::fs::create_dir(&child).unwrap();
        assert!(deletion_target(&child).is_err());
        assert!(deletion_target(temp.path()).is_err());
        std::fs::create_dir_all(repository.join(".git/worktrees/linked")).unwrap();
        assert!(deletion_target(&repository).is_err());
        let linked = temp.path().join("linked");
        std::fs::create_dir(&linked).unwrap();
        std::fs::write(linked.join(".git"), "gitdir: elsewhere").unwrap();
        assert!(deletion_target(&linked).is_err());
    }

    #[test]
    fn fetch_updates_remote_refs_without_changing_worktree() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        create(&source, None).unwrap();
        fn commit(path: &Path, contents: &str) {
            std::fs::write(path.join("tracked"), contents).unwrap();
            assert!(
                git_command(path)
                    .args(["add", "tracked"])
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
            assert!(
                git_command(path)
                    .args([
                        "-c",
                        "user.name=Test",
                        "-c",
                        "user.email=test@example.com",
                        "commit",
                        "-m",
                        contents
                    ])
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        assert!(fetch(&source).is_err());
        commit(&source, "first");
        let target = temp.path().join("target");
        create(&target, Some(source.to_str().unwrap())).unwrap();
        let before = git_command(&target)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        commit(&source, "second");
        std::fs::write(target.join("tracked"), "local changes").unwrap();
        fetch(&target).unwrap();
        assert_eq!(
            git_command(&target)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
            before
        );
        let remote = git_command(&target)
            .args(["rev-parse", "refs/remotes/origin/HEAD"])
            .output()
            .unwrap()
            .stdout;
        assert_ne!(remote, before);
        assert_eq!(
            std::fs::read_to_string(target.join("tracked")).unwrap(),
            "local changes"
        );
        let snapshot = feather_git::git::load_initial(&target, None).unwrap();
        let session = snapshot.session.unwrap();
        session.close().unwrap();
        assert!(session.next_page().is_err());
    }

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

#[cfg(test)]
mod stash_tests {
    use super::*;

    fn run(path: &Path, args: &[&str]) -> Vec<u8> {
        let output = git_command(path).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    #[test]
    fn stash_preserves_index_worktree_untracked_and_ignored() {
        let repo = tempfile::tempdir().unwrap();
        let path = repo.path();
        run(path, &["init", "-b", "main"]);
        run(path, &["config", "core.autocrlf", "false"]);
        run(path, &["config", "user.name", "Test"]);
        run(path, &["config", "user.email", "test@example.invalid"]);
        run(path, &["config", "commit.gpgsign", "false"]);
        std::fs::write(path.join("tracked"), "base\n").unwrap();
        std::fs::write(path.join(".gitignore"), "ignored\n").unwrap();
        run(path, &["add", "."]);
        run(path, &["commit", "-m", "base"]);
        assert!(stash_all(path, "--이름 있는 작업").is_err());
        std::fs::write(path.join("tracked"), "staged\n").unwrap();
        run(path, &["add", "tracked"]);
        std::fs::write(path.join("tracked"), "unstaged\n").unwrap();
        std::fs::write(path.join("new"), "untracked\n").unwrap();
        std::fs::write(path.join("ignored"), "keep\n").unwrap();
        let before = run(path, &["status", "--porcelain=v1", "-z"]);
        assert!(stash_all(path, "   ").is_err());
        assert_eq!(run(path, &["status", "--porcelain=v1", "-z"]), before);
        stash_all(path, "--이름 있는 작업").unwrap();
        assert!(run(path, &["status", "--porcelain=v1", "-z"]).is_empty());
        assert_eq!(std::fs::read(path.join("tracked")).unwrap(), b"base\n");
        assert!(!path.join("new").exists());
        assert_eq!(std::fs::read(path.join("ignored")).unwrap(), b"keep\n");
        let chosen = String::from_utf8(run(path, &["rev-parse", "refs/stash"]))
            .unwrap()
            .trim()
            .to_owned();
        std::fs::write(path.join("tracked"), "other stash\n").unwrap();
        stash_all(path, "--이름 있는 작업").unwrap();
        let subjects = String::from_utf8(run(path, &["stash", "list", "--format=%gs"])).unwrap();
        assert!(subjects.lines().all(|s| s.ends_with("--이름 있는 작업")));
        let snapshot = feather_git::git::load_initial(path, None).unwrap();
        assert_eq!(
            snapshot
                .references
                .iter()
                .filter(|r| r.kind == feather_git::model::RefTab::Stash)
                .count(),
            2
        );
        assert!(restore_stash(path, "--all").is_err());
        let head = String::from_utf8(run(path, &["rev-parse", "HEAD"])).unwrap();
        assert!(restore_stash(path, head.trim()).is_err());
        assert_eq!(
            run(path, &["stash", "list", "--format=%gd"])
                .split(|b| *b == b'\n')
                .filter(|s| !s.is_empty())
                .count(),
            2
        );
        let entries = run(path, &["stash", "list", "--format=%H"]);
        restore_stash(path, &chosen).unwrap();
        assert_eq!(run(path, &["stash", "list", "--format=%H"]), entries);
        assert_eq!(run(path, &["status", "--porcelain=v1", "-z"]), before);
        assert_eq!(run(path, &["show", ":tracked"]), b"staged\n");
        assert_eq!(std::fs::read(path.join("tracked")).unwrap(), b"unstaged\n");
        assert_eq!(std::fs::read(path.join("new")).unwrap(), b"untracked\n");
        std::fs::write(path.join("tracked"), "local edit\n").unwrap();
        assert!(restore_stash(path, &chosen).is_err());
        assert_eq!(
            std::fs::read(path.join("tracked")).unwrap(),
            b"local edit\n"
        );
        assert_eq!(run(path, &["stash", "list", "--format=%H"]), entries);
        let status = run(path, &["status", "--porcelain=v1", "-z"]);
        let index = run(path, &["show", ":tracked"]);
        let newest = String::from_utf8(run(path, &["rev-parse", "stash@{0}"])).unwrap();
        assert!(delete_stash(path, "stash@{0}", &chosen).is_err());
        assert!(delete_stash(path, "--all", &chosen).is_err());
        delete_stash(path, "stash@{1}", &chosen).unwrap();
        assert!(delete_stash(path, "stash@{1}", &chosen).is_err());
        assert_eq!(
            run(path, &["stash", "list", "--format=%H"]),
            newest.as_bytes()
        );
        assert_eq!(run(path, &["status", "--porcelain=v1", "-z"]), status);
        assert_eq!(run(path, &["show", ":tracked"]), index);
        assert_eq!(
            std::fs::read(path.join("tracked")).unwrap(),
            b"local edit\n"
        );
        assert_eq!(std::fs::read(path.join("new")).unwrap(), b"untracked\n");
    }
}
