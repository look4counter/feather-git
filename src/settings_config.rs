use anyhow::{Context, Result, bail};
use gix::bstr::ByteSlice;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn repo(path: &Path) -> Result<gix::Repository> {
    repo_with_options(path, open_options())
}

pub(crate) fn repo_with_options(
    path: &Path,
    options: gix::open::Options,
) -> Result<gix::Repository> {
    let mut repo = gix::discover_opts(path, Default::default(), options)?;
    let mut combined = gix::config::File::default();
    for path in system_configs() {
        combined.append(read_config_file(&path)?)?;
    }
    combined.append(repo.config_snapshot().plumbing().clone())?;
    let mut snapshot = repo.config_snapshot_mut();
    *snapshot = combined;
    snapshot.commit()?;
    Ok(repo)
}

pub fn open_options() -> gix::open::Options {
    let mut options = gix::open::Options::default();
    options.permissions.config.git_binary = false;
    options.permissions.config.system = false;
    options.permissions.attributes.git_binary = false;
    options.permissions.attributes.system = false;
    options
}

fn system_configs() -> Vec<PathBuf> {
    if std::env::var("GIT_CONFIG_NOSYSTEM")
        .is_ok_and(|s| ["1", "true", "yes"].contains(&s.as_str()))
    {
        return Vec::new();
    }
    if let Some(path) = std::env::var_os("GIT_CONFIG_SYSTEM") {
        return vec![PathBuf::from(path)]
            .into_iter()
            .filter(|p| p.is_file())
            .collect();
    }
    let mut paths = Vec::new();
    #[cfg(windows)]
    {
        if let Some(base) = std::env::var_os("ProgramData") {
            paths.push(PathBuf::from(base).join("Git/config"));
        }
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(base) = std::env::var_os(variable) {
                paths.push(PathBuf::from(base).join("Git/etc/gitconfig"));
            }
        }
        if let Some(search) = std::env::var_os("PATH") {
            for entry in std::env::split_paths(&search) {
                if entry.join("git.exe").is_file() {
                    for parent in entry.ancestors().take(3) {
                        paths.push(parent.join("etc/gitconfig"));
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    paths.push(PathBuf::from("/etc/gitconfig"));
    let mut seen = std::collections::BTreeSet::new();
    paths
        .into_iter()
        .filter(|p| p.is_file())
        .filter_map(|p| p.canonicalize().ok())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

pub fn config_path(
    path: &Path,
    global: bool,
    env: &BTreeMap<OsString, OsString>,
) -> Result<PathBuf> {
    if global {
        if let Some(path) = env
            .get(std::ffi::OsStr::new("GIT_CONFIG_GLOBAL"))
            .cloned()
            .or_else(|| std::env::var_os("GIT_CONFIG_GLOBAL"))
        {
            return Ok(path.into());
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .context("전역 설정 경로를 찾을 수 없습니다.")?;
        let home = PathBuf::from(home).join(".gitconfig");
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.parent().unwrap().join(".config"))
            .join("git/config");
        return Ok(if home.exists() || !xdg.exists() {
            home
        } else {
            xdg
        });
    }
    Ok(repo(path)?.common_dir().join("config"))
}

pub fn read_config_file(path: &Path) -> Result<gix::config::File> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    Ok(gix::config::File::from_bytes_no_includes(
        &bytes,
        gix::config::file::Metadata::from(gix::config::Source::Local),
        Default::default(),
    )?)
}

pub fn config_values(
    path: &Path,
    scope: Option<&str>,
    key: &str,
    env: &BTreeMap<OsString, OsString>,
) -> Result<Vec<String>> {
    let values = if scope.is_some() {
        read_config_file(&config_path(path, scope == Some("--global"), env)?)?
            .raw_values(key)
            .unwrap_or_default()
    } else if env.contains_key(std::ffi::OsStr::new("GIT_CONFIG_GLOBAL")) {
        let mut values = config_values(path, Some("--global"), key, env)?;
        if repo(path).is_ok() {
            values.extend(config_values(path, Some("--local"), key, env)?);
        }
        return Ok(values);
    } else if let Ok(repo) = repo(path) {
        repo.config_snapshot()
            .plumbing()
            .raw_values(key)
            .unwrap_or_default()
    } else {
        return config_values(path, Some("--global"), key, env);
    };
    values
        .into_iter()
        .map(|v| Ok(String::from_utf8(v.to_vec())?))
        .collect()
}

pub fn config_set(
    path: &Path,
    global: bool,
    key: &str,
    values: &[String],
    env: &BTreeMap<OsString, OsString>,
) -> Result<()> {
    edit_config(path, global, env, |config| {
        if let Ok(mut existing) = config.raw_values_mut(key) {
            existing.delete_all();
        }
        let (section, field) = key.rsplit_once('.').context("잘못된 설정 키입니다.")?;
        let (section, subsection) = section.split_once('.').map_or((section, None), |(s, sub)| {
            (s, Some(sub.as_bytes().as_bstr()))
        });
        for value in values {
            config
                .new_section(section, subsection.map(|s| s.to_owned()))?
                .push(field, Some(value.as_bytes().as_bstr()))?;
        }
        Ok(())
    })
}

pub fn add_remote(path: &Path, name: &str, url: &str) -> Result<()> {
    if name.chars().any(|c| c.is_control()) || url.chars().any(|c| c.is_control()) {
        bail!("원격 이름과 URL에는 제어문자를 사용할 수 없습니다.");
    }
    let name = name.trim();
    let url = url.trim();
    if name.is_empty()
        || !git2::Reference::is_valid_name(&format!("refs/remotes/{name}/branch"))
        || name.starts_with('-')
        || name.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        bail!("원격 이름을 올바르게 입력하세요.");
    }
    if url.is_empty() || url.starts_with('-') || url.chars().any(|c| c.is_control()) {
        bail!("원격 URL을 올바르게 입력하세요.");
    }
    edit_config(path, false, &BTreeMap::new(), |config| {
        let repo = repo(path)?;
        let snapshot = repo.config_snapshot();
        for file in [config as &gix::config::File, snapshot.plumbing()] {
            if file.sections_by_name("remote").is_some_and(|mut sections| {
                sections.any(|section| {
                    section.value_names().next().is_some()
                        && section
                            .header()
                            .subsection_name()
                            .is_some_and(|sub| sub == name.as_bytes())
                })
            }) {
                bail!("같은 이름의 원격 저장소가 이미 있습니다.");
            }
        }
        let fetch = format!("+refs/heads/*:refs/remotes/{name}/*");
        let mut section =
            config.new_section("remote", Some(name.as_bytes().as_bstr().to_owned()))?;
        section.push("url", Some(url.as_bytes().as_bstr()))?;
        section.push("fetch", Some(fetch.as_bytes().as_bstr()))?;
        Ok(())
    })
}

fn edit_config(
    path: &Path,
    global: bool,
    env: &BTreeMap<OsString, OsString>,
    edit: impl FnOnce(&mut gix::config::File) -> Result<()>,
) -> Result<()> {
    let path = config_path(path, global, env)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lock_name = path.as_os_str().to_owned();
    lock_name.push(".lock");
    let lock_path = PathBuf::from(lock_name);
    let mut lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .context("Git 설정 파일이 잠겨 있습니다.")?;
    let result = (|| {
        let mut config = read_config_file(&path)?;
        edit(&mut config)?;
        config.write_to(&mut lock)?;
        lock.flush()?;
        lock.sync_all()?;
        drop(lock);
        fs::rename(&lock_path, &path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&lock_path);
    }
    result
}
