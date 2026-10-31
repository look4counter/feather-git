use anyhow::{Context, Result, bail};
use feather_git::native_git::Command;
use std::path::PathBuf;

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
        let mut command = Command::at(
            self.repository
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new(".")),
        );
        command.env("GIT_TERMINAL_PROMPT", "0");
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
        }
        let result = (|| {
            self.set_values(scope, "user.name", std::slice::from_ref(&identity.name))?;
            self.set_values(scope, "user.email", std::slice::from_ref(&identity.email))?;
            let saved = if global {
                self.identity(scope)?
            } else {
                self.effective_identity()?
            };
            if saved != identity {
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

    pub fn use_global_identity(&self) -> Result<()> {
        if self.repository.is_none() {
            bail!("저장소를 연 뒤 전역 설정 사용을 적용하세요.");
        }
        let global = self.identity("--global")?;
        if global.name.is_empty() || global.email.is_empty() {
            bail!("전역 이름과 이메일을 먼저 설정하세요.");
        }
        let before = [
            ("user.name", self.values(Some("--local"), "user.name")?),
            ("user.email", self.values(Some("--local"), "user.email")?),
        ];
        let result = (|| {
            for (key, _) in &before {
                self.set_values("--local", key, &[])?;
            }
            if self.effective_identity()? != global {
                bail!("다른 Git 설정이 전역 작성자 정보를 덮어쓰고 있습니다.");
            }
            Ok(())
        })();
        if let Err(error) = result {
            let restored = before.iter().fold(true, |restored, (key, values)| {
                let unchanged = self.values(Some("--local"), key).ok().as_ref() == Some(values);
                (unchanged || self.set_values("--local", key, values).is_ok()) && restored
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
        Ok(SettingsSnapshot {
            global,
            effective: self.effective_identity()?,
            use_global: self.repository.is_none()
                || (self.values(Some("--local"), "user.name")?.is_empty()
                    && self.values(Some("--local"), "user.email")?.is_empty()),
            info: self.remote_info()?,
        })
    }

    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        let path = self
            .repository
            .as_deref()
            .context("저장소를 먼저 열어 주세요.")?;
        feather_git::native_git::add_remote(path, name, url)
    }

    pub fn remove_remote(&self, name: &str) -> Result<()> {
        if self.repository.is_none() {
            bail!("저장소를 먼저 열어 주세요.");
        }
        let output = self.command().args(["remote", "remove", name]).output()?;
        if !output.status.success() {
            bail!("원격 저장소를 삭제하지 못했습니다. 설정 잠금과 원격 이름을 확인하세요.");
        }
        Ok(())
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
        Ok(RepositoryInfo { remotes })
    }

    #[cfg(test)]
    fn oracle_command(&self) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        if let Some(path) = &self.repository {
            command.current_dir(path);
        }
        if let Some(path) = &self.global_file {
            command
                .env("GIT_CONFIG_GLOBAL", path)
                .env("GIT_CONFIG_NOSYSTEM", "1");
        }
        command
    }

    #[cfg(test)]
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
    fn removing_remote_preserves_other_remotes_and_local_work() {
        let (_temp, context) = context();
        context
            .add_remote("origin", "https://example.invalid/origin.git")
            .unwrap();
        context
            .add_remote("backup", "https://example.invalid/backup.git")
            .unwrap();
        context
            .set_values("--local", "user.name", &["작성자".into()])
            .unwrap();
        context
            .set_values("--local", "branch.main.remote", &["origin".into()])
            .unwrap();
        context
            .set_values("--local", "branch.main.merge", &["refs/heads/main".into()])
            .unwrap();
        let path = context.repository.as_ref().unwrap();
        let file = path.join("draft.txt");
        std::fs::write(&file, "초안").unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let tree_id = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("테스트", "test@example.invalid").unwrap();
        let commit = repo
            .commit(
                Some("refs/heads/main"),
                &signature,
                &signature,
                "초기",
                &tree,
                &[],
            )
            .unwrap();
        for name in [
            "refs/heads/feature",
            "refs/remotes/origin/main",
            "refs/remotes/backup/main",
        ] {
            repo.reference(name, commit, false, "검증").unwrap();
        }
        let config = path.join(".git/config");
        let before = std::fs::read(&config).unwrap();
        let lock = path.join(".git/config.lock");
        std::fs::write(&lock, "locked").unwrap();
        assert!(context.remove_remote("origin").is_err());
        assert_eq!(std::fs::read(&config).unwrap(), before);
        std::fs::remove_file(lock).unwrap();
        context.remove_remote("origin").unwrap();
        assert!(repo.find_reference("refs/remotes/origin/main").is_err());
        for name in [
            "refs/heads/main",
            "refs/heads/feature",
            "refs/remotes/backup/main",
        ] {
            assert_eq!(repo.find_reference(name).unwrap().target(), Some(commit));
        }
        let info = context.remote_info().unwrap();
        assert_eq!(info.remotes.len(), 1);
        assert_eq!(info.remotes[0].name, "backup");
        assert_eq!(std::fs::read_to_string(file).unwrap(), "초안");
        assert_eq!(
            context.values(Some("--local"), "user.name").unwrap(),
            vec!["작성자"]
        );
        assert!(
            context
                .values(Some("--local"), "branch.main.remote")
                .unwrap()
                .is_empty()
        );
        assert!(context.remove_remote("origin").is_err());
        context
            .add_remote("origin", "https://example.invalid/again.git")
            .unwrap();
        assert_eq!(context.remote_info().unwrap().remotes.len(), 2);
        assert!(GitContext::new(None).remove_remote("origin").is_err());
    }

    #[test]
    fn adding_remotes_is_atomic_and_preserves_existing_configuration() {
        let (_temp, context) = context();
        context
            .set_values("--local", "user.name", &["작성자".into()])
            .unwrap();
        context
            .add_remote("origin", "https://example.invalid/main.git")
            .unwrap();
        context
            .add_remote("backup", "git@example.invalid:backup.git")
            .unwrap();
        assert_eq!(
            context
                .values(Some("--local"), "remote.backup.url")
                .unwrap(),
            vec!["git@example.invalid:backup.git"]
        );
        assert_eq!(context.remote_info().unwrap().remotes.len(), 2);
        assert_eq!(
            context
                .values(Some("--local"), "remote.origin.fetch")
                .unwrap(),
            vec!["+refs/heads/*:refs/remotes/origin/*"]
        );
        let config = context.repository.as_ref().unwrap().join(".git/config");
        let before = std::fs::read(&config).unwrap();
        for (name, url) in [
            ("origin", "other"),
            ("", "url"),
            ("bad..name", "url"),
            ("new", ""),
            ("new", "url\nvalue"),
        ] {
            assert!(context.add_remote(name, url).is_err());
            assert_eq!(std::fs::read(&config).unwrap(), before);
        }
        let lock = config.with_extension("lock");
        std::fs::write(&lock, "locked").unwrap();
        assert!(
            context
                .add_remote("new", "https://example.invalid/new.git")
                .is_err()
        );
        assert_eq!(std::fs::read(&config).unwrap(), before);
        std::fs::remove_file(lock).unwrap();
        context
            .add_remote("new", "https://example.invalid/new.git?q=one;two#part")
            .unwrap();
        assert_eq!(
            context.values(Some("--local"), "remote.new.url").unwrap(),
            vec!["https://example.invalid/new.git?q=one;two#part"]
        );
        assert_eq!(
            context.values(Some("--local"), "user.name").unwrap(),
            vec!["작성자"]
        );
        assert!(GitContext::new(None).add_remote("origin", "url").is_err());
    }

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
        let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let repo = temp.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        let context = GitContext {
            repository: Some(repo),
            global_file: Some(temp.path().join("global.config")),
        };
        assert!(
            context
                .oracle_command()
                .arg("init")
                .output()
                .unwrap()
                .status
                .success()
        );
        (temp, context)
    }

    #[test]
    fn using_global_removes_only_local_identity_and_preserves_global_and_other_settings() {
        let (_temp, context) = context();
        let global = Identity {
            name: "전역".into(),
            email: "global@example.invalid".into(),
        };
        let local = Identity {
            name: "개별".into(),
            email: "local@example.invalid".into(),
        };
        context.save_identity(&global, true).unwrap();
        context.save_identity(&local, false).unwrap();
        context
            .set_values("--local", "core.autocrlf", &["false".into()])
            .unwrap();
        assert!(!context.load().unwrap().use_global);
        context.use_global_identity().unwrap();
        assert_eq!(context.identity("--global").unwrap(), global);
        assert_eq!(context.identity("--local").unwrap(), Identity::default());
        assert_eq!(context.effective_identity().unwrap(), global);
        assert!(context.load().unwrap().use_global);
        assert_eq!(
            context.values(Some("--local"), "core.autocrlf").unwrap(),
            vec!["false"]
        );
        context.save_identity(&local, false).unwrap();
        std::fs::write(
            context
                .repository
                .as_ref()
                .unwrap()
                .join(".git/config.lock"),
            "locked",
        )
        .unwrap();
        assert!(context.use_global_identity().is_err());
        assert_eq!(context.identity("--local").unwrap(), local);
        assert_eq!(context.identity("--global").unwrap(), global);
    }

    #[test]
    fn using_global_rejects_missing_global_identity_and_missing_repository_without_writes() {
        let (_temp, mut context) = context();
        let local = Identity {
            name: "개별".into(),
            email: "local@example.invalid".into(),
        };
        context.save_identity(&local, false).unwrap();
        assert!(context.use_global_identity().is_err());
        assert_eq!(context.identity("--local").unwrap(), local);
        assert_eq!(context.identity("--global").unwrap(), Identity::default());
        context.repository = None;
        assert!(context.use_global_identity().is_err());
        assert!(context.load().unwrap().use_global);
    }

    #[test]
    fn global_and_local_saves_preserve_the_other_scope_and_reads_do_not_write() {
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
        let updated_global = Identity {
            name: "새 전역 사용자".into(),
            email: "new-global@example.invalid".into(),
        };
        context.save_identity(&updated_global, true).unwrap();
        assert_eq!(context.identity("--local").unwrap(), local);
        assert_eq!(context.effective_identity().unwrap(), local);
        assert_eq!(context.identity("--global").unwrap(), updated_global);
        let snapshot = context.load().unwrap();
        assert_eq!(snapshot.global, updated_global);
        assert_eq!(snapshot.effective, local);
        assert_eq!(context.identity("--local").unwrap(), local);
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
        let changed = Identity {
            name: "저장소 없는 전역 사용자".into(),
            email: "standalone@example.invalid".into(),
        };
        context.save_identity(&changed, true).unwrap();
        assert_eq!(context.load().unwrap().effective, changed);
    }

    #[test]
    fn config_write_failure_restores_identity() {
        let (_temp, context) = context();
        let before = Identity {
            name: "before".into(),
            email: "before@example.invalid".into(),
        };
        context.save_identity(&before, true).unwrap();
        // 실제 전역 파일 대신 격리된 전역 설정 파일의 잠금을 사용한다.
        std::fs::write(
            context
                .global_file
                .as_ref()
                .unwrap()
                .with_extension("config.lock"),
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
                    .oracle_command()
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
        assert_eq!(
            context
                .values(Some("--local"), "branch.main.remote")
                .unwrap(),
            vec!["origin"]
        );
        assert_eq!(
            display_url(&origin.fetch_urls[0]),
            "https://example.invalid/team/repo.git"
        );
    }
}
