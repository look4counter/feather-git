use crate::settings_config as local;
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

pub struct Command {
    path: PathBuf,
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
}
impl Command {
    pub fn at(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            args: vec![],
            env: BTreeMap::new(),
        }
    }
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }
    pub fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        for a in args {
            self.arg(a);
        }
        self
    }
    pub fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.env
            .insert(key.as_ref().to_owned(), value.as_ref().to_owned());
        self
    }
    pub fn output(&self) -> std::io::Result<Output> {
        Ok(match self.run() {
            Ok((code, stdout)) => Output {
                status: Status(code),
                stdout,
            },
            Err(_) => Output {
                status: Status(128),
                stdout: vec![],
            },
        })
    }
    fn run(&self) -> Result<(i32, Vec<u8>)> {
        let a: Vec<_> = self
            .args
            .iter()
            .map(|x| x.to_str().context("Invalid argument"))
            .collect::<Result<_>>()?;
        match a.first().copied() {
            Some("config") => {
                let scope = a
                    .iter()
                    .find(|s| ["--global", "--local"].contains(s))
                    .copied();
                let op = a
                    .iter()
                    .position(|s| ["--get-all", "--unset-all", "--add"].contains(s))
                    .context("Missing operation")?;
                let key = *a.get(op + 1).context("Missing key")?;
                let mut values = local::config_values(&self.path, scope, key, &self.env)?;
                match a[op] {
                    "--get-all" => {
                        if values.is_empty() {
                            return Ok((1, vec![]));
                        }
                        let sep = if a.contains(&"--null") { "\0" } else { "\n" };
                        Ok((0, (values.join(sep) + sep).into_bytes()))
                    }
                    "--unset-all" => {
                        if values.is_empty() {
                            return Ok((5, vec![]));
                        }
                        local::config_set(
                            &self.path,
                            scope == Some("--global"),
                            key,
                            &[],
                            &self.env,
                        )?;
                        Ok((0, vec![]))
                    }
                    "--add" => {
                        values.push(a.get(op + 2).context("Missing value")?.to_string());
                        local::config_set(
                            &self.path,
                            scope == Some("--global"),
                            key,
                            &values,
                            &self.env,
                        )?;
                        Ok((0, vec![]))
                    }
                    _ => unreachable!(),
                }
            }
            Some("remote") => {
                let repo = git2::Repository::open(&self.path)?;
                match a.get(1).copied() {
                    None => {
                        let mut names = repo
                            .remotes()?
                            .iter()
                            .collect::<Result<Vec<_>, _>>()?
                            .into_iter()
                            .flatten()
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        names.sort();
                        names.dedup();
                        Ok((0, names.join("\n").into_bytes()))
                    }
                    Some("remove") => {
                        repo.remote_delete(a.get(2).context("Missing remote")?)?;
                        Ok((0, vec![]))
                    }
                    Some("get-url") => {
                        let name = a.last().context("Missing remote")?;
                        let fetch = local::config_values(
                            &self.path,
                            None,
                            &format!("remote.{name}.url"),
                            &self.env,
                        )?;
                        let mut urls = if a.contains(&"--push") {
                            local::config_values(
                                &self.path,
                                None,
                                &format!("remote.{name}.pushurl"),
                                &self.env,
                            )?
                        } else {
                            fetch.clone()
                        };
                        if urls.is_empty() {
                            urls = fetch;
                        }
                        if urls.is_empty() {
                            bail!("Remote not found");
                        }
                        Ok((0, urls.join("\n").into_bytes()))
                    }
                    _ => bail!("Unsupported remote operation"),
                }
            }
            _ => bail!("Unsupported settings operation"),
        }
    }
}
pub fn add_remote(path: &Path, name: &str, url: &str) -> Result<()> {
    local::add_remote(path, name, url)
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
