use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub version: u8,
    pub tabs: Vec<Option<PathBuf>>,
    pub active: usize,
    pub recent: Vec<PathBuf>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: 1,
            tabs: vec![None],
            active: 0,
            recent: Vec::new(),
        }
    }
}

pub fn state_path() -> Result<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_STATE_HOME"))
        .context("사용자 앱 데이터 경로를 찾을 수 없습니다.")?;
    Ok(PathBuf::from(root)
        .join("FeatherGit")
        .join("workspace.json"))
}

pub fn load(path: &Path) -> Result<Session> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Session::default()),
        Err(error) => return Err(error).context("작업 공간 상태를 읽지 못했습니다."),
    };
    let mut state: Session =
        serde_json::from_slice(&bytes).context("작업 공간 상태 파일이 손상되었습니다.")?;
    if state.version != 1 {
        bail!("지원하지 않는 작업 공간 상태 버전입니다.");
    }
    if state.tabs.is_empty() {
        state.tabs.push(None);
    }
    state.active = state.active.min(state.tabs.len() - 1);
    let mut recent = Vec::new();
    for path in state.recent {
        if !recent.contains(&path) {
            recent.push(path);
        }
    }
    state.recent = recent;
    Ok(state)
}

pub fn restore_paths(state: &mut Session, arguments: Vec<PathBuf>) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tab in &mut state.tabs {
        if let Some(path) = tab
            && !seen.insert(path.clone())
        {
            *tab = None;
        }
    }
    for path in arguments {
        let root = path.canonicalize().unwrap_or_else(|_| {
            if path.is_absolute() {
                path.clone()
            } else {
                std::env::current_dir().unwrap_or_default().join(&path)
            }
        });
        remember(&mut state.recent, root.clone());
        if let Some(index) = state
            .tabs
            .iter()
            .position(|tab| tab.as_ref() == Some(&root))
        {
            state.active = index;
        } else {
            state.tabs.push(Some(root));
            state.active = state.tabs.len() - 1;
        }
    }
    if let Some(tab) = state.tabs.get_mut(state.active)
        && let Some(path) = tab.take()
    {
        match feather_git::git::repository_root(&path) {
            Ok(root) => *tab = Some(root),
            Err(error) => errors.push(format!("{}: {error:#}", display_path(&path))),
        }
    }
    errors
}

pub fn save(path: &Path, state: &Session) -> Result<()> {
    let parent = path.parent().context("상태 저장 경로가 잘못되었습니다.")?;
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(state)?;
    use std::io::Write;
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, path).context("작업 공간 상태를 저장하지 못했습니다.")
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    }
}

pub fn remember(recent: &mut Vec<PathBuf>, path: PathBuf) {
    recent.retain(|item| *item != path);
    recent.insert(0, path);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_active_tab_does_not_validate_inactive_repositories() {
        let temp = tempfile::tempdir().unwrap();
        let invalid = temp.path().join("not-a-repository");
        std::fs::write(&invalid, "ordinary file").unwrap();
        let mut state = Session {
            tabs: vec![
                None,
                Some(invalid.clone()),
                Some(temp.path().join("missing")),
            ],
            active: 0,
            ..Session::default()
        };
        let before = state.tabs.clone();
        assert!(restore_paths(&mut state, Vec::new()).is_empty());
        assert_eq!(state.tabs, before);
        state.active = 1;
        assert_eq!(restore_paths(&mut state, Vec::new()).len(), 1);
        assert!(state.tabs[1].is_none());
        assert_eq!(state.tabs[2], before[2]);
    }
    #[test]
    fn restoring_paths_defers_inactive_errors_and_merges_arguments() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        crate::setup::create(&first, None).unwrap();
        crate::setup::create(&second, None).unwrap();
        let mut state = Session {
            tabs: vec![
                Some(first.canonicalize().unwrap()),
                None,
                Some(temp.path().join("missing")),
                Some(first.canonicalize().unwrap()),
            ],
            active: 1,
            ..Session::default()
        };
        let errors = restore_paths(&mut state, vec![second.clone(), first.clone()]);
        assert!(errors.is_empty());
        assert_eq!(
            state.tabs,
            vec![
                Some(first.canonicalize().unwrap()),
                None,
                Some(temp.path().join("missing")),
                None,
                Some(second.canonicalize().unwrap())
            ]
        );
        assert_eq!(state.active, 0);
        assert_eq!(
            state.recent,
            vec![
                first.canonicalize().unwrap(),
                second.canonicalize().unwrap()
            ]
        );
        state.active = 2;
        assert_eq!(restore_paths(&mut state, Vec::new()).len(), 1);
        assert!(state.tabs[2].is_none());
    }
    #[test]
    fn session_roundtrip_preserves_empty_tabs_order_active_and_recent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("workspace.json");
        assert_eq!(load(&path).unwrap().tabs, vec![None]);
        let mut state = Session {
            tabs: vec![Some("한글 repo".into()), None, Some("other".into())],
            active: 1,
            ..Session::default()
        };
        remember(&mut state.recent, "other".into());
        remember(&mut state.recent, "한글 repo".into());
        remember(&mut state.recent, "other".into());
        save(&path, &state).unwrap();
        save(&path, &state).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(restored.tabs, state.tabs);
        assert_eq!(restored.active, 1);
        assert_eq!(
            restored.recent,
            vec![PathBuf::from("other"), PathBuf::from("한글 repo")]
        );
        std::fs::write(&path, "broken").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "broken");
    }
    #[test]
    fn display_hides_only_windows_extended_prefix() {
        assert_eq!(
            display_path(Path::new(r"\\?\D:\Project\zed")),
            r"D:\Project\zed"
        );
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\repo")),
            r"\\server\share\repo"
        );
        assert_eq!(display_path(Path::new(r"D:\repo")), r"D:\repo");
    }
}
