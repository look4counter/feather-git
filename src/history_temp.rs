use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Default)]
struct RegistryState {
    paths: HashSet<PathBuf>,
    shutting_down: bool,
}
#[derive(Default)]
struct Registry(Mutex<RegistryState>);

fn registry() -> Arc<Registry> {
    static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default).clone()
}

fn remove_export(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    #[allow(
        clippy::permissions_set_readonly_false,
        reason = "Windows 읽기 전용 속성만 해제한다"
    )]
    match std::fs::metadata(path) {
        Ok(metadata) => {
            let mut permissions = metadata.permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                std::fs::set_permissions(path, permissions)?;
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if let Some(parent) = path.parent() {
        match std::fs::remove_dir(parent) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

impl Registry {
    fn remove(state: &mut RegistryState, path: &Path) {
        match remove_export(path) {
            Ok(()) => {
                state.paths.remove(path);
            }
            Err(error) => eprintln!("파일 이력 임시파일 삭제 실패: {}: {error}", path.display()),
        }
    }

    fn shutdown(&self) {
        let mut state = self.0.lock().unwrap();
        state.shutting_down = true;
        for path in state.paths.clone() {
            Self::remove(&mut state, &path);
        }
    }
}

#[derive(Default)]
struct SessionState {
    closed: bool,
    paths: Vec<PathBuf>,
}

// 추출 작업도 세션을 소유하므로 취소된 작업의 반환값이 버려져도 추적한다.
pub(crate) struct Session {
    state: Mutex<SessionState>,
    registry: Arc<Registry>,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            state: Mutex::default(),
            registry: registry(),
        }
    }
}
impl Session {
    pub(crate) fn track(&self, path: PathBuf) {
        let mut session = self.state.lock().unwrap();
        let mut registry = self.registry.0.lock().unwrap();
        registry.paths.insert(path.clone());
        if session.closed || registry.shutting_down {
            Registry::remove(&mut registry, &path);
        } else {
            session.paths.push(path);
        }
    }

    pub(crate) fn close(&self) {
        let mut session = self.state.lock().unwrap();
        session.closed = true;
        let mut registry = self.registry.0.lock().unwrap();
        for path in session.paths.drain(..) {
            Registry::remove(&mut registry, &path);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

pub(crate) fn shutdown() {
    registry().shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(registry: &Arc<Registry>) -> Session {
        Session {
            state: Mutex::default(),
            registry: registry.clone(),
        }
    }
    fn exported(root: &Path, name: &str) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("revision.txt");
        std::fs::write(&path, b"original content").unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[test]
    fn close_cleans_readonly_exports_is_isolated_idempotent_and_handles_late_completion() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Arc::new(Registry::default());
        let first = session(&registry);
        let second = session(&registry);
        let a = exported(dir.path(), "a");
        let b = exported(dir.path(), "b");
        first.track(a.clone());
        second.track(b.clone());
        assert!(a.exists() && b.exists());
        first.close();
        first.close();
        assert!(!a.exists() && !a.parent().unwrap().exists());
        assert!(b.exists());
        let late = exported(dir.path(), "late");
        first.track(late.clone());
        assert!(!late.exists());
        drop(second);
        assert!(!b.exists());
        assert!(registry.0.lock().unwrap().paths.is_empty());
    }

    #[test]
    fn shutdown_cleans_active_session_and_late_exports() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Arc::new(Registry::default());
        let active = session(&registry);
        let path = exported(dir.path(), "active");
        active.track(path.clone());
        registry.shutdown();
        assert!(!path.exists());
        let late = exported(dir.path(), "late");
        active.track(late.clone());
        assert!(!late.exists());
        active.close();
        assert!(registry.0.lock().unwrap().paths.is_empty());
    }

    #[test]
    fn concurrent_close_and_registration_do_not_lose_export() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Arc::new(Registry::default());
        let session = Arc::new(session(&registry));
        let path = exported(dir.path(), "concurrent");
        let ready = Arc::new(std::sync::Barrier::new(2));
        let worker_session = session.clone();
        let worker_path = path.clone();
        let worker_ready = ready.clone();
        let worker = std::thread::spawn(move || {
            worker_ready.wait();
            worker_session.track(worker_path);
        });
        ready.wait();
        session.close();
        worker.join().unwrap();
        assert!(!path.exists());
        assert!(registry.0.lock().unwrap().paths.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn locked_export_is_retained_and_retried_at_shutdown() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let registry = Arc::new(Registry::default());
        let session = session(&registry);
        let path = exported(dir.path(), "locked");
        session.track(path.clone());
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        session.close();
        assert!(path.exists());
        assert!(registry.0.lock().unwrap().paths.contains(&path));
        drop(lock);
        registry.shutdown();
        assert!(!path.exists());
        assert!(!path.parent().unwrap().exists());
        assert!(registry.0.lock().unwrap().paths.is_empty());
    }
}
