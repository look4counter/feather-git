use anyhow::{Context as _, Result, anyhow, bail, ensure};
use semver::Version;
use std::{
    env, fs,
    io::{self, Read, Seek, SeekFrom},
    os::windows::{fs::MetadataExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{ChildStdin, Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use url::Url;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MANIFEST_URL: &str = "https://look4counter.github.io/look4counter/versions.xml";
const APP_ID: &str = "com.look4counter.feather-git";
const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_EXE: u64 = 128 * 1024 * 1024;
const STAGE_PREFIX: &str = ".feather-git-update-";
const CANDIDATE: &str = "download.exe";
const HELPER: &str = "helper.exe";
const ROLLBACK: &str = "rollback.exe";
static EXIT_PIPE: Mutex<Option<ChildStdin>> = Mutex::new(None);

#[derive(Clone)]
pub struct Release {
    pub version: Version,
    url: Url,
}

fn parse_manifest(xml: &str, current: &Version) -> Result<Option<Release>> {
    let document =
        roxmltree::Document::parse(xml).context("업데이트 XML 형식이 잘못되었습니다.")?;
    let root = document.root_element();
    ensure!(
        root.has_tag_name("versions"),
        "업데이트 XML에 versions 요소가 없습니다."
    );
    let mut entries = root
        .children()
        .filter(|node| node.has_tag_name("version") && node.attribute("id") == Some(APP_ID));
    let entry = entries
        .next()
        .context("Feather Git 버전 항목이 없습니다.")?;
    ensure!(
        entries.next().is_none(),
        "Feather Git 버전 항목이 중복되었습니다."
    );
    let version = Version::parse(
        entry
            .attribute("version")
            .context("버전 번호가 없습니다.")?,
    )
    .context("버전 번호가 올바르지 않습니다.")?;
    let url = Url::parse(
        entry
            .attribute("url")
            .context("다운로드 주소가 없습니다.")?,
    )
    .context("다운로드 주소가 올바르지 않습니다.")?;
    let release = Release { version, url };
    release.validate_url()?;
    Ok(release
        .version
        .cmp_precedence(current)
        .is_gt()
        .then_some(release))
}

impl Release {
    fn validate_url(&self) -> Result<()> {
        let asset = |tag: &str| {
            format!(
                "https://github.com/look4counter/feather-git/releases/download/{tag}/feather-git.exe"
            )
        };
        ensure!(
            self.url.as_str() == asset(&self.version.to_string())
                || self.url.as_str() == asset(&format!("v{}", self.version)),
            "다운로드 주소가 Feather Git의 공식 릴리스와 일치하지 않습니다."
        );
        Ok(())
    }
}

fn client(hosts: &'static [&'static str]) -> Result<avatar_http::blocking::Client> {
    use avatar_http::redirect::Policy;
    let mut builder = avatar_http::blocking::Client::builder()
        .use_native_tls()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("FeatherGit/", env!("CARGO_PKG_VERSION")))
        .redirect(Policy::custom(move |attempt| {
            let url = attempt.url();
            if attempt.previous().len() < 5
                && url.scheme() == "https"
                && url.port().is_none()
                && url.host_str().is_some_and(|host| hosts.contains(&host))
            {
                attempt.follow()
            } else {
                attempt.error("허용되지 않은 업데이트 리디렉션")
            }
        }));
    for certificate in crate::update_certificates::certificates()? {
        builder = builder.add_root_certificate(certificate);
    }
    Ok(builder.build()?)
}

pub fn check() -> Result<Option<Release>> {
    let response = client(&["look4counter.github.io"])?
        .get(MANIFEST_URL)
        .send()
        .context("버전 목록에 연결하지 못했습니다.")?
        .error_for_status()
        .context("버전 목록을 다운로드하지 못했습니다.")?;
    let mut xml = String::new();
    response.take(MAX_MANIFEST + 1).read_to_string(&mut xml)?;
    ensure!(
        xml.len() as u64 <= MAX_MANIFEST,
        "업데이트 XML이 너무 큽니다."
    );
    parse_manifest(
        xml.trim_start_matches('\u{feff}'),
        &Version::parse(CURRENT_VERSION)?,
    )
}

pub struct PreparedUpdate {
    installed: PathBuf,
    stage: tempfile::TempDir,
}

pub fn prepare(release: Release) -> Result<PreparedUpdate> {
    release.validate_url()?;
    ensure!(
        release
            .version
            .cmp_precedence(&Version::parse(CURRENT_VERSION)?)
            .is_gt(),
        "현재 버전보다 새 버전만 설치할 수 있습니다."
    );
    let installed = env::current_exe()?;
    let parent = installed
        .parent()
        .context("설치 경로를 확인할 수 없습니다.")?;
    let stage = tempfile::Builder::new()
        .prefix(STAGE_PREFIX)
        .tempdir_in(parent)
        .context("실행 파일 폴더에 업데이트를 준비할 수 없습니다.")?;
    let response = client(&[
        "github.com",
        "release-assets.githubusercontent.com",
        "objects.githubusercontent.com",
        "github-releases.githubusercontent.com",
    ])?
    .get(release.url)
    .send()
    .context("업데이트를 다운로드하지 못했습니다.")?
    .error_for_status()
    .context("릴리스의 실행 파일을 읽을 수 없습니다.")?;
    let expected = response.content_length();
    ensure!(
        expected.is_none_or(|size| size <= MAX_EXE),
        "업데이트 파일이 너무 큽니다."
    );
    let path = stage.path().join(CANDIDATE);
    let mut file = fs::File::create(&path)?;
    let size = io::copy(&mut response.take(MAX_EXE + 1), &mut file)
        .context("업데이트 파일을 저장하지 못했습니다.")?;
    ensure!(
        size <= MAX_EXE && expected.is_none_or(|length| length == size),
        "업데이트 파일의 크기가 올바르지 않습니다."
    );
    file.sync_all()?;
    drop(file);
    validate_executable(&path)?;
    fs::copy(&installed, stage.path().join(HELPER))
        .context("현재 실행 파일을 백업하지 못했습니다.")?;
    Ok(PreparedUpdate { installed, stage })
}

fn validate_executable(path: &Path) -> Result<()> {
    let mut file = fs::File::open(path)?;
    let mut header = [0; 64];
    file.read_exact(&mut header)
        .context("실행 파일 헤더가 불완전합니다.")?;
    ensure!(&header[..2] == b"MZ", "Windows 실행 파일이 아닙니다.");
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap());
    file.seek(SeekFrom::Start(offset.into()))?;
    let mut signature = [0; 4];
    file.read_exact(&mut signature)
        .context("실행 파일 형식이 불완전합니다.")?;
    ensure!(
        &signature == b"PE\0\0",
        "Windows 실행 파일 형식이 올바르지 않습니다."
    );
    Ok(())
}

impl PreparedUpdate {
    pub fn launch(self) -> Result<()> {
        let mut pipe = EXIT_PIPE
            .lock()
            .map_err(|_| anyhow!("업데이트 상태를 읽지 못했습니다."))?;
        ensure!(pipe.is_none(), "업데이트가 이미 시작되었습니다.");
        let mut helper = Command::new(self.stage.path().join(HELPER))
            .arg("--apply-update")
            .arg(&self.installed)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .context("업데이트 도우미를 시작할 수 없습니다.")?;
        *pipe = Some(
            helper
                .stdin
                .take()
                .context("종료 대기 파이프를 열지 못했습니다.")?,
        );
        let _ = self.stage.keep();
        Ok(())
    }
}

fn valid_stage(installed: &Path, stage: &Path) -> bool {
    installed.parent().is_some()
        && installed.parent() == stage.parent()
        && stage
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(STAGE_PREFIX) && name.len() > STAGE_PREFIX.len())
        && stage
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.is_dir() && metadata.file_attributes() & 0x400 == 0)
}

fn install_and_start(
    installed: &Path,
    stage: &Path,
    launch: impl FnOnce() -> Result<()>,
) -> Result<()> {
    fs::rename(stage.join(CANDIDATE), installed).context(
        "실행 파일을 교체하지 못했습니다. 실행 중인 다른 Feather Git 창을 닫고 다시 시도하세요.",
    )?;
    if let Err(error) = launch() {
        let rollback = stage.join(ROLLBACK);
        let result =
            fs::copy(stage.join(HELPER), &rollback).and_then(|_| fs::rename(&rollback, installed));
        if let Err(restore_error) = result {
            bail!(
                "새 버전을 실행하지 못했습니다: {error:#}\n복원 실패: {restore_error}\n이전 실행 파일: {}",
                stage.join(HELPER).display()
            );
        }
        bail!("새 버전을 실행하지 못해 이전 실행 파일을 복원했습니다: {error:#}");
    }
    Ok(())
}

fn apply(installed: &Path) -> Result<()> {
    let helper = env::current_exe()?;
    let stage = helper.parent().context("업데이트 작업 폴더가 없습니다.")?;
    ensure!(
        helper.file_name().is_some_and(|name| name == HELPER) && valid_stage(installed, stage),
        "업데이트 작업 경로가 올바르지 않습니다."
    );
    // 빈 파이프는 기존 프로세스가 종료되어 운영 체제가 쓰기 핸들을 닫았다는 뜻입니다.
    ensure!(
        io::stdin().read(&mut [0])? == 0,
        "업데이트 종료 대기 신호가 잘못되었습니다."
    );
    validate_executable(&stage.join(CANDIDATE))?;
    install_and_start(installed, stage, || {
        Command::new(installed)
            .arg("--update-cleanup")
            .arg(stage)
            .spawn()
            .context("교체한 실행 파일을 열지 못했습니다.")?;
        Ok(())
    })
}

fn cleanup(stage: PathBuf) {
    let Ok(installed) = env::current_exe() else {
        return;
    };
    if !valid_stage(&installed, &stage) {
        return;
    }
    std::thread::spawn(move || {
        for _ in 0..40 {
            for file in [CANDIDATE, HELPER, ROLLBACK] {
                let _ = fs::remove_file(stage.join(file));
            }
            if fs::remove_dir(&stage).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
}

fn report_failure(error: anyhow::Error) {
    use windows::{
        Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
        core::HSTRING,
    };
    let text = HSTRING::from(format!("{error:#}"));
    let title = HSTRING::from("Feather Git 업데이트 실패");
    unsafe { MessageBoxW(None, &text, &title, MB_OK | MB_ICONERROR) };
}

pub fn application_args() -> Option<Vec<String>> {
    let mut args = env::args_os().skip(1);
    let first = args.next();
    match first.as_deref().and_then(|arg| arg.to_str()) {
        Some("--apply-update") => {
            if let Err(error) = args
                .next()
                .context("교체할 실행 파일 경로가 없습니다.")
                .and_then(|installed| apply(Path::new(&installed)))
            {
                report_failure(error);
            }
            None
        }
        Some("--update-cleanup") => {
            if let Some(stage) = args.next() {
                cleanup(stage.into());
            }
            Some(Vec::new())
        }
        _ => Some(
            first
                .into_iter()
                .chain(args)
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, tag: &str) -> String {
        format!(
            r#"<versions><version id="other" version="99.0.0"/>
            <version id="{APP_ID}" version="{version}" url="https://github.com/look4counter/feather-git/releases/download/{tag}/feather-git.exe"> </version>
            </versions>"#
        )
    }

    #[test]
    fn compares_versions_without_downgrading() {
        let current = Version::parse("1.0.3").unwrap();
        assert!(
            parse_manifest(&manifest("1.0.3", "1.0.3"), &current)
                .unwrap()
                .is_none()
        );
        assert!(
            parse_manifest(&manifest("1.0.2", "1.0.2"), &current)
                .unwrap()
                .is_none()
        );
        assert!(
            parse_manifest(&manifest("1.0.3-rc.1", "1.0.3-rc.1"), &current)
                .unwrap()
                .is_none()
        );
        let next = parse_manifest(&manifest("1.0.10", "v1.0.10"), &current)
            .unwrap()
            .unwrap();
        assert_eq!(next.version, Version::parse("1.0.10").unwrap());
    }

    #[test]
    fn rejects_missing_duplicate_malformed_and_unofficial_updates() {
        let current = Version::parse("1.0.3").unwrap();
        let xml = manifest("1.0.4", "1.0.4");
        let duplicate = xml.replace(
            "</versions>",
            &format!(r#"<version id="{APP_ID}"/></versions>"#),
        );
        for invalid in [
            "<versions/>".into(),
            "<versions><version>".into(),
            duplicate,
            xml.replace("https://github.com", "http://github.com"),
            xml.replace("look4counter/feather-git", "unrelated/feather-git"),
            xml.replace("/download/1.0.4/", "/download/1.0.3/"),
        ] {
            assert!(parse_manifest(&invalid, &current).is_err(), "{invalid}");
        }
    }

    #[test]
    fn replaces_file_and_rolls_back_when_restart_fails() {
        let directory = tempfile::tempdir().unwrap();
        let installed = directory.path().join("feather-git.exe");
        let stage = tempfile::Builder::new()
            .prefix(STAGE_PREFIX)
            .tempdir_in(directory.path())
            .unwrap();
        assert!(valid_stage(&installed, stage.path()));
        fs::write(&installed, b"old").unwrap();
        fs::write(stage.path().join(HELPER), b"old").unwrap();
        assert!(
            install_and_start(&installed, stage.path(), || panic!("missing download")).is_err()
        );
        assert_eq!(fs::read(&installed).unwrap(), b"old");

        fs::write(stage.path().join(CANDIDATE), b"new").unwrap();
        assert!(
            install_and_start(&installed, stage.path(), || {
                assert_eq!(fs::read(&installed).unwrap(), b"new");
                bail!("restart failed")
            })
            .is_err()
        );
        assert_eq!(fs::read(&installed).unwrap(), b"old");

        fs::write(stage.path().join(CANDIDATE), b"new").unwrap();
        install_and_start(&installed, stage.path(), || Ok(())).unwrap();
        assert_eq!(fs::read(&installed).unwrap(), b"new");
    }

    #[test]
    fn rejects_partial_download_even_if_it_starts_with_a_dos_header() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(CANDIDATE);
        fs::write(&path, b"MZ").unwrap();
        assert!(validate_executable(&path).is_err());
        let mut header = [0; 68];
        header[..2].copy_from_slice(b"MZ");
        header[60] = 64;
        fs::write(&path, header).unwrap();
        assert!(validate_executable(&path).is_err());
    }
}
