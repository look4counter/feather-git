use crate::settings_git::GitContext;
use anyhow::{Context, Result, bail};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub struct LoginTarget {
    host: String,
    path: String,
    username: Option<String>,
}

impl LoginTarget {
    pub fn parse(value: &str) -> Result<Self> {
        if value.chars().any(|c| c.is_control() || c.is_whitespace()) {
            bail!("공백과 줄바꿈이 없는 HTTPS 주소를 입력하세요.");
        }
        let url = url::Url::parse(value).context("로그인할 HTTPS 주소를 입력하세요.")?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || value.to_ascii_lowercase().contains("%0a")
            || value.to_ascii_lowercase().contains("%0d")
        {
            bail!(
                "사용자정보·쿼리가 없는 HTTPS 주소를 사용하세요. SSH 인증은 SSH 키로 관리합니다."
            );
        }
        let host = format!(
            "{}{}",
            url.host_str().unwrap(),
            url.port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default()
        );
        Ok(Self {
            host,
            path: url.path().trim_start_matches('/').to_owned(),
            username: None,
        })
    }

    fn input(&self) -> Vec<u8> {
        format!(
            "protocol=https\nhost={}\npath={}\n{}\n",
            self.host,
            self.path,
            self.username
                .as_ref()
                .map(|name| format!("username={name}\n"))
                .unwrap_or_default()
        )
        .into_bytes()
    }

    pub fn for_account(mut self, username: &str) -> Result<Self> {
        if username.chars().any(|c| c.is_control()) {
            bail!("계정 ID에 제어문자를 사용할 수 없습니다.");
        }
        self.username = (!username.trim().is_empty()).then(|| username.trim().to_owned());
        Ok(self)
    }
}

pub trait CredentialRunner {
    fn run(
        &self,
        action: &str,
        input: &[u8],
        interactive: bool,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>>;
}

#[derive(Clone)]
pub struct Gcm {
    command: String,
    context: GitContext,
}

impl Gcm {
    pub fn detect(context: &GitContext) -> Option<Self> {
        for name in feather_git::native_git::credential_helpers() {
            let mut command = helper_command(context, &name);
            command.arg("--version");
            if run_process(
                command,
                &[],
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
            .is_ok()
            {
                return Some(Self {
                    command: name.to_string_lossy().into_owned(),
                    context: context.clone(),
                });
            }
        }
        None
    }
}

impl CredentialRunner for Gcm {
    fn run(
        &self,
        action: &str,
        input: &[u8],
        interactive: bool,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>> {
        if !["get", "store", "erase"].contains(&action) {
            bail!("지원하지 않는 인증 작업입니다.");
        }
        let mut command = helper_command(&self.context, std::path::Path::new(&self.command));
        command
            .arg(action)
            .env("GIT_CONFIG_COUNT", "4")
            .env("GIT_CONFIG_KEY_0", "credential.trace")
            .env("GIT_CONFIG_VALUE_0", "false")
            .env("GIT_CONFIG_KEY_1", "credential.traceSecrets")
            .env("GIT_CONFIG_VALUE_1", "false")
            .env("GIT_CONFIG_KEY_2", "credential.msauthFlow")
            .env("GIT_CONFIG_VALUE_2", "system")
            .env("GIT_CONFIG_KEY_3", "credential.msauthUseBroker")
            .env("GIT_CONFIG_VALUE_3", "false");
        command
            .env(
                "GCM_INTERACTIVE",
                if interactive { "always" } else { "never" },
            )
            .env("GCM_GUI_PROMPT", if interactive { "1" } else { "0" })
            .env("GCM_GITHUB_AUTHMODES", "browser")
            .env("GCM_GITLAB_AUTHMODES", "browser")
            .env("GCM_BITBUCKET_AUTHMODES", "oauth")
            .env("GCM_TRACE", "0")
            .env("GCM_TRACE_SECRETS", "0")
            .env("GIT_TRACE", "0")
            .env("GIT_TRACE_CURL", "0")
            .env("GIT_CURL_VERBOSE", "0");
        if interactive {
            command.env("GCM_GITHUB_ALWAYS_REFRESH_CREDENTIALS", "true");
        }
        run_process(
            command,
            input,
            Duration::from_secs(if interactive { 180 } else { 20 }),
            cancel,
        )
    }
}

fn helper_command(context: &GitContext, executable: &std::path::Path) -> Command {
    let mut command = Command::new(executable);
    if let Some(path) = &context.repository {
        command.current_dir(path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn run_process(
    mut command: Command,
    input: &[u8],
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    if cancel.load(Ordering::Relaxed) {
        bail!("작업을 취소했습니다.");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Git Credential Manager를 실행할 수 없습니다.")?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = vec![];
        let _ = stdout.by_ref().take(1024 * 1024).read_to_end(&mut bytes);
        bytes
    });
    let err = std::thread::spawn(move || {
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
    });
    let written = child.stdin.take().unwrap().write_all(input);
    let started = Instant::now();
    let result = loop {
        if cancel.load(Ordering::Relaxed) || started.elapsed() >= timeout || written.is_err() {
            terminate(&mut child);
            let _ = child.wait();
            break Err(anyhow::anyhow!(if cancel.load(Ordering::Relaxed) {
                "작업을 취소했습니다."
            } else {
                "인증이 완료되지 않았습니다. 취소 또는 시간 초과 여부를 확인하세요."
            }));
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(error) => {
                terminate(&mut child);
                let _ = child.wait();
                break Err(error.into());
            }
        }
    };
    let mut bytes = out.join().unwrap_or_default();
    let _ = err.join();
    if result.as_ref().is_ok_and(|status| status.success()) {
        return Ok(bytes);
    }
    bytes.fill(0);
    match result {
        Err(error) => Err(error),
        Ok(_) => bail!(
            "GCM 작업을 완료하지 못했습니다. 로그인 취소, 서비스 지원, GCM 설치 상태를 확인하세요."
        ),
    }
}

fn terminate(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Git이 시작한 GCM도 함께 종료해 브라우저 인증 대기가 남지 않게 한다.
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

pub fn authenticate(
    runner: &impl CredentialRunner,
    target: &LoginTarget,
    interactive: bool,
    cancel: &AtomicBool,
) -> Result<String> {
    let mut response = runner.run("get", &target.input(), interactive, cancel)?;
    let result = (|| {
        let field = |key: &[u8]| {
            response
                .split(|byte| *byte == b'\n')
                .find_map(|line| line.strip_suffix(b"\r").unwrap_or(line).strip_prefix(key))
        };
        let username = field(b"username=").context("저장된 로그인 정보가 없습니다.")?;
        let password = field(b"password=")
            .filter(|value| !value.is_empty())
            .context("저장된 로그인 정보가 없습니다.")?;
        // 토큰을 계정명으로 잘못 반환하는 공급자는 상세 이름을 표시하지 않는다.
        let account = if username.len() > 100
            || username.is_empty()
            || username == password
            || username.iter().any(u8::is_ascii_control)
        {
            "연결된 계정".to_owned()
        } else {
            String::from_utf8_lossy(username).into_owned()
        };
        if interactive {
            let mut input = target.input();
            input.truncate(input.len() - 1);
            input.extend_from_slice(b"username=");
            input.extend_from_slice(username);
            input.extend_from_slice(b"\npassword=");
            input.extend_from_slice(password);
            input.extend_from_slice(b"\n\n");
            let stored = runner.run("store", &input, false, cancel);
            input.fill(0);
            stored?;
        }
        Ok(account)
    })();
    response.fill(0);
    result
}

pub fn logout(
    runner: &impl CredentialRunner,
    target: &LoginTarget,
    cancel: &AtomicBool,
) -> Result<()> {
    runner
        .run("erase", &target.input(), false, cancel)
        .map(|_| ())
}

pub fn logout_profile(
    runner: Option<&impl CredentialRunner>,
    profile: &feather_git::auth_profiles::AuthProfile,
    store: &dyn feather_git::auth_profiles::ExistingCredentialStore,
    cancel: &AtomicBool,
) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        bail!("로그아웃을 취소했습니다.");
    }
    let helper_result = if let Some(runner) = runner
        && !profile.username.is_empty()
        && profile.username != "저장된 계정"
    {
        let target = LoginTarget::parse(&profile.url)?.for_account(&profile.username)?;
        Some(logout(runner, &target, cancel))
    } else {
        None
    };
    if cancel.load(Ordering::Relaxed) {
        bail!("로그아웃을 취소했습니다.");
    }
    let removed = feather_git::auth_profiles::erase_existing_for(profile, store)?;
    if removed > 0
        || profile.imported
        || helper_result.as_ref().is_some_and(|result| result.is_ok())
    {
        return Ok(());
    }
    if let Some(Err(error)) = helper_result {
        return Err(error);
    }
    bail!("GCM 도구나 제거할 저장 인증을 확인하지 못했습니다. 앱 인증 등록은 유지합니다.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn malformed_credentials_and_store_failure_never_report_login_success() {
        struct BrokenGcm {
            malformed: bool,
        }
        impl CredentialRunner for BrokenGcm {
            fn run(&self, action: &str, _: &[u8], _: bool, _: &AtomicBool) -> Result<Vec<u8>> {
                if action == "store" {
                    bail!("저장소 사용 불가");
                }
                Ok(if self.malformed {
                    b"username=account\n\n".to_vec()
                } else {
                    b"username=account\r\npassword=secret-token\r\n\r\n".to_vec()
                })
            }
        }
        let target = LoginTarget::parse("https://example.invalid").unwrap();
        let cancel = AtomicBool::new(false);
        for malformed in [true, false] {
            let error = authenticate(&BrokenGcm { malformed }, &target, true, &cancel)
                .unwrap_err()
                .to_string();
            assert!(!error.contains("secret-token"));
        }
        assert_eq!(
            authenticate(&BrokenGcm { malformed: false }, &target, false, &cancel).unwrap(),
            "account"
        );
    }

    #[test]
    fn process_runner_enforces_timeout_and_cancellation() {
        let mut command = Command::new("git");
        command.arg("--version");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        assert!(run_process(command, &[], Duration::ZERO, &AtomicBool::new(false)).is_err());
        assert!(
            run_process(
                Command::new("not-started"),
                &[],
                Duration::from_secs(1),
                &AtomicBool::new(true)
            )
            .is_err()
        );
    }

    #[derive(Default)]
    struct FakeGcm {
        calls: Mutex<Vec<(String, Vec<u8>, bool)>>,
    }

    struct ExistingStore {
        entries: Mutex<Vec<feather_git::auth_profiles::ExistingCredential>>,
        ignore_delete: bool,
        fail_delete: bool,
    }
    impl feather_git::auth_profiles::ExistingCredentialStore for ExistingStore {
        fn entries(&self) -> Result<Vec<feather_git::auth_profiles::ExistingCredential>> {
            Ok(self.entries.lock().unwrap().clone())
        }
        fn erase(&self, target: &str) -> Result<()> {
            if self.fail_delete {
                bail!("권한 오류");
            }
            if !self.ignore_delete {
                self.entries
                    .lock()
                    .unwrap()
                    .retain(|entry| entry.target != target);
            }
            Ok(())
        }
    }
    fn windows_profile() -> feather_git::auth_profiles::AuthProfile {
        feather_git::auth_profiles::AuthProfile {
            id: "existing".into(),
            url: "https://github.com".into(),
            username: "alice".into(),
            method: feather_git::auth_profiles::AuthMethod::Gcm,
            imported: true,
            hidden: false,
        }
    }
    fn windows_entries() -> Vec<feather_git::auth_profiles::ExistingCredential> {
        vec![
            feather_git::auth_profiles::ExistingCredential {
                target: "git:https://alice@github.com".into(),
                username: "alice".into(),
            },
            feather_git::auth_profiles::ExistingCredential {
                target: "git:https://bob@github.com".into(),
                username: "bob".into(),
            },
        ]
    }

    #[test]
    fn helper_noop_success_still_removes_exact_windows_account_and_supports_no_helper() {
        let runner = FakeGcm::default();
        let profile = windows_profile();
        let store = ExistingStore {
            entries: Mutex::new(windows_entries()),
            ignore_delete: false,
            fail_delete: false,
        };
        logout_profile(Some(&runner), &profile, &store, &AtomicBool::new(false)).unwrap();
        assert_eq!(store.entries.lock().unwrap().len(), 1);
        assert_eq!(store.entries.lock().unwrap()[0].username, "bob");
        assert_eq!(runner.calls.lock().unwrap()[0].0, "erase");
        let store = ExistingStore {
            entries: Mutex::new(windows_entries()),
            ignore_delete: false,
            fail_delete: false,
        };
        logout_profile(None::<&FakeGcm>, &profile, &store, &AtomicBool::new(false)).unwrap();
        assert_eq!(store.entries.lock().unwrap().len(), 1);
        struct FailedHelper;
        impl CredentialRunner for FailedHelper {
            fn run(&self, _: &str, _: &[u8], _: bool, _: &AtomicBool) -> Result<Vec<u8>> {
                bail!("GCM 실행 실패");
            }
        }
        let store = ExistingStore {
            entries: Mutex::new(windows_entries()),
            ignore_delete: false,
            fail_delete: false,
        };
        logout_profile(
            Some(&FailedHelper),
            &profile,
            &store,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(store.entries.lock().unwrap().len(), 1);
        let mut unknown = profile.clone();
        unknown.username = "저장된 계정".into();
        let store = ExistingStore {
            entries: Mutex::new(vec![
                feather_git::auth_profiles::ExistingCredential {
                    target: "git:https://github.com".into(),
                    username: String::new(),
                },
                feather_git::auth_profiles::ExistingCredential {
                    target: "git:https://bob@github.com".into(),
                    username: "bob".into(),
                },
            ]),
            ignore_delete: false,
            fail_delete: false,
        };
        logout_profile(None::<&FakeGcm>, &unknown, &store, &AtomicBool::new(false)).unwrap();
        assert_eq!(store.entries.lock().unwrap()[0].username, "bob");
    }

    #[test]
    fn helper_success_does_not_mask_windows_delete_failure_or_remaining_account() {
        let runner = FakeGcm::default();
        for (ignore_delete, fail_delete) in [(true, false), (false, true)] {
            let store = ExistingStore {
                entries: Mutex::new(windows_entries()),
                ignore_delete,
                fail_delete,
            };
            assert!(
                logout_profile(
                    Some(&runner),
                    &windows_profile(),
                    &store,
                    &AtomicBool::new(false)
                )
                .is_err()
            );
            assert_eq!(store.entries.lock().unwrap().len(), 2);
        }
    }

    #[test]
    fn logout_erases_only_the_requested_account_without_reading_or_sending_secrets() {
        let runner = FakeGcm::default();
        let target = LoginTarget::parse("https://github.com/team/repo.git")
            .unwrap()
            .for_account("alice")
            .unwrap();
        logout(&runner, &target, &AtomicBool::new(false)).unwrap();
        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "erase");
        assert!(!calls[0].2);
        let request = std::str::from_utf8(&calls[0].1).unwrap();
        for field in [
            "protocol=https\n",
            "host=github.com\n",
            "path=team/repo.git\n",
            "username=alice\n",
        ] {
            assert!(request.contains(field));
        }
        assert!(!request.contains("password="));
    }
    impl CredentialRunner for FakeGcm {
        fn run(
            &self,
            action: &str,
            input: &[u8],
            interactive: bool,
            cancel: &AtomicBool,
        ) -> Result<Vec<u8>> {
            if cancel.load(Ordering::Relaxed) {
                bail!("인증이 취소되었습니다.");
            }
            self.calls
                .lock()
                .unwrap()
                .push((action.into(), input.to_vec(), interactive));
            Ok(if action == "get" {
                b"username=account\npassword=secret-token\n\n".to_vec()
            } else {
                vec![]
            })
        }
    }

    #[test]
    fn https_targets_reject_protocol_injection_secrets_and_ssh() {
        for invalid in [
            "git@github.com:team/repo.git",
            "http://example.invalid",
            "https://u:secret@example.invalid",
            "https://example.invalid?token=secret",
            "https://example.invalid\nhost=other",
            "https://example.invalid/%0a",
        ] {
            assert!(LoginTarget::parse(invalid).is_err(), "{invalid}");
        }
        let target = LoginTarget::parse("https://example.invalid:8443/team/repo.git").unwrap();
        assert!(
            String::from_utf8(target.input())
                .unwrap()
                .contains("host=example.invalid:8443")
        );
        let target = LoginTarget::parse("https://github.com/team/repo")
            .unwrap()
            .for_account("account")
            .unwrap();
        assert!(
            String::from_utf8(target.input())
                .unwrap()
                .contains("username=account\n")
        );
        assert!(
            LoginTarget::parse("https://github.com")
                .unwrap()
                .for_account("account\npassword=secret")
                .is_err()
        );
    }

    #[test]
    fn login_stores_in_gcm_and_returns_only_account_then_logout_erases() {
        let runner = FakeGcm::default();
        let target = LoginTarget::parse("https://example.invalid/team/repo.git").unwrap();
        let cancel = AtomicBool::new(false);
        assert_eq!(
            authenticate(&runner, &target, true, &cancel).unwrap(),
            "account"
        );
        logout(&runner, &target, &cancel).unwrap();
        let calls = runner.calls.lock().unwrap();
        assert_eq!(
            calls.iter().map(|call| call.0.as_str()).collect::<Vec<_>>(),
            vec!["get", "store", "erase"]
        );
        assert!(calls[0].2);
        assert!(
            calls[1]
                .1
                .windows(b"password=secret-token".len())
                .any(|part| part == b"password=secret-token")
        );
        assert!(
            !calls[2]
                .1
                .windows(b"secret-token".len())
                .any(|part| part == b"secret-token")
        );
    }

    #[test]
    fn status_is_noninteractive_and_cancellation_prevents_authentication() {
        let runner = FakeGcm::default();
        let target = LoginTarget::parse("https://example.invalid").unwrap();
        let cancel = AtomicBool::new(false);
        assert_eq!(
            authenticate(&runner, &target, false, &cancel).unwrap(),
            "account"
        );
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
        assert!(!runner.calls.lock().unwrap()[0].2);
        cancel.store(true, Ordering::Relaxed);
        assert!(authenticate(&runner, &target, true, &cancel).is_err());
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
    }
}
