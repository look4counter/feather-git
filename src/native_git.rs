pub use crate::native_local::add_remote;
use crate::{native_local as local, native_refs as refs};
use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

/// Existing operation boundaries use this in-process request builder; no process is spawned.
#[derive(Default)]
pub struct Command {
    pub path: PathBuf,
    args: Vec<OsString>,
    env: BTreeMap<OsString, OsString>,
}
pub struct Status(i32);
impl Status {
    pub fn success(&self) -> bool {
        self.0 == 0
    }
    pub fn code(&self) -> Option<i32> {
        Some(self.0)
    }
}
pub struct Output {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
impl Command {
    pub fn at(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            ..Default::default()
        }
    }
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }
    pub fn env(&mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.env.insert(name.as_ref().into(), value.as_ref().into());
        self
    }
    pub fn output(&mut self) -> std::io::Result<Output> {
        let args = self
            .args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let (status, stdout, stderr) = match self.run(&args) {
            Ok(v) => (0, v, Vec::new()),
            Err(e) => (1, Vec::new(), format!("{e:#}").into_bytes()),
        };
        Ok(Output {
            status: Status(status),
            stdout,
            stderr,
        })
    }
    fn run(&self, args: &[&str]) -> Result<Vec<u8>> {
        let mut a = args;
        while a.first() == Some(&"-c") {
            a = a.get(2..).context("잘못된 요청입니다.")?;
        }
        if a.first() == Some(&"--literal-pathspecs") {
            a = &a[1..];
        }
        match a.first().copied().unwrap_or_default() {
            "fetch" => {
                crate::native_remote::fetch(&self.path, true, None)?;
                Ok(Vec::new())
            }
            "pull" => {
                crate::native_remote::pull(&self.path)?;
                Ok(Vec::new())
            }
            "push" => {
                crate::native_remote::push(&self.path, a)?;
                Ok(Vec::new())
            }
            "switch" => {
                crate::native_operations::checkout(&self.path, a)?;
                Ok(Vec::new())
            }
            "reset" if !a.contains(&"--quiet") => {
                crate::native_operations::reset(&self.path, a)?;
                Ok(Vec::new())
            }
            "restore" => {
                crate::native_operations::restore(&self.path, a)?;
                Ok(Vec::new())
            }
            "stash" => crate::native_operations::stash(&self.path, a),
            "merge" => {
                crate::native_operations::merge(&self.path, a)?;
                Ok(Vec::new())
            }
            "rebase" => {
                crate::native_operations::rebase(&self.path, a)?;
                Ok(Vec::new())
            }
            "revert" => {
                crate::native_operations::revert(&self.path, a)?;
                Ok(Vec::new())
            }
            "status" => Ok(if crate::git::load_stage(&self.path)?.is_empty() {
                Vec::new()
            } else {
                b"changes".to_vec()
            }),
            "remote" => refs::remote_request(&self.path, a),
            "for-each-ref" => refs::list(&self.path, a),
            "update-ref" => refs::update(&self.path, a),
            "branch" => refs::branch(&self.path, a),
            "tag" => refs::tag(&self.path, a),
            "worktree" => refs::worktrees(&self.path),
            "merge-base" => {
                let repo = local::repo(&self.path)?;
                let left = local::oid(&repo, a[2])?;
                let right = local::oid(&repo, a[3])?;
                if repo.merge_base(left, right)?.detach() != left {
                    bail!("조상 커밋이 아닙니다.");
                }
                Ok(Vec::new())
            }
            "add" => {
                local::stage_paths(&self.path, &paths_after_separator(a), true, false)?;
                Ok(Vec::new())
            }
            "rm" => {
                local::stage_paths(&self.path, &paths_after_separator(a), false, true)?;
                Ok(Vec::new())
            }
            "reset" if a.contains(&"--quiet") => {
                local::stage_paths(&self.path, &paths_after_separator(a), false, false)?;
                Ok(Vec::new())
            }
            "init" => {
                init_repository(&self.path)?;
                Ok(Vec::new())
            }
            "update-index" if a.contains(&"--no-split-index") => {
                let alternative = self
                    .env
                    .get(OsStr::new("GIT_INDEX_FILE"))
                    .map(OsString::as_os_str);
                let mut index = match local::index(&self.path, alternative) {
                    Ok(index) => index,
                    Err(error) => {
                        let original = local::repo(&self.path)?.index_path();
                        let target = alternative
                            .map(PathBuf::from)
                            .context("전체 인덱스 대상이 없습니다.")?;
                        if std::fs::read(&original)? != std::fs::read(&target)? {
                            return Err(error);
                        }
                        let mut index = local::index(&self.path, None)?;
                        index.set_path(target);
                        index
                    }
                };
                index.remove_tree();
                index.write(Default::default())?;
                Ok(Vec::new())
            }
            "rev-list" => {
                let repo = local::repo(&self.path)?;
                let id = local::oid(&repo, a.last().unwrap())?;
                let commit = repo.find_commit(id)?;
                let mut ids = vec![id.to_string()];
                ids.extend(commit.parent_ids().map(|id| id.to_string()));
                Ok(ids.join(" ").into_bytes())
            }
            "ls-tree" => {
                let repo = local::repo(&self.path)?;
                let id = local::oid(&repo, a.last().unwrap())?;
                let object = repo.find_object(id)?;
                let tree = if object.kind == gix::objs::Kind::Commit {
                    repo.find_commit(id)?.tree()?
                } else {
                    repo.find_tree(id)?
                };
                let mut out = Vec::new();
                list_tree(&repo, &tree, &[], &mut out)?;
                Ok(out)
            }
            "config" => self.config(&a[1..]),
            "rev-parse" => {
                let repo = local::repo(&self.path)?;
                if a.get(1) == Some(&"--git-path") {
                    return Ok(repo.path().join(a[2]).to_string_lossy().as_bytes().to_vec());
                }
                Ok(local::oid(&repo, a.last().context("참조가 없습니다.")?)?
                    .to_string()
                    .into_bytes())
            }
            "check-ref-format" => {
                let _: gix::refs::FullName = a
                    .last()
                    .context("참조가 없습니다.")?
                    .to_string()
                    .try_into()?;
                Ok(Vec::new())
            }
            "show-ref" => {
                local::repo(&self.path)?.find_reference(*a.last().unwrap())?;
                Ok(Vec::new())
            }
            "symbolic-ref" => {
                let repo = local::repo(&self.path)?;
                let reference = repo.find_reference(*a.last().unwrap())?;
                let name = reference
                    .target()
                    .try_name()
                    .context("심볼릭 참조가 아닙니다.")?
                    .as_bstr()
                    .to_str()?
                    .to_owned();
                Ok(if a.contains(&"--short") {
                    name.strip_prefix("refs/heads/")
                        .unwrap_or(&name)
                        .as_bytes()
                        .to_vec()
                } else {
                    name.into_bytes()
                })
            }
            "write-tree" => {
                let repo = local::repo(&self.path)?;
                let index = local::index(
                    &self.path,
                    self.env
                        .get(OsStr::new("GIT_INDEX_FILE"))
                        .map(OsString::as_os_str),
                )?;
                Ok(local::index_tree(&repo, &index)?.to_string().into_bytes())
            }
            "ls-files" => {
                let index = local::index(
                    &self.path,
                    self.env
                        .get(OsStr::new("GIT_INDEX_FILE"))
                        .map(OsString::as_os_str),
                )?;
                let mut out = Vec::new();
                for e in index.entries() {
                    out.extend_from_slice(
                        format!("{:06o} {} {}\t", e.mode.bits(), e.id, e.stage() as u8).as_bytes(),
                    );
                    out.extend_from_slice(e.path(&index));
                    out.push(0);
                }
                Ok(out)
            }
            _ => bail!(
                "아직 전환되지 않은 라이브러리 작업입니다: {}",
                a.first().unwrap_or(&"")
            ),
        }
    }
    fn config(&self, args: &[&str]) -> Result<Vec<u8>> {
        let scope = args
            .iter()
            .find(|s| ["--global", "--local"].contains(s))
            .copied();
        let operation = args
            .iter()
            .find(|s| ["--get-all", "--unset-all", "--add"].contains(s))
            .copied();
        let key = args
            .iter()
            .position(|s| Some(*s) == operation)
            .and_then(|i| args.get(i + 1))
            .copied()
            .context("설정 요청이 올바르지 않습니다.")?;
        let values = local::config_values(&self.path, scope, key, &self.env)?;
        match operation {
            Some("--get-all") => {
                if values.is_empty() {
                    bail!("설정값이 없습니다.");
                }
                let mut out = values
                    .join(if args.contains(&"--null") { "\0" } else { "\n" })
                    .into_bytes();
                out.push(if args.contains(&"--null") { 0 } else { b'\n' });
                Ok(out)
            }
            Some("--unset-all") => {
                local::config_set(&self.path, scope == Some("--global"), key, &[], &self.env)?;
                Ok(Vec::new())
            }
            Some("--add") => {
                let mut values = values;
                values.push(args.last().unwrap().to_string());
                local::config_set(
                    &self.path,
                    scope == Some("--global"),
                    key,
                    &values,
                    &self.env,
                )?;
                Ok(Vec::new())
            }
            _ => bail!("지원하지 않는 설정 작업입니다."),
        }
    }
}

fn paths_after_separator(a: &[&str]) -> Vec<PathBuf> {
    a.iter()
        .position(|s| *s == "--")
        .map(|i| a[i + 1..].iter().map(PathBuf::from).collect())
        .unwrap_or_default()
}
fn list_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    prefix: &[u8],
    out: &mut Vec<u8>,
) -> Result<()> {
    for entry in tree.iter() {
        let entry = entry?;
        let mut name = prefix.to_vec();
        name.extend_from_slice(entry.filename());
        if entry.mode().is_tree() {
            name.push(b'/');
            list_tree(repo, &repo.find_tree(entry.object_id())?, &name, out)?;
        } else {
            out.extend_from_slice(
                format!(
                    "{:06o} {} {}\t",
                    entry.mode().value(),
                    if entry.mode().is_commit() {
                        "commit"
                    } else {
                        "blob"
                    },
                    entry.object_id()
                )
                .as_bytes(),
            );
            out.extend_from_slice(&name);
            out.push(0);
        }
    }
    Ok(())
}
pub fn commit(path: &Path, message: &str) -> Result<()> {
    local::commit(path, message)
}
pub fn gix_repository(path: &Path) -> Result<gix::Repository> {
    local::repo(path)
}
pub fn init_repository(path: &Path) -> Result<()> {
    gix::ThreadSafeRepository::init_opts(
        path,
        gix::create::Kind::WithWorktree,
        Default::default(),
        local::open_options(),
    )?;
    Ok(())
}
pub fn clone_repository(url: &str, path: &Path) -> Result<()> {
    crate::native_remote::clone_repository(url, path)
}
pub fn credential_helpers() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for variable in ["ProgramFiles", "LOCALAPPDATA"] {
        if let Some(base) = std::env::var_os(variable) {
            for suffix in [
                "Git/mingw64/bin/git-credential-manager.exe",
                "Git/mingw64/libexec/git-core/git-credential-manager.exe",
                "Git Credential Manager/git-credential-manager.exe",
                "Programs/Git/mingw64/bin/git-credential-manager.exe",
            ] {
                let p = PathBuf::from(&base).join(suffix);
                if p.is_file() {
                    paths.push(p);
                }
            }
        }
    }
    paths.extend([
        PathBuf::from("git-credential-manager"),
        PathBuf::from("git-credential-manager-core"),
    ]);
    paths
}

pub fn ordinary_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path.to_owned()
}

pub fn set_index_content(
    path: &Path,
    raw: &[u8],
    content: Option<&[u8]>,
    mode: u32,
    expected: Option<(gix::ObjectId, u32)>,
) -> Result<()> {
    local::set_index_content(path, raw, content, mode, expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &Path, args: &[&str]) {
        let output = Command::at(path).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn native_workflows_run_with_git_removed_from_path() {
        let temp = tempfile::tempdir().unwrap();
        let empty = temp.path().join("empty-path");
        std::fs::create_dir(&empty).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "native_git::tests::without_git_child",
                "--nocapture",
            ])
            .env("PATH", empty)
            .env("FEATHER_WITHOUT_GIT_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    #[ignore = "isolated child process with Git removed from PATH"]
    fn without_git_child() {
        assert_eq!(std::env::var("FEATHER_WITHOUT_GIT_CHILD").unwrap(), "1");
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        gix::init(&source).unwrap();
        for (key, value) in [
            ("user.name", "Native test"),
            ("user.email", "native@example.invalid"),
            ("core.autocrlf", "false"),
            ("commit.gpgsign", "false"),
        ] {
            run(&source, &["config", "--local", "--add", key, value]);
        }
        std::fs::write(source.join("file.txt"), "one\n").unwrap();
        run(&source, &["add", "--", "file.txt"]);
        commit(&source, "first").unwrap();
        let bare = temp.path().join("remote.git");
        git2::Repository::init_bare(ordinary_path(&bare)).unwrap();
        let remote = bare.to_str().unwrap();
        run(
            &source,
            &["config", "--local", "--add", "remote.origin.url", remote],
        );
        run(
            &source,
            &[
                "config",
                "--local",
                "--add",
                "remote.origin.fetch",
                "+refs/heads/*:refs/remotes/origin/*",
            ],
        );
        let name = local::repo(&source)
            .unwrap()
            .head_name()
            .unwrap()
            .unwrap()
            .as_bstr()
            .to_str()
            .unwrap()
            .to_owned();
        run(
            &source,
            &[
                "push",
                "-u",
                &format!("--force-with-lease={name}:"),
                "--",
                "origin",
                &format!("{name}:{name}"),
            ],
        );
        let bare_repo = git2::Repository::open_bare(ordinary_path(&bare)).unwrap();
        bare_repo.set_head(&name).unwrap();
        let cloned = temp.path().join("cloned");
        clone_repository(remote, &cloned).unwrap();
        std::fs::write(source.join("file.txt"), "two\n").unwrap();
        run(&source, &["add", "--", "file.txt"]);
        commit(&source, "second").unwrap();
        run(&source, &["push"]);
        run(&cloned, &["fetch", "--all"]);
        run(&cloned, &["pull", "--ff-only"]);
        assert_eq!(
            std::fs::read_to_string(cloned.join("file.txt"))
                .unwrap()
                .replace("\r\n", "\n"),
            "two\n"
        );
        run(&source, &["branch", "--no-track", "--", "topic", "HEAD"]);
        run(&source, &["switch", "--no-guess", "--", "topic"]);
        std::fs::write(source.join("file.txt"), "stash\n").unwrap();
        run(
            &source,
            &["stash", "push", "--include-untracked", "--message", "saved"],
        );
        let stash = local::oid(&local::repo(&source).unwrap(), "refs/stash")
            .unwrap()
            .to_string();
        run(&source, &["stash", "apply", "--index", &stash]);
        assert_eq!(std::fs::read(source.join("file.txt")).unwrap(), b"stash\n");
    }
}
