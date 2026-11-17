use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub const RECENT_LIMIT: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryEntry {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub version: u32,
    pub pull_mode: String,
    pub open: Vec<RepositoryEntry>,
    pub active: Option<PathBuf>,
    pub start_tab_open: bool,
    pub start_tab_active: bool,
    pub recent: Vec<RepositoryEntry>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: 1,
            pull_mode: "merge".into(),
            open: Vec::new(),
            active: None,
            start_tab_open: true,
            start_tab_active: true,
            recent: Vec::new(),
        }
    }
}

fn normalized_path(path: &Path) -> PathBuf {
    crate::repository::repository_identity(path)
}

pub fn remember(recent: &mut Vec<RepositoryEntry>, mut entry: RepositoryEntry) {
    entry.path = normalized_path(&entry.path);
    for old in recent.iter_mut() { old.path = normalized_path(&old.path); }
    remember_normalized(recent, entry);
}

pub fn remember_normalized(recent: &mut Vec<RepositoryEntry>, entry: RepositoryEntry) {
    recent.retain(|old| old.path != entry.path);
    recent.insert(0, entry);
    recent.truncate(RECENT_LIMIT);
}

pub fn settings_path() -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .context("사용자 설정 경로를 찾을 수 없습니다.")?;
    Ok(base.join("feather-git").join("workspace.json"))
}

pub fn read(path: &Path) -> Result<Session> {
    let bytes = match read_bytes(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Session::default()),
        Err(error) => return Err(error).context("작업 공간 설정을 읽을 수 없습니다."),
    };
    let mut session: Session =
        serde_json::from_slice(&bytes).context("작업 공간 설정이 손상되었습니다.")?;
    if session.version != 1 {
        bail!(
            "지원하지 않는 작업 공간 설정 버전입니다: {}",
            session.version
        );
    }
    let mut recent = Vec::new();
    for entry in session.recent.into_iter().rev() {
        remember(&mut recent, entry);
    }
    session.recent = recent;
    let mut open = Vec::new();
    for mut entry in session.open {
        entry.path = normalized_path(&entry.path);
        if !open.iter().any(|old: &RepositoryEntry| old.path == entry.path) { open.push(entry); }
    }
    session.open = open;
    session.active = session.active.as_deref().map(normalized_path);
    Ok(session)
}

pub fn write(path: &Path, session: &Session) -> Result<()> {
    write_bytes(path, &serde_json::to_vec_pretty(session)?)
}

pub fn read_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("작업 공간 설정 경로가 잘못되었습니다.")?;
    std::fs::create_dir_all(parent)?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temporary = path.with_extension(format!("{}-{nonce}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.context("작업 공간 설정을 저장할 수 없습니다.")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(path: impl Into<PathBuf>) -> RepositoryEntry {
        RepositoryEntry {
            name: "한글 저장소".into(),
            path: path.into(),
        }
    }
    #[test]
    fn legacy_worktree_paths_migrate_and_shared_worktrees_stay_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        git2::Repository::init(&root).unwrap();
        let second = root.join(".git2");
        let repo = git2::Repository::init_bare(&second).unwrap();
        repo.set_workdir(&root, false).unwrap();
        let mut config = repo.config().unwrap();
        config.set_bool("core.bare", false).unwrap();
        config.set_str("core.worktree", root.to_str().unwrap()).unwrap();
        let path = dir.path().join("session.json");
        let first = root.join(".git");
        let session = Session {
            open: vec![entry(&root), entry(&first), entry(&second)],
            recent: vec![entry(&root), entry(&first), entry(&second)],
            active: Some(root.clone()),
            ..Default::default()
        };
        write(&path, &session).unwrap();
        let loaded = read(&path).unwrap();
        assert_eq!(loaded.open.len(), 2);
        assert_eq!(loaded.recent.len(), 2);
        assert_eq!(loaded.active, Some(first.canonicalize().unwrap()));
        assert!(loaded.recent.iter().any(|item| item.path == second.canonicalize().unwrap()));
    }

    #[test]
    fn pull_mode_defaults_for_old_json_and_survives_serialization() {
        let old:Session=serde_json::from_str(r#"{"version":1,"open":[],"recent":[]}"#).unwrap();
        assert_eq!(old.pull_mode,"merge");
        let selected=Session{pull_mode:"ff-only".into(),..Default::default()};
        let restored:Session=serde_json::from_slice(&serde_json::to_vec(&selected).unwrap()).unwrap();
        assert_eq!(restored.pull_mode,"ff-only");assert_eq!(restored.version,1);
    }
    #[test]
    fn state_round_trip_replaces_existing_file_and_keeps_unicode_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("설정 폴더/workspace.json");
        assert!(read(&path).unwrap().open.is_empty());
        let one = entry(dir.path().join("첫 번째"));
        let two = entry(dir.path().join("second"));
        let mut state = Session {
            open: vec![one.clone(), two.clone()],
            active: Some(one.path.clone()),
            recent: vec![two, one],
            start_tab_open: false,
            start_tab_active: false,
            ..Default::default()
        };
        write(&path, &state).unwrap();
        let loaded = read(&path).unwrap();
        assert_eq!(loaded.open, state.open);
        assert_eq!(loaded.active, state.active);
        assert_eq!(loaded.recent, state.recent);
        state.open.remove(0);
        write(&path, &state).unwrap();
        assert_eq!(read(&path).unwrap().open, state.open);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
    }
    #[test]
    fn recent_is_unique_capped_and_reordered_on_reuse() {
        let dir = tempfile::tempdir().unwrap();
        let mut recent = Vec::new();
        for i in 0..25 {
            remember(&mut recent, entry(dir.path().join(i.to_string())));
        }
        assert_eq!(recent.len(), RECENT_LIMIT);
        let last = recent.last().unwrap().clone();
        remember(&mut recent, last.clone());
        assert_eq!(recent[0], last);
        assert_eq!(recent.len(), RECENT_LIMIT);
        let real = entry(dir.path().to_path_buf());
        remember(&mut recent, real);
        remember(&mut recent, entry(dir.path().join(".")));
        assert_eq!(
            recent
                .iter()
                .filter(|e| e.path == dir.path().canonicalize().unwrap())
                .count(),
            1
        );
    }
    #[test]
    fn corrupt_unsupported_and_unreadable_settings_report_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(&path, "{broken").unwrap();
        assert!(read(&path).is_err());
        std::fs::write(&path, r#"{"version":99}"#).unwrap();
        assert!(read(&path).is_err());
        assert!(read(dir.path()).is_err());
    }
    #[test]
    fn failed_replacement_preserves_destination_and_cleans_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("keep"), "original").unwrap();
        assert!(write(&path, &Session::default()).is_err());
        assert_eq!(
            std::fs::read_to_string(path.join("keep")).unwrap(),
            "original"
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        let file = dir.path().join("file");
        std::fs::write(&file, "keep").unwrap();
        assert!(write(&file.join("state.json"), &Session::default()).is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "keep");
    }
}
