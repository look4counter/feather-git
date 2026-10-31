use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMethod {
    Gcm,
    BrowserGcm,
    BrowserOAuth,
    Password,
}
impl AuthMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Gcm => "GCM",
            Self::BrowserGcm => "브라우저 로그인 · GCM",
            Self::BrowserOAuth => "브라우저 로그인 · 독립 OAuth",
            Self::Password => "ID와 비밀번호",
        }
    }
    pub fn uses_vault(self) -> bool {
        matches!(self, Self::BrowserOAuth | Self::Password)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthProfile {
    pub id: String,
    pub url: String,
    pub username: String,
    pub method: AuthMethod,
    #[serde(default, skip_serializing_if = "is_false")]
    pub imported: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}
fn same_url(a: &str, b: &str) -> bool {
    normalize_url(a)
        .ok()
        .zip(normalize_url(b).ok())
        .is_some_and(|(a, b)| a == b)
}

fn visible(profiles: &[AuthProfile]) -> Vec<AuthProfile> {
    profiles
        .iter()
        .filter(|profile| {
            !profile.hidden
                && (!profile.imported
                    || !profiles.iter().any(|other| {
                        !other.imported
                            && !other.hidden
                            && same_url(&other.url, &profile.url)
                            && other.username == profile.username
                    }))
        })
        .cloned()
        .collect()
}

#[derive(Clone)]
pub struct ExistingCredential {
    pub target: String,
    pub username: String,
}

pub trait ExistingCredentialStore {
    fn entries(&self) -> Result<Vec<ExistingCredential>>;
    fn erase(&self, target: &str) -> Result<()>;
}
pub struct SystemExistingCredentials;
impl ExistingCredentialStore for SystemExistingCredentials {
    fn entries(&self) -> Result<Vec<ExistingCredential>> {
        existing_credentials()
    }
    fn erase(&self, target: &str) -> Result<()> {
        #[cfg(windows)]
        {
            use windows::{
                Win32::{Foundation::ERROR_NOT_FOUND, Security::Credentials::*},
                core::PCWSTR,
            };
            let target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
            match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
                Ok(()) => Ok(()),
                Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
                Err(_) => bail!("Windows에 저장된 해당 Git 인증을 제거하지 못했습니다."),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = target;
            bail!("이 OS에서는 Windows Git 인증 제거를 지원하지 않습니다.");
        }
    }
}

fn matches_existing(profile: &AuthProfile, credential: &ExistingCredential) -> bool {
    let Some(existing) = existing_profile(credential) else {
        return false;
    };
    let Ok(scope) = normalize_url(&profile.url) else {
        return false;
    };
    if existing.url != scope {
        return false;
    }
    let github = url::Url::parse(&scope).is_ok_and(|url| url.host_str() == Some("github.com"));
    if github {
        existing.username.eq_ignore_ascii_case(&profile.username)
    } else {
        existing.username == profile.username
    }
}

pub fn erase_existing_for(
    profile: &AuthProfile,
    store: &dyn ExistingCredentialStore,
) -> Result<usize> {
    let matching = store
        .entries()?
        .into_iter()
        .filter(|entry| matches_existing(profile, entry))
        .collect::<Vec<_>>();
    if matching.len() > 1
        && ["저장된 계정", "PersonalAccessToken", "oauth2", "OAuth2"]
            .contains(&profile.username.as_str())
    {
        bail!(
            "계정 정보가 불명확한 저장 인증이 여러 개라 안전하게 로그아웃할 수 없습니다. Windows 자격 증명 관리자에서 대상을 확인하세요."
        );
    }
    for (index, entry) in matching.iter().enumerate() {
        if store.erase(&entry.target).is_err() {
            if index > 0 {
                bail!(
                    "일부 Git 인증은 제거했지만 나머지 항목을 지우지 못했습니다. 앱 등록은 유지합니다."
                );
            }
            bail!("Windows에 저장된 해당 Git 인증을 제거하지 못했습니다. 앱 등록은 유지합니다.");
        }
    }
    if store
        .entries()?
        .iter()
        .any(|entry| matches_existing(profile, entry))
    {
        bail!(
            "해당 Git 인증이 Windows 저장소에 아직 남아 있습니다. 로그아웃을 완료하지 못했습니다."
        );
    }
    Ok(matching.len())
}

fn existing_profile(credential: &ExistingCredential) -> Option<AuthProfile> {
    let mut parsed = url::Url::parse(credential.target.strip_prefix("git:")?).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    let account = if credential.username.trim().is_empty() {
        parsed.username()
    } else {
        credential.username.trim()
    };
    let username = if account.is_empty()
        || account.len() > 100
        || account.chars().any(char::is_control)
        || [
            "ghp_",
            "github_pat_",
            "gho_",
            "ghu_",
            "ghs_",
            "ghr_",
            "glpat-",
        ]
        .iter()
        .any(|prefix| account.starts_with(prefix))
    {
        "저장된 계정".to_owned()
    } else {
        account.to_owned()
    };
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    let url = normalize_url(parsed.as_str()).ok()?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in url.bytes().chain([0]).chain(username.bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    Some(AuthProfile {
        id: format!("gcm-{hash:016x}"),
        url,
        username,
        method: AuthMethod::Gcm,
        imported: true,
        hidden: false,
    })
}

#[cfg(windows)]
fn existing_credentials() -> Result<Vec<ExistingCredential>> {
    use windows::{
        Win32::{Foundation::ERROR_NOT_FOUND, Security::Credentials::*},
        core::PCWSTR,
    };
    let filter: Vec<u16> = "git:*\0".encode_utf16().collect();
    let mut count = 0;
    let mut credentials = std::ptr::null_mut();
    unsafe {
        match CredEnumerateW(PCWSTR(filter.as_ptr()), None, &mut count, &mut credentials) {
            Ok(()) => {}
            Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => return Ok(vec![]),
            Err(_) => bail!(
                "기존 Windows Git/GCM 인증 정보를 읽지 못했습니다. 앱 등록 목록만 표시합니다."
            ),
        }
        let mut result = Vec::new();
        if count > 0 {
            for pointer in std::slice::from_raw_parts(credentials, count as usize) {
                let credential = &**pointer;
                if credential.Type != CRED_TYPE_GENERIC {
                    continue;
                }
                // Only metadata is copied. CredentialBlob is never accessed.
                let target = PCWSTR(credential.TargetName.0)
                    .to_string()
                    .unwrap_or_default();
                let username = if credential.UserName.is_null() {
                    String::new()
                } else {
                    PCWSTR(credential.UserName.0)
                        .to_string()
                        .unwrap_or_default()
                };
                result.push(ExistingCredential { target, username });
            }
        }
        CredFree(credentials.cast());
        Ok(result)
    }
}
#[cfg(not(windows))]
fn existing_credentials() -> Result<Vec<ExistingCredential>> {
    Ok(vec![])
}

pub fn merge_existing_at(
    path: &Path,
    credentials: &[ExistingCredential],
) -> Result<Vec<AuthProfile>> {
    let _lock = locked(path)?;
    let mut profiles = load_from(path)?;
    let before = profiles.clone();
    let imports = credentials
        .iter()
        .filter_map(existing_profile)
        .collect::<Vec<_>>();
    profiles.retain(|profile| {
        !profile.imported || profile.hidden || imports.iter().any(|entry| entry.id == profile.id)
    });
    for profile in imports {
        if !profiles.iter().any(|entry| entry.id == profile.id) {
            profiles.push(profile);
        }
    }
    if profiles != before {
        persist(path, &profiles)?;
    }
    Ok(visible(&profiles))
}

pub fn load_with_existing() -> Result<(Vec<AuthProfile>, Option<String>)> {
    let current = load()?;
    let credentials = match existing_credentials() {
        Ok(credentials) => credentials,
        Err(error) => return Ok((current, Some(error.to_string()))),
    };
    match merge_existing_at(&store_path()?, &credentials) {
        Ok(profiles) => Ok((profiles, None)),
        Err(_) => Ok((
            current,
            Some("기존 인증 목록을 갱신하지 못했습니다. 앱 등록 목록을 유지합니다.".into()),
        )),
    }
}

pub struct Secret(String);
impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl Drop for Secret {
    fn drop(&mut self) {
        // Replacing every byte with NUL preserves valid UTF-8.
        unsafe {
            self.0.as_bytes_mut().fill(0);
        }
    }
}

pub trait SecretStore {
    fn read(&self, id: &str) -> Result<Secret>;
    fn write(&self, id: &str, username: &str, secret: &Secret) -> Result<()>;
    fn erase(&self, id: &str) -> Result<()>;
}
pub struct SystemSecrets;
#[derive(Debug)]
struct MissingSecret;
impl std::fmt::Display for MissingSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("저장된 인증 비밀이 없습니다.")
    }
}
impl std::error::Error for MissingSecret {}

#[cfg(windows)]
impl SecretStore for SystemSecrets {
    fn read(&self, id: &str) -> Result<Secret> {
        use windows::{
            Win32::{Foundation::ERROR_NOT_FOUND, Security::Credentials::*},
            core::PCWSTR,
        };
        let key: Vec<u16> = format!("FeatherGit/auth/{id}\0").encode_utf16().collect();
        let mut credential = std::ptr::null_mut();
        unsafe {
            match CredReadW(
                PCWSTR(key.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut credential,
            ) {
                Ok(()) => {}
                Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => {
                    return Err(MissingSecret.into());
                }
                Err(_) => bail!("저장된 인증 비밀을 읽을 수 없습니다."),
            }
            let value = &*credential;
            let bytes = if value.CredentialBlobSize == 0 {
                vec![]
            } else {
                std::slice::from_raw_parts(value.CredentialBlob, value.CredentialBlobSize as usize)
                    .to_vec()
            };
            CredFree(credential.cast());
            let secret = String::from_utf8(bytes)
                .map_err(|_| anyhow::anyhow!("인증 데이터 형식이 잘못되었습니다."))?;
            Ok(Secret::new(secret))
        }
    }
    fn write(&self, id: &str, username: &str, secret: &Secret) -> Result<()> {
        use windows::{Win32::Security::Credentials::*, core::PWSTR};
        if secret.expose().is_empty() || secret.expose().len() > 2560 {
            bail!("비밀번호 또는 토큰 길이를 확인하세요.");
        }
        let mut key: Vec<u16> = format!("FeatherGit/auth/{id}\0").encode_utf16().collect();
        let mut user: Vec<u16> = username.encode_utf16().chain(Some(0)).collect();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(key.as_mut_ptr()),
            UserName: PWSTR(user.as_mut_ptr()),
            CredentialBlobSize: secret.expose().len() as u32,
            CredentialBlob: secret.expose().as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        unsafe { CredWriteW(&credential, 0) }
            .map_err(|_| anyhow::anyhow!("Windows 자격 증명에 저장하지 못했습니다."))
    }
    fn erase(&self, id: &str) -> Result<()> {
        use windows::{
            Win32::{Foundation::ERROR_NOT_FOUND, Security::Credentials::*},
            core::PCWSTR,
        };
        let key: Vec<u16> = format!("FeatherGit/auth/{id}\0").encode_utf16().collect();
        match unsafe { CredDeleteW(PCWSTR(key.as_ptr()), CRED_TYPE_GENERIC, None) } {
            Ok(()) => Ok(()),
            Err(error) if error.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
            Err(_) => bail!("Windows 자격 증명을 삭제하지 못했습니다."),
        }
    }
}
#[cfg(not(windows))]
impl SecretStore for SystemSecrets {
    fn read(&self, _: &str) -> Result<Secret> {
        bail!("이 OS에서는 직접 인증 저장을 지원하지 않습니다.")
    }
    fn write(&self, _: &str, _: &str, _: &Secret) -> Result<()> {
        bail!("이 OS에서는 직접 인증 저장을 지원하지 않습니다.")
    }
    fn erase(&self, _: &str) -> Result<()> {
        bail!("이 OS에서는 직접 인증 저장을 지원하지 않습니다.")
    }
}

pub fn normalize_url(value: &str) -> Result<String> {
    if value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        bail!("공백 없는 HTTPS URL을 입력하세요.");
    }
    let url =
        url::Url::parse(value).map_err(|_| anyhow::anyhow!("올바른 HTTPS URL을 입력하세요."))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("사용자 정보·쿼리·프래그먼트 없는 HTTPS URL을 입력하세요.");
    }
    Ok(url.to_string())
}

pub fn credential_request(value: &str, username: Option<&str>) -> Result<Vec<u8>> {
    if value.chars().any(char::is_control)
        || username.is_some_and(|name| name.chars().any(char::is_control))
    {
        bail!("인증 요청에 제어문자를 사용할 수 없습니다.");
    }
    let url =
        url::Url::parse(value).map_err(|_| anyhow::anyhow!("올바른 인증 URL이 필요합니다."))?;
    if url.scheme() != "https" || url.host_str().is_none() {
        bail!("HTTPS 인증 주소가 필요합니다.");
    }
    let host = &url[url::Position::BeforeHost..url::Position::AfterPort];
    let account = username
        .filter(|name| !name.is_empty())
        .or_else(|| (!url.username().is_empty()).then(|| url.username()));
    Ok(format!(
        "protocol=https\nhost={host}\npath={}\n{}\n",
        url.path().trim_start_matches('/'),
        account
            .map(|name| format!("username={name}\n"))
            .unwrap_or_default()
    )
    .into_bytes())
}

pub fn store_path() -> Result<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_STATE_HOME"))
        .context("사용자 인증 목록 경로를 찾을 수 없습니다.")?;
    Ok(PathBuf::from(root).join("FeatherGit/auth.json"))
}

pub fn load_from(path: &Path) -> Result<Vec<AuthProfile>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("인증 목록 파일이 손상되었습니다."),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(_) => bail!("인증 목록을 읽을 수 없습니다."),
    }
}
pub fn load() -> Result<Vec<AuthProfile>> {
    Ok(visible(&load_from(&store_path()?)?))
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn locked(path: &Path) -> Result<Lock> {
    fs::create_dir_all(path.parent().context("인증 목록 경로가 잘못되었습니다.")?)?;
    let lock = path.with_extension("lock");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .context("인증 목록을 다른 작업이 변경 중입니다.")?;
    Ok(Lock(lock))
}
fn persist(path: &Path, profiles: &[AuthProfile]) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec(profiles)?)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub fn check_new(url: &str) -> Result<String> {
    let url = normalize_url(url)?;
    if load()?
        .iter()
        .any(|profile| !profile.imported && same_url(&profile.url, &url))
    {
        bail!("이 URL의 인증이 이미 등록되어 있습니다. 삭제 후 다시 추가하세요.");
    }
    Ok(url)
}
pub fn register_at(
    path: &Path,
    vault: &dyn SecretStore,
    url: &str,
    username: &str,
    method: AuthMethod,
    secret: Option<&Secret>,
) -> Result<Vec<AuthProfile>> {
    let url = normalize_url(url)?;
    if username.trim().is_empty() || username.chars().any(|c| c.is_control()) {
        bail!("계정 ID를 입력하세요.");
    }
    let _lock = locked(path)?;
    let mut profiles = load_from(path)?;
    if profiles
        .iter()
        .any(|p| !p.hidden && !p.imported && same_url(&p.url, &url))
    {
        bail!("이 URL의 인증이 이미 등록되어 있습니다.");
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let id = format!(
        "{:x}-{:x}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let profile = AuthProfile {
        id,
        url,
        username: username.trim().to_owned(),
        method,
        imported: false,
        hidden: false,
    };
    if method.uses_vault() {
        let secret = secret.context("비밀번호 또는 토큰을 입력하세요.")?;
        if secret.expose().is_empty() {
            bail!("비밀번호 또는 토큰을 입력하세요.");
        }
        if secret.expose().chars().any(char::is_control) {
            bail!("비밀번호 또는 토큰에 제어문자를 사용할 수 없습니다.");
        }
        vault.write(&profile.id, &profile.username, secret)?;
    }
    profiles.push(profile.clone());
    if persist(path, &profiles).is_err() {
        if method.uses_vault() && vault.erase(&profile.id).is_err() {
            bail!(
                "인증 목록 저장과 비밀 롤백에 실패했습니다. Windows 자격 증명 관리자를 확인하세요."
            );
        }
        bail!("인증 목록을 저장하지 못했습니다.");
    }
    Ok(visible(&profiles))
}
pub fn register(
    url: &str,
    username: &str,
    method: AuthMethod,
    secret: Option<&Secret>,
) -> Result<Vec<AuthProfile>> {
    register_at(
        &store_path()?,
        &SystemSecrets,
        url,
        username,
        method,
        secret,
    )
}
fn remove_impl(
    path: &Path,
    vault: &dyn SecretStore,
    id: &str,
    before_remove: impl FnOnce(&AuthProfile) -> Result<bool>,
) -> Result<Vec<AuthProfile>> {
    let _lock = locked(path)?;
    let mut profiles = load_from(path)?;
    let index = profiles
        .iter()
        .position(|p| p.id == id && !p.hidden)
        .context("삭제할 인증이 없습니다.")?;
    let profile = profiles[index].clone();
    let shared_logged_out = before_remove(&profile)?;
    profiles.remove(index);
    if shared_logged_out {
        profiles.retain(|entry| {
            !(entry.imported
                && same_url(&entry.url, &profile.url)
                && entry.username == profile.username)
        });
    } else {
        for imported in profiles.iter_mut().filter(|entry| {
            entry.imported
                && same_url(&entry.url, &profile.url)
                && entry.username == profile.username
        }) {
            imported.hidden = true;
        }
        if profile.imported {
            let mut hidden = profile.clone();
            hidden.hidden = true;
            profiles.push(hidden);
        }
    }
    let old_secret = if profile.method.uses_vault() {
        // Missing OS credentials must not prevent removing a stale registration.
        let old = match vault.read(id) {
            Ok(secret) => Some(secret),
            Err(error) if error.is::<MissingSecret>() => None,
            Err(error) => return Err(error),
        };
        vault.erase(id)?;
        old
    } else {
        None
    };
    if persist(path, &profiles).is_err() {
        if let Some(secret) = old_secret {
            vault
                .write(id, &profile.username, &secret)
                .context("인증 삭제 롤백에 실패했습니다.")?;
        }
        if shared_logged_out {
            bail!(
                "GCM 로그아웃은 완료했지만 앱 인증 목록을 저장하지 못했습니다. 설정을 다시 열어 목록을 정리하세요."
            );
        }
        bail!("인증 목록을 변경하지 못했습니다.");
    }
    Ok(visible(&profiles))
}
pub fn remove_at(path: &Path, vault: &dyn SecretStore, id: &str) -> Result<Vec<AuthProfile>> {
    remove_impl(path, vault, id, |_| Ok(false))
}
pub fn remove_after_logout_at(
    path: &Path,
    vault: &dyn SecretStore,
    id: &str,
    logout: impl FnOnce(&AuthProfile) -> Result<()>,
) -> Result<Vec<AuthProfile>> {
    remove_impl(path, vault, id, |profile| {
        if profile.method.uses_vault() {
            return Ok(false);
        }
        logout(profile)?;
        Ok(true)
    })
}
pub fn remove_after_logout(
    id: &str,
    logout: impl FnOnce(&AuthProfile) -> Result<()>,
) -> Result<Vec<AuthProfile>> {
    remove_after_logout_at(&store_path()?, &SystemSecrets, id, logout)
}
pub fn remove(id: &str) -> Result<Vec<AuthProfile>> {
    remove_at(&store_path()?, &SystemSecrets, id)
}

pub fn select<'a>(
    profiles: &'a [AuthProfile],
    remote: &str,
    username: Option<&str>,
) -> Option<&'a AuthProfile> {
    let url = normalize_url(remote).ok()?;
    profiles
        .iter()
        .filter_map(|p| {
            if p.hidden || p.imported || username.is_some_and(|name| name != p.username) {
                return None;
            }
            let scope = normalize_url(&p.url).ok()?;
            (url == scope
                || url
                    .strip_prefix(&scope)
                    .is_some_and(|rest| scope.ends_with('/') || rest.starts_with('/')))
            .then_some((scope.len(), p))
        })
        .max_by_key(|(length, _)| *length)
        .map(|(_, profile)| profile)
}

pub fn for_remote_urls(profiles: &[AuthProfile], remotes: &[String]) -> Vec<AuthProfile> {
    let mut result = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for remote in remotes {
        let Ok(mut parsed) = url::Url::parse(remote) else {
            continue;
        };
        if parsed.scheme() != "https" || parsed.host_str().is_none() {
            continue;
        }
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);
        parsed.set_query(None);
        parsed.set_fragment(None);
        let Ok(mut scope) = normalize_url(parsed.as_str()) else {
            continue;
        };
        loop {
            let matches = profiles
                .iter()
                .filter(|profile| {
                    !profile.hidden && normalize_url(&profile.url).is_ok_and(|url| url == scope)
                })
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                for profile in matches {
                    if seen.insert(profile.id.clone()) {
                        result.push(profile.clone());
                    }
                }
                break;
            }
            let Ok(mut parent) = url::Url::parse(&scope) else {
                break;
            };
            let path = parent.path().trim_end_matches('/');
            if path.is_empty() {
                break;
            }
            let parent_path = path
                .rsplit_once('/')
                .map(|(path, _)| path)
                .unwrap_or("")
                .to_owned();
            parent.set_path(&parent_path);
            let Ok(next) = normalize_url(parent.as_str()) else {
                break;
            };
            if next == scope {
                break;
            }
            scope = next;
        }
    }
    result
}

pub enum ResolvedAuth {
    Direct { username: String, secret: Secret },
    Gcm { username: String, url: String },
}
pub fn resolve(
    profiles: &[AuthProfile],
    vault: &dyn SecretStore,
    remote: &str,
    username: Option<&str>,
) -> Result<Option<ResolvedAuth>> {
    let Some(profile) = select(profiles, remote, username) else {
        return Ok(None);
    };
    if profile.method.uses_vault() {
        Ok(Some(ResolvedAuth::Direct {
            username: profile.username.clone(),
            secret: vault.read(&profile.id)?,
        }))
    } else {
        Ok(Some(ResolvedAuth::Gcm {
            username: profile.username.clone(),
            url: profile.url.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Vault {
        values: Mutex<std::collections::BTreeMap<String, String>>,
        fail_erase: bool,
        fail_read: bool,
    }
    impl SecretStore for Vault {
        fn read(&self, id: &str) -> Result<Secret> {
            if self.fail_read {
                bail!("읽기 권한 오류");
            }
            Ok(Secret::new(
                self.values
                    .lock()
                    .unwrap()
                    .get(id)
                    .ok_or(MissingSecret)?
                    .clone(),
            ))
        }
        fn write(&self, id: &str, _: &str, s: &Secret) -> Result<()> {
            self.values
                .lock()
                .unwrap()
                .insert(id.into(), s.expose().into());
            Ok(())
        }
        fn erase(&self, id: &str) -> Result<()> {
            if self.fail_erase {
                bail!("locked");
            }
            self.values.lock().unwrap().remove(id);
            Ok(())
        }
    }

    #[test]
    fn git_suffix_is_a_real_credential_path_and_registered_url_is_used_verbatim() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let vault = Vault::default();
        assert_eq!(
            normalize_url("https://github.com/tcpip147/zit.git").unwrap(),
            "https://github.com/tcpip147/zit.git"
        );
        assert_ne!(
            normalize_url("https://github.com/tcpip147/zit").unwrap(),
            normalize_url("https://github.com/tcpip147/zit.git").unwrap()
        );
        let old = register_at(
            &path,
            &vault,
            "https://github.com/tcpip147/zit",
            "account",
            AuthMethod::Gcm,
            None,
        )
        .unwrap();
        assert!(select(&old, "https://github.com/tcpip147/zit.git", None).is_none());
        let entries = register_at(
            &path,
            &vault,
            "https://github.com/tcpip147/zit.git",
            "account",
            AuthMethod::Gcm,
            None,
        )
        .unwrap();
        let Some(ResolvedAuth::Gcm { url, username }) = resolve(
            &entries,
            &vault,
            "https://github.com/tcpip147/zit.git",
            None,
        )
        .unwrap() else {
            panic!("등록 GCM 인증이 필요합니다.");
        };
        assert_eq!(url, "https://github.com/tcpip147/zit.git");
        assert_eq!(
            std::str::from_utf8(&credential_request(&url, Some(&username)).unwrap()).unwrap(),
            "protocol=https\nhost=github.com\npath=tcpip147/zit.git\nusername=account\n\n"
        );
    }

    #[test]
    fn parent_scope_requests_its_own_path_and_preserves_https_port() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let vault = Vault::default();
        let entries = register_at(
            &path,
            &vault,
            "https://example.invalid:8443/team",
            "account",
            AuthMethod::Gcm,
            None,
        )
        .unwrap();
        let Some(ResolvedAuth::Gcm { url, username }) = resolve(
            &entries,
            &vault,
            "https://example.invalid:8443/team/repo.git",
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert_eq!(url, "https://example.invalid:8443/team");
        let request =
            String::from_utf8(credential_request(&url, Some(&username)).unwrap()).unwrap();
        assert!(request.contains("host=example.invalid:8443\n"));
        assert!(request.contains("path=team\n"));
        assert!(!request.contains("repo.git"));
        assert!(credential_request(&url, Some("account\npassword=secret")).is_err());
    }

    #[test]
    fn exact_windows_target_removal_preserves_other_accounts_paths_and_services() {
        struct Store(Mutex<Vec<ExistingCredential>>);
        impl ExistingCredentialStore for Store {
            fn entries(&self) -> Result<Vec<ExistingCredential>> {
                Ok(self.0.lock().unwrap().clone())
            }
            fn erase(&self, target: &str) -> Result<()> {
                self.0
                    .lock()
                    .unwrap()
                    .retain(|entry| entry.target != target);
                Ok(())
            }
        }
        let selected = ExistingCredential {
            target: "git:https://alice@github.com/team/repo.git".into(),
            username: "alice".into(),
        };
        let profile = existing_profile(&selected).unwrap();
        let store = Store(Mutex::new(vec![
            selected,
            ExistingCredential {
                target: "git:https://bob@github.com/team/repo.git".into(),
                username: "bob".into(),
            },
            ExistingCredential {
                target: "git:https://github.com/team/other.git".into(),
                username: "alice".into(),
            },
            ExistingCredential {
                target: "git:https://gitlab.com/team/repo.git".into(),
                username: "alice".into(),
            },
        ]));
        assert_eq!(erase_existing_for(&profile, &store).unwrap(), 1);
        let remaining = store.entries().unwrap();
        assert_eq!(remaining.len(), 3);
        assert!(remaining.iter().any(|entry| entry.username == "bob"));
        assert_eq!(erase_existing_for(&profile, &store).unwrap(), 0);
    }

    #[test]
    fn logout_removal_calls_account_hook_before_metadata_and_preserves_failures() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let vault = Vault::default();
        let entries = register_at(
            &path,
            &vault,
            "https://github.com",
            "alice",
            AuthMethod::Gcm,
            None,
        )
        .unwrap();
        let before = fs::read(&path).unwrap();
        assert!(
            remove_after_logout_at(&path, &vault, &entries[0].id, |_| bail!("GCM 실패")).is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::write(path.with_extension("lock"), "locked").unwrap();
        assert!(
            remove_after_logout_at(&path, &vault, &entries[0].id, |_| panic!(
                "잠금 실패 시 GCM을 실행하면 안 됩니다."
            ))
            .is_err()
        );
        fs::remove_file(path.with_extension("lock")).unwrap();
        let result = remove_after_logout_at(&path, &vault, &entries[0].id, |profile| {
            assert_eq!(profile.username, "alice");
            assert_eq!(profile.url, "https://github.com/");
            assert_eq!(fs::read(&path).unwrap(), before);
            Ok(())
        })
        .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn actual_logout_allows_future_login_and_reports_partial_metadata_failure() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let vault = Vault::default();
        let existing = vec![ExistingCredential {
            target: "git:https://github.com".into(),
            username: "alice".into(),
        }];
        let entries = merge_existing_at(&path, &existing).unwrap();
        fs::create_dir(path.with_extension("json.tmp")).unwrap();
        let error = remove_after_logout_at(&path, &vault, &entries[0].id, |_| Ok(()))
            .unwrap_err()
            .to_string();
        assert!(error.contains("로그아웃은 완료"));
        assert_eq!(load_from(&path).unwrap(), entries);
        fs::remove_dir(path.with_extension("json.tmp")).unwrap();
        assert!(
            remove_after_logout_at(&path, &vault, &entries[0].id, |_| Ok(()))
                .unwrap()
                .is_empty()
        );
        assert!(load_from(&path).unwrap().is_empty());
        assert_eq!(merge_existing_at(&path, &existing).unwrap().len(), 1);
        let direct = register_at(
            &path,
            &vault,
            "https://other.invalid",
            "user",
            AuthMethod::Password,
            Some(&Secret::new("secret".into())),
        )
        .unwrap();
        let id = &direct
            .iter()
            .find(|profile| profile.method.uses_vault())
            .unwrap()
            .id;
        remove_after_logout_at(&path, &vault, id, |_| {
            panic!("직접 인증에는 GCM을 실행하면 안 됩니다.")
        })
        .unwrap();
        assert!(vault.read(id).is_err());
    }

    #[test]
    fn project_list_uses_exact_then_nearest_parent_and_recalculates_after_removal() {
        fn profile(id: &str, url: &str, imported: bool) -> AuthProfile {
            AuthProfile {
                id: id.into(),
                url: url.into(),
                username: id.into(),
                method: AuthMethod::Gcm,
                imported,
                hidden: false,
            }
        }
        let mut entries = vec![
            profile("host", "https://github.com", false),
            profile("owner", "https://github.com/tcpip147", true),
            profile("repo", "https://github.com/tcpip147/zit.git", false),
        ];
        let remotes = vec!["https://github.com/tcpip147/zit.git".into()];
        assert_eq!(
            for_remote_urls(&entries, &remotes)
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            vec!["repo"]
        );
        entries.pop();
        assert_eq!(for_remote_urls(&entries, &remotes)[0].id, "owner");
        entries.pop();
        assert_eq!(for_remote_urls(&entries, &remotes)[0].id, "host");
        assert!(for_remote_urls(&entries, &[]).is_empty());
    }

    #[test]
    fn project_list_combines_remotes_and_accounts_without_prefix_leaks_or_duplicates() {
        let entries = [
            ("a", "https://github.com/team/repo.git", "alice"),
            ("b", "https://github.com/team/repo.git", "bob"),
            ("host", "https://github.com", "host-user"),
            ("other", "https://gitlab.com/team/repo", "other"),
        ]
        .map(|(id, url, username)| AuthProfile {
            id: id.into(),
            url: url.into(),
            username: username.into(),
            method: AuthMethod::Gcm,
            imported: false,
            hidden: false,
        });
        let remotes = [
            "https://github.com/team/repo.git",
            "https://user@github.com/team/repo.git",
            "https://github.com/team/repo-other",
        ]
        .map(str::to_owned);
        assert_eq!(
            for_remote_urls(&entries, &remotes)
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "host"]
        );
        assert!(
            for_remote_urls(
                &entries,
                &["https://github.com.evil.invalid/team/repo".into()]
            )
            .is_empty()
        );
        assert!(for_remote_urls(&entries, &["git@github.com:team/repo.git".into()]).is_empty());
        let mut hidden = entries.to_vec();
        hidden[0].hidden = true;
        assert_eq!(for_remote_urls(&hidden, &[remotes[0].clone()])[0].id, "b");
    }

    #[test]
    fn existing_git_accounts_are_imported_without_url_secrets_or_duplicate_rows() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let existing = vec![
            ExistingCredential { target: "git:https://ignored:secret-password@github.com/team/repo.git?token=secret-token#secret-fragment".into(), username: "alice".into() },
            ExistingCredential { target: "git:https://github.com/team/repo.git".into(), username: "alice".into() },
            ExistingCredential { target: "git:https://github.com/team/repo.git".into(), username: "bob".into() },
            ExistingCredential { target: "other:https://example.invalid".into(), username: "unrelated".into() },
        ];
        let entries = merge_existing_at(&path, &existing).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(
            entries.iter().all(
                |profile| profile.imported && profile.url == "https://github.com/team/repo.git"
            )
        );
        let saved = fs::read_to_string(&path).unwrap();
        for secret in [
            "secret-password",
            "secret-token",
            "secret-fragment",
            "ignored",
            "unrelated",
        ] {
            assert!(!saved.contains(secret));
        }
        assert_eq!(merge_existing_at(&path, &existing).unwrap(), entries);
        // Merely displaying existing accounts must not force one account on a transport.
        assert!(select(&entries, "https://github.com/team/repo.git", None).is_none());
        let before = fs::read(&path).unwrap();
        fs::write(path.with_extension("lock"), "locked").unwrap();
        assert!(merge_existing_at(&path, &[]).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn imported_account_dismissal_persists_without_touching_shared_secrets() {
        struct NoSecrets;
        impl SecretStore for NoSecrets {
            fn read(&self, _: &str) -> Result<Secret> {
                panic!("외부 비밀을 읽으면 안 됩니다.")
            }
            fn write(&self, _: &str, _: &str, _: &Secret) -> Result<()> {
                panic!("외부 비밀을 쓰면 안 됩니다.")
            }
            fn erase(&self, _: &str) -> Result<()> {
                panic!("외부 비밀을 지우면 안 됩니다.")
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let existing = vec![ExistingCredential {
            target: "git:https://github.com".into(),
            username: "account".into(),
        }];
        let entries = merge_existing_at(&path, &existing).unwrap();
        assert!(
            remove_at(&path, &NoSecrets, &entries[0].id)
                .unwrap()
                .is_empty()
        );
        assert!(merge_existing_at(&path, &existing).unwrap().is_empty());
        assert!(load_from(&path).unwrap()[0].hidden);
    }

    #[test]
    fn previous_profiles_remain_compatible_and_override_imported_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        fs::write(
            &path,
            br#"[{"id":"old","url":"https://github.com","username":"alice","method":"Gcm"}]"#,
        )
        .unwrap();
        let existing = vec![ExistingCredential {
            target: "git:https://github.com".into(),
            username: "alice".into(),
        }];
        let entries = merge_existing_at(&path, &existing).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "old");
        assert!(!entries[0].imported);
        assert!(
            remove_at(&path, &Vault::default(), "old")
                .unwrap()
                .is_empty()
        );
        assert!(merge_existing_at(&path, &existing).unwrap().is_empty());
    }
    #[test]
    fn profiles_persist_without_secrets_and_select_most_specific_scope() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let vault = Vault::default();
        register_at(
            &path,
            &vault,
            "https://github.com",
            "host-user",
            AuthMethod::Gcm,
            None,
        )
        .unwrap();
        let entries = register_at(
            &path,
            &vault,
            "https://github.com/team/repo.git",
            "repo-user",
            AuthMethod::Password,
            Some(&Secret::new("secret-token".into())),
        )
        .unwrap();
        assert!(!fs::read_to_string(&path).unwrap().contains("secret-token"));
        assert_eq!(load_from(&path).unwrap(), entries);
        assert_eq!(
            select(&entries, "https://github.com/team/repo.git", None)
                .unwrap()
                .username,
            "repo-user"
        );
        assert_eq!(
            select(&entries, "https://github.com/team/repo-other", None)
                .unwrap()
                .username,
            "host-user"
        );
        assert!(select(&entries, "https://github.com.evil.invalid/team/repo", None).is_none());
        assert!(
            register_at(
                &path,
                &vault,
                "https://github.com/team/repo.git",
                "other",
                AuthMethod::Gcm,
                None
            )
            .is_err()
        );
        let id = &entries[1].id;
        assert_eq!(vault.read(id).unwrap().expose(), "secret-token");
        match resolve(&entries, &vault, "https://github.com/team/repo.git", None)
            .unwrap()
            .unwrap()
        {
            ResolvedAuth::Direct { username, secret } => {
                assert_eq!(username, "repo-user");
                assert_eq!(secret.expose(), "secret-token");
            }
            _ => panic!("직접 인증이 선택되어야 합니다."),
        }
        assert!(matches!(
            resolve(&entries, &vault, "https://github.com/other", None).unwrap(),
            Some(ResolvedAuth::Gcm { .. })
        ));
        assert!(
            resolve(
                &entries,
                &vault,
                "https://github.com/team/repo",
                Some("unknown")
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(remove_at(&path, &vault, id).unwrap().len(), 1);
        assert!(vault.read(id).is_err());
    }
    #[test]
    fn invalid_input_and_failed_deletion_preserve_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let mut vault = Vault::default();
        for url in [
            "http://example.invalid",
            "https://user:secret@example.invalid",
            "https://example.invalid?token=secret",
            "https://example.invalid\n",
        ] {
            assert!(normalize_url(url).is_err());
        }
        let profiles = register_at(
            &path,
            &vault,
            "https://example.invalid",
            "account",
            AuthMethod::Password,
            Some(&Secret::new("token".into())),
        )
        .unwrap();
        vault.fail_read = true;
        assert!(remove_at(&path, &vault, &profiles[0].id).is_err());
        assert_eq!(vault.values.lock().unwrap().len(), 1);
        vault.fail_read = false;
        vault.fail_erase = true;
        assert!(remove_at(&path, &vault, &profiles[0].id).is_err());
        assert_eq!(load_from(&path).unwrap(), profiles);
        fs::write(path.with_extension("lock"), "locked").unwrap();
        assert!(
            register_at(
                &path,
                &vault,
                "https://other.invalid",
                "user",
                AuthMethod::Gcm,
                None
            )
            .is_err()
        );
        assert_eq!(load_from(&path).unwrap(), profiles);
    }

    #[test]
    fn metadata_write_failures_roll_back_created_and_deleted_secrets() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("auth.json");
        let vault = Vault::default();
        let secret = Secret::new("secret-token".into());
        fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(
            register_at(
                &path,
                &vault,
                "https://example.invalid",
                "account",
                AuthMethod::Password,
                Some(&secret)
            )
            .is_err()
        );
        assert!(vault.values.lock().unwrap().is_empty());
        assert!(!path.exists());
        fs::remove_dir(path.with_extension("json.tmp")).unwrap();
        let profiles = register_at(
            &path,
            &vault,
            "https://example.invalid",
            "account",
            AuthMethod::Password,
            Some(&secret),
        )
        .unwrap();
        fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(remove_at(&path, &vault, &profiles[0].id).is_err());
        assert_eq!(load_from(&path).unwrap(), profiles);
        assert_eq!(
            vault.read(&profiles[0].id).unwrap().expose(),
            "secret-token"
        );
    }
}
