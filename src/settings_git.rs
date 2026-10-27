use anyhow::{Context, Result, bail};
use std::{path::PathBuf, process::Command};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

#[derive(Clone)]
pub struct GitContext {
    pub repository: Option<PathBuf>,
    #[cfg(test)]
    global_file: Option<PathBuf>,
}

#[derive(Default)]
pub struct RemoteInfo {
    pub name: String,
    pub fetch_urls: Vec<String>,
    pub push_urls: Vec<String>,
}

#[derive(Default)]
pub struct RepositoryInfo {
    pub remotes: Vec<RemoteInfo>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
}

pub struct SettingsSnapshot {
    pub global: Identity,
    pub effective: Identity,
    pub use_global: bool,
    pub info: RepositoryInfo,
}

impl GitContext {
    pub fn new(repository: Option<PathBuf>) -> Self {
        Self {
            repository,
            #[cfg(test)]
            global_file: None,
        }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new("git");
        if let Some(path) = &self.repository {
            command.current_dir(path);
        }
        command.env("GIT_TERMINAL_PROMPT", "0");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        #[cfg(test)]
        if let Some(global) = &self.global_file {
            command
                .env("GIT_CONFIG_GLOBAL", global)
                .env("GIT_CONFIG_NOSYSTEM", "1");
        }
        command
    }

    fn values(&self, scope: Option<&str>, key: &str) -> Result<Vec<String>> {
        let mut command = self.command();
        command.arg("config");
        if let Some(scope) = scope {
            command.arg(scope);
        }
        let output = command
            .args(["--null", "--get-all", key])
            .output()
            .context("Git 설정을 읽을 수 없습니다.")?;
        if output.status.code() == Some(1) {
            return Ok(vec![]);
        }
        if !output.status.success() {
            bail!("Git 설정을 읽을 수 없습니다. 설정 파일과 저장소 권한을 확인하세요.");
        }
        Ok(String::from_utf8(output.stdout)?
            .split_terminator('\0')
            .map(str::to_owned)
            .collect())
    }

    fn read_identity(&self, scope: Option<&str>) -> Result<Identity> {
        Ok(Identity {
            name: self.values(scope, "user.name")?.pop().unwrap_or_default(),
            email: self.values(scope, "user.email")?.pop().unwrap_or_default(),
        })
    }

    pub fn identity(&self, scope: &str) -> Result<Identity> {
        self.read_identity(Some(scope))
    }

    pub fn effective_identity(&self) -> Result<Identity> {
        if self.repository.is_some() {
            self.read_identity(None)
        } else {
            self.identity("--global")
        }
    }

    fn set_values(&self, scope: &str, key: &str, values: &[String]) -> Result<()> {
        let output = self
            .command()
            .args(["config", scope, "--unset-all", key])
            .output()?;
        if !output.status.success() && output.status.code() != Some(5) {
            bail!("Git 설정을 변경할 수 없습니다. 파일 잠금과 권한을 확인하세요.");
        }
        for value in values {
            let output = self
                .command()
                .args(["config", scope, "--add", key, value])
                .output()?;
            if !output.status.success() {
                bail!("Git 설정을 저장하지 못했습니다.");
            }
        }
        Ok(())
    }

    pub fn save_identity(&self, identity: &Identity, global: bool) -> Result<()> {
        let identity = Identity {
            name: identity.name.trim().into(),
            email: identity.email.trim().into(),
        };
        if identity.name.is_empty()
            || identity
                .name
                .chars()
                .any(|c| c.is_control() || "<>".contains(c))
            || identity.email.is_empty()
            || !identity
                .email
                .split_once('@')
                .is_some_and(|(name, domain)| {
                    !name.is_empty() && !domain.is_empty() && !domain.contains('@')
                })
            || identity
                .email
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "<>".contains(c))
        {
            bail!("이름과 이메일 주소를 올바르게 입력하세요. 줄바꿈은 사용할 수 없습니다.");
        }
        if !global && self.repository.is_none() {
            bail!("저장소를 연 뒤 개별 설정을 저장하세요.");
        }
        let scope = if global { "--global" } else { "--local" };
        let mut before = Vec::new();
        for key in ["user.name", "user.email"] {
            before.push((scope, key, self.values(Some(scope), key)?));
            if global && self.repository.is_some() {
                before.push(("--local", key, self.values(Some("--local"), key)?));
            }
        }
        let result = (|| {
            self.set_values(scope, "user.name", std::slice::from_ref(&identity.name))?;
            self.set_values(scope, "user.email", std::slice::from_ref(&identity.email))?;
            if global && self.repository.is_some() {
                self.set_values("--local", "user.name", &[])?;
                self.set_values("--local", "user.email", &[])?;
            }
            if self.effective_identity()? != identity {
                bail!("다른 Git 설정이 사용자 정보를 덮어쓰고 있습니다.");
            }
            Ok(())
        })();
        if let Err(error) = result {
            let restored = before.iter().fold(true, |restored, (scope, key, values)| {
                let unchanged = self.values(Some(scope), key).ok().as_ref() == Some(values);
                (unchanged || self.set_values(scope, key, values).is_ok()) && restored
            });
            if !restored {
                bail!(
                    "{error} 기존 값 일부를 복구하지 못했습니다. 설정 파일 권한과 잠금을 확인하세요."
                );
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn load(&self) -> Result<SettingsSnapshot> {
        let global = self.identity("--global")?;
        let use_global = if self.repository.is_some() {
            self.values(Some("--local"), "user.name")?.is_empty()
                && self.values(Some("--local"), "user.email")?.is_empty()
        } else {
            true
        };
        Ok(SettingsSnapshot {
            global,
            effective: self.effective_identity()?,
            use_global,
            info: self.remote_info()?,
        })
    }

    pub fn remote_info(&self) -> Result<RepositoryInfo> {
        if self.repository.is_none() {
            return Ok(RepositoryInfo::default());
        }
        let output = self.command().arg("remote").output()?;
        if !output.status.success() {
            bail!("현재 저장소의 원격 정보를 읽을 수 없습니다.");
        }
        let mut remotes = vec![];
        for name in String::from_utf8(output.stdout)?.lines() {
            let urls = |push: bool| -> Result<Vec<String>> {
                let mut command = self.command();
                command.args(["remote", "get-url", "--all"]);
                if push {
                    command.arg("--push");
                }
                let output = command.args(["--", name]).output()?;
                if !output.status.success() {
                    bail!("원격 저장소 주소를 읽을 수 없습니다.");
                }
                Ok(String::from_utf8(output.stdout)?
                    .lines()
                    .map(str::to_owned)
                    .collect())
            };
            remotes.push(RemoteInfo {
                name: name.into(),
                fetch_urls: urls(false)?,
                push_urls: urls(true)?,
            });
        }
        let output = self
            .command()
            .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
            .output()?;
        let branch = output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned());
        let upstream = if let Some(branch) = &branch {
            let remote = self.values(None, &format!("branch.{branch}.remote"))?.pop();
            let merge = self.values(None, &format!("branch.{branch}.merge"))?.pop();
            remote.zip(merge).map(|(remote, merge)| {
                format!(
                    "{remote}/{}",
                    merge.strip_prefix("refs/heads/").unwrap_or(&merge)
                )
            })
        } else {
            None
        };
        Ok(RepositoryInfo {
            remotes,
            branch,
            upstream,
        })
    }

    pub fn gcm_configured(&self) -> bool {
        let helpers = self.values(None, "credential.helper").unwrap_or_default();
        helpers
            .rsplit(|value| value.is_empty())
            .next()
            .unwrap_or_default()
            .iter()
            .any(|value| {
                value == "manager"
                    || value == "manager-core"
                    || value.contains("git-credential-manager")
            })
    }
}

pub fn display_url(value: &str) -> String {
    if let Ok(mut url) = url::Url::parse(value)
        && url.has_host()
    {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_query(None);
        url.set_fragment(None);
        return url.to_string();
    }
    // SCP 형식의 SSH 주소나 로컬 경로에도 쿼리/프래그먼트는 표시하지 않는다.
    let value = value.split(['?', '#']).next().unwrap_or_default();
    if let Some((scheme, address)) = value.split_once("://") {
        let (authority, path) = address.split_once('/').unwrap_or((address, ""));
        let host = authority.rsplit('@').next().unwrap_or_default();
        return format!("{scheme}://{host}/{path}");
    }
    value.rsplit('@').next().unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_remote_credentials_are_redacted_and_helper_resets_are_respected() {
        assert_eq!(
            display_url("https://user:secret@example.invalid:bad/repo?token=secret"),
            "https://example.invalid:bad/repo"
        );
        let (_temp, context) = context();
        context
            .set_values("--global", "credential.helper", &["manager".into()])
            .unwrap();
        context
            .set_values("--local", "credential.helper", &[String::new()])
            .unwrap();
        assert!(!context.gcm_configured());
        context
            .set_values(
                "--local",
                "credential.helper",
                &[String::new(), "manager".into()],
            )
            .unwrap();
        assert!(context.gcm_configured());
    }

    fn context() -> (tempfile::TempDir, GitContext) {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        let context = GitContext {
            repository: Some(repo),
            global_file: Some(temp.path().join("global.config")),
        };
        assert!(
            context
                .command()
                .arg("init")
                .output()
                .unwrap()
                .status
                .success()
        );
        (temp, context)
    }

    #[test]
    fn global_identity_removes_local_overrides_and_local_save_preserves_global() {
        let (_temp, context) = context();
        let global = Identity {
            name: "전역 사용자".into(),
            email: "global@example.invalid".into(),
        };
        let local = Identity {
            name: "개별 사용자".into(),
            email: "local@example.invalid".into(),
        };
        context.save_identity(&global, true).unwrap();
        context.save_identity(&local, false).unwrap();
        assert_eq!(context.identity("--global").unwrap(), global);
        assert_eq!(context.effective_identity().unwrap(), local);
        context.save_identity(&global, true).unwrap();
        assert_eq!(context.identity("--local").unwrap(), Identity::default());
        assert_eq!(context.effective_identity().unwrap(), global);
    }

    #[test]
    fn invalid_values_and_missing_repository_do_not_write_config() {
        let (_temp, mut context) = context();
        let valid = Identity {
            name: "사용자".into(),
            email: "user@example.invalid".into(),
        };
        context.save_identity(&valid, true).unwrap();
        assert!(
            context
                .save_identity(
                    &Identity {
                        name: "bad\nname".into(),
                        ..valid.clone()
                    },
                    true
                )
                .is_err()
        );
        assert_eq!(context.identity("--global").unwrap(), valid);
        for email in ["@example.invalid", "user@", "user@@example.invalid"] {
            assert!(
                context
                    .save_identity(
                        &Identity {
                            email: email.into(),
                            ..valid.clone()
                        },
                        true
                    )
                    .is_err()
            );
        }
        assert_eq!(context.identity("--global").unwrap(), valid);
        context.repository = None;
        assert!(context.save_identity(&valid, false).is_err());
    }

    #[test]
    fn config_write_failure_restores_identity() {
        let (_temp, context) = context();
        let before = Identity {
            name: "before".into(),
            email: "before@example.invalid".into(),
        };
        context.save_identity(&before, true).unwrap();
        // 로컬 잠금으로 전역 저장 후 로컬 제거 단계의 실패를 유도한다.
        std::fs::write(
            context
                .repository
                .as_ref()
                .unwrap()
                .join(".git/config.lock"),
            "locked",
        )
        .unwrap();
        let changed = Identity {
            name: "after".into(),
            email: "after@example.invalid".into(),
        };
        assert!(context.save_identity(&changed, true).is_err());
        assert_eq!(context.identity("--global").unwrap(), before);
    }

    #[test]
    fn remote_urls_and_tracking_are_read_without_network_and_secrets_are_hidden() {
        let (_temp, context) = context();
        for args in [
            vec![
                "remote",
                "add",
                "origin",
                "https://user:secret@example.invalid/team/repo.git?token=secret",
            ],
            vec![
                "remote",
                "set-url",
                "--push",
                "origin",
                "ssh://git@example.invalid/team/repo.git",
            ],
            vec!["remote", "add", "backup", "https://backup.invalid/repo.git"],
            vec!["config", "branch.main.remote", "origin"],
            vec!["config", "branch.main.merge", "refs/heads/main"],
            vec!["symbolic-ref", "HEAD", "refs/heads/main"],
        ] {
            assert!(
                context
                    .command()
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        let info = context.remote_info().unwrap();
        assert_eq!(info.remotes.len(), 2);
        let origin = info
            .remotes
            .iter()
            .find(|remote| remote.name == "origin")
            .unwrap();
        assert_eq!(
            origin.push_urls,
            vec!["ssh://git@example.invalid/team/repo.git"]
        );
        assert_eq!(info.upstream.as_deref(), Some("origin/main"));
        assert_eq!(
            display_url(&origin.fetch_urls[0]),
            "https://example.invalid/team/repo.git"
        );
    }
}
