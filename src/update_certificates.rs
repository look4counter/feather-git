use anyhow::{Context, Result, ensure};
use avatar_http::Certificate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_FILES: usize = 32;
const MAX_STORE_BYTES: usize = MAX_FILES * MAX_FILE_BYTES * 4 + 64 * 1024;

#[derive(Serialize, Deserialize)]
struct StoredCertificate {
    name: String,
    bytes: Vec<u8>,
}

pub struct Entry {
    pub id: String,
    pub name: String,
}

impl StoredCertificate {
    fn id(&self) -> String {
        format!("{:x}", Sha256::digest(&self.bytes))
    }

    fn certificates(&self) -> Result<Vec<Certificate>> {
        ensure!(
            !self
                .bytes
                .windows(b"PRIVATE KEY".len())
                .any(|part| part == b"PRIVATE KEY"),
            "개인 키가 포함된 파일은 등록할 수 없습니다."
        );
        let certificates = if self
            .bytes
            .windows(b"-----BEGIN".len())
            .any(|part| part == b"-----BEGIN")
        {
            Certificate::from_pem_bundle(&self.bytes)?
        } else {
            vec![Certificate::from_der(&self.bytes)?]
        };
        ensure!(!certificates.is_empty(), "파일에 X.509 인증서가 없습니다.");
        Ok(certificates)
    }
}

fn path() -> Result<PathBuf> {
    Ok(crate::auth_profiles::store_path()?.with_file_name("update-certificates.json"))
}

fn read_limited(file: fs::File, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "인증서 파일 크기 제한을 초과했습니다."
    );
    Ok(bytes)
}

fn load(path: &Path) -> Result<Vec<StoredCertificate>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).context("업데이트 인증서 목록을 열 수 없습니다."),
    };
    serde_json::from_slice(&read_limited(file, MAX_STORE_BYTES)?)
        .context("업데이트 인증서 목록을 읽을 수 없습니다.")
}

fn entries(stored: Vec<StoredCertificate>) -> Vec<Entry> {
    stored
        .into_iter()
        .map(|cert| Entry {
            id: cert.id(),
            name: cert.name,
        })
        .collect()
}

pub fn list() -> Result<Vec<Entry>> {
    Ok(entries(load(&path()?)?))
}

pub fn certificates() -> Result<Vec<Certificate>> {
    let mut result = Vec::new();
    for stored in load(&path()?)? {
        result.extend(
            stored
                .certificates()
                .with_context(|| format!("업데이트 인증서를 읽을 수 없습니다. {}", stored.name))?,
        );
    }
    Ok(result)
}

fn change_at(
    path: &Path,
    change: impl FnOnce(&mut Vec<StoredCertificate>) -> Result<()>,
) -> Result<Vec<Entry>> {
    let directory = path.parent().context("인증서 저장 폴더가 없습니다.")?;
    fs::create_dir_all(directory)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))
        .context("업데이트 인증서 잠금 파일을 열 수 없습니다.")?;
    lock.lock()
        .context("업데이트 인증서 목록을 잠글 수 없습니다.")?;
    let mut stored = load(path)?;
    change(&mut stored)?;
    let mut temp = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut temp, &stored)?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .context("업데이트 인증서 목록을 저장할 수 없습니다.")?;
    Ok(entries(stored))
}

fn import_at(path: &Path, files: &[PathBuf]) -> Result<Vec<Entry>> {
    change_at(path, |stored| {
        for file in files {
            let name = file
                .file_name()
                .context("인증서 파일 이름이 없습니다.")?
                .to_string_lossy()
                .into_owned();
            let candidate = (|| {
                let bytes = read_limited(fs::File::open(file)?, MAX_FILE_BYTES)?;
                let candidate = StoredCertificate {
                    name: name.clone(),
                    bytes,
                };
                let mut builder =
                    avatar_http::blocking::Client::builder().tls_built_in_root_certs(false);
                for cert in candidate.certificates()? {
                    builder = builder.add_root_certificate(cert);
                }
                builder.build()?;
                Ok::<_, anyhow::Error>(candidate)
            })()
            .with_context(|| format!("인증서 파일을 등록할 수 없습니다. {name}"))?;
            if !stored
                .iter()
                .any(|existing| existing.bytes == candidate.bytes)
            {
                ensure!(
                    stored.len() < MAX_FILES,
                    "인증서 파일은 최대 {MAX_FILES}개까지 등록할 수 있습니다."
                );
                stored.push(candidate);
            }
        }
        Ok(())
    })
}

pub fn import(files: &[PathBuf]) -> Result<Vec<Entry>> {
    import_at(&path()?, files)
}

fn remove_at(path: &Path, id: &str) -> Result<Vec<Entry>> {
    change_at(path, |stored| {
        stored.retain(|entry| entry.id() != id);
        Ok(())
    })
}

pub fn remove(id: &str) -> Result<Vec<Entry>> {
    remove_at(&path()?, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CA: &str = r#"-----BEGIN CERTIFICATE-----
MIIBkTCCATegAwIBAgIUKlnm+kG6NL8YBXwCYNq3HluaFMAwCgYIKoZIzj0EAwIw
HjEcMBoGA1UEAwwTRmVhdGhlciBHaXQgVGVzdCBDQTAeFw0yNjExMjEwMzQ4MzJa
Fw0zNjExMTgwMzQ4MzJaMB4xHDAaBgNVBAMME0ZlYXRoZXIgR2l0IFRlc3QgQ0Ew
WTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAASW6bMiu+/+rV35Db60BgKwA7Z4azQ8
nKCwSplsdfxj2s7m/J6lrYZEE4MNWYcGpRsgewS6uISOJjfjrumtJNOoo1MwUTAd
BgNVHQ4EFgQUFam+78/l/adcbWTBzYMkSD5irjcwHwYDVR0jBBgwFoAUFam+78/l
/adcbWTBzYMkSD5irjcwDwYDVR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNIADBF
AiAMzN2LKERJxytgMHD9zbCDvSjNvTwb4NKKCB55CAB8YQIhAI/i6UoZ4TQxs8E5
zFclivu0IUgNV/rwS97qyelCbGIq
-----END CERTIFICATE-----
"#;

    #[test]
    fn update_certificate_import_is_persistent_deduplicated_and_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("certificates.json");
        let cert = dir.path().join("corporate.pem");
        let duplicate = dir.path().join("duplicate.pem");
        let invalid = dir.path().join("invalid.cer");
        fs::write(&cert, TEST_CA).unwrap();
        fs::write(&duplicate, TEST_CA).unwrap();
        fs::write(&invalid, b"not a certificate").unwrap();

        assert!(load(&store).unwrap().is_empty());
        let registered = import_at(&store, &[cert.clone(), duplicate]).unwrap();
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].name, "corporate.pem");
        let stored_before = fs::read(&store).unwrap();
        fs::write(&cert, format!("{TEST_CA}\n")).unwrap();
        assert!(import_at(&store, &[cert, invalid]).is_err());
        assert_eq!(fs::read(&store).unwrap(), stored_before);

        let reloaded = load(&store).unwrap();
        assert_eq!(reloaded[0].certificates().unwrap().len(), 1);
        assert_eq!(reloaded[0].id(), registered[0].id);
        assert!(remove_at(&store, &registered[0].id).unwrap().is_empty());
        assert!(load(&store).unwrap().is_empty());
    }
}
