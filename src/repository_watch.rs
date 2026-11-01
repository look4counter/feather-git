use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Stage,
    Full,
}

#[derive(Clone, Copy, Debug)]
pub struct Batch {
    pub scope: Scope,
    pub first: Instant,
    pub last: Instant,
}

impl Batch {
    pub fn event(scope: Scope, now: Instant) -> Self {
        Self {
            scope,
            first: now,
            last: now,
        }
    }

    fn merge(&mut self, other: Self) {
        self.scope = self.scope.max(other.scope);
        self.first = self.first.min(other.first);
        self.last = self.last.max(other.last);
    }
}

#[derive(Default)]
pub struct Scheduler {
    pending: Option<Batch>,
    running: bool,
    started: Option<Instant>,
}

impl Scheduler {
    pub fn push(&mut self, batch: Batch) {
        if let Some(pending) = &mut self.pending {
            pending.merge(batch);
        } else {
            self.pending = Some(batch);
        }
    }

    pub fn begin(&mut self, now: Instant, active: bool, blocked: bool) -> Option<Scope> {
        if !active || blocked || self.running {
            return None;
        }
        let pending = self.pending?;
        let quiet = now.saturating_duration_since(pending.last) >= Duration::from_millis(500);
        let overdue = now.saturating_duration_since(pending.first) >= Duration::from_secs(2);
        let throttled = !quiet
            && self
                .started
                .is_some_and(|start| now.saturating_duration_since(start) < Duration::from_secs(2));
        if (!quiet && !overdue) || throttled {
            return None;
        }
        self.pending = None;
        self.running = true;
        self.started = Some(now);
        Some(pending.scope)
    }

    pub fn finish(&mut self) {
        self.running = false;
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use anyhow::{Context, Result};
    use std::{
        collections::BTreeSet,
        fs::{File, OpenOptions},
        os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
        path::{Path, PathBuf},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        thread::JoinHandle,
    };
    use windows::Win32::{
        Foundation::{CloseHandle, ERROR_NOTIFY_ENUM_DIR, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
            FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
            FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, ReadDirectoryChangesW,
        },
        System::{
            IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
            Threading::{CreateEventW, WaitForSingleObject},
        },
    };

    #[derive(Default)]
    struct Mailbox {
        batch: Option<Batch>,
        error: Option<String>,
    }

    fn publish(mailbox: &Mutex<Mailbox>, scope: Scope) {
        if let Ok(mut state) = mailbox.lock() {
            let next = Batch::event(scope, Instant::now());
            if let Some(batch) = &mut state.batch {
                batch.merge(next);
            } else {
                state.batch = Some(next);
            }
        }
    }

    fn metadata_scope(relative: &Path) -> Option<Scope> {
        let name = relative.to_string_lossy().replace('\\', "/");
        if name.ends_with(".lock") || name.starts_with("objects/") || name.starts_with("logs/") {
            return None;
        }
        if name == "index" || name == "info/exclude" || name == "info/attributes" {
            return Some(Scope::Stage);
        }
        if [
            "HEAD",
            "packed-refs",
            "config",
            "config.worktree",
            "commondir",
            "MERGE_HEAD",
            "MERGE_MSG",
            "REVERT_HEAD",
            "CHERRY_PICK_HEAD",
            "ORIG_HEAD",
        ]
        .contains(&name.as_str())
            || name.starts_with("refs/")
            || name.starts_with("rebase-merge")
            || name.starts_with("rebase-apply")
            || name.starts_with("sequencer")
            || name.starts_with("feather-stash-recovery")
        {
            return Some(Scope::Full);
        }
        None
    }

    struct Filter {
        repo: git2::Repository,
        workdir: Option<PathBuf>,
        gitdirs: Vec<PathBuf>,
        tracked: BTreeSet<String>,
    }

    impl Filter {
        fn new(path: &Path) -> Result<Self> {
            let repo = git2::Repository::discover(crate::native_git::ordinary_path(path))?;
            let workdir = repo.workdir().map(Path::to_path_buf);
            let mut gitdirs = vec![repo.path().to_path_buf(), repo.commondir().to_path_buf()];
            gitdirs.sort();
            gitdirs.dedup();
            gitdirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
            let mut filter = Self {
                repo,
                workdir,
                gitdirs,
                tracked: BTreeSet::new(),
            };
            filter.refresh_index()?;
            Ok(filter)
        }

        fn refresh_index(&mut self) -> Result<()> {
            let mut index = self.repo.index()?;
            index.read(true)?;
            self.tracked = index
                .iter()
                .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
                .collect();
            Ok(())
        }

        fn classify(&mut self, path: &Path) -> Option<Scope> {
            for root in &self.gitdirs {
                if let Ok(relative) = path.strip_prefix(root) {
                    let scope = metadata_scope(relative);
                    if relative == Path::new("index") {
                        let _ = self.refresh_index();
                    }
                    return scope;
                }
            }
            let relative = path.strip_prefix(self.workdir.as_ref()?).ok()?;
            if relative == Path::new(".git") {
                return Some(Scope::Full);
            }
            let name = relative.to_string_lossy().replace('\\', "/");
            let prefix = format!("{name}/");
            if self.tracked.contains(&name)
                || self
                    .tracked
                    .range(prefix.clone()..)
                    .next()
                    .is_some_and(|p| p.starts_with(&prefix))
            {
                return Some(Scope::Stage);
            }
            if relative
                .file_name()
                .is_some_and(|name| name == ".gitignore")
            {
                if let Some(parent) = relative.parent().filter(|p| !p.as_os_str().is_empty()) {
                    let parent = format!("{}/", parent.to_string_lossy().replace('\\', "/"));
                    if self.repo.is_path_ignored(parent).unwrap_or(false) {
                        return None;
                    }
                }
                return Some(Scope::Stage);
            }
            // Deleted directories can no longer be queried with is_dir(). Test both forms.
            let ignored = self.repo.is_path_ignored(&name).unwrap_or(false)
                || self.repo.is_path_ignored(&prefix).unwrap_or(false);
            (!ignored).then_some(Scope::Stage)
        }
    }

    pub struct Watcher {
        stop: Arc<AtomicBool>,
        mailbox: Arc<Mutex<Mailbox>>,
        threads: Vec<JoinHandle<()>>,
    }

    impl Watcher {
        pub fn start(path: &Path) -> Result<Self> {
            let filter = Filter::new(path)?;
            let mut roots = filter.gitdirs.clone();
            roots.extend(filter.workdir);
            roots.sort_by_key(|path| path.components().count());
            let mut unique: Vec<PathBuf> = Vec::new();
            for root in roots {
                if !unique.iter().any(|parent| root.starts_with(parent)) {
                    unique.push(root);
                }
            }
            let mut watcher = Self {
                stop: Arc::new(AtomicBool::new(false)),
                mailbox: Arc::new(Mutex::new(Mailbox::default())),
                threads: Vec::new(),
            };
            for root in unique {
                let file = OpenOptions::new()
                    .access_mode(FILE_LIST_DIRECTORY.0)
                    .share_mode(7)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OVERLAPPED.0)
                    .open(&root)
                    .with_context(|| {
                        format!("변경 감시를 시작할 수 없습니다: {}", root.display())
                    })?;
                let path = path.to_path_buf();
                let stop = watcher.stop.clone();
                let mailbox = watcher.mailbox.clone();
                // Arm the first read before returning, so immediate external edits are not lost.
                let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
                watcher.threads.push(std::thread::spawn(move || {
                    let result = watch(&path, &root, file, &stop, &mailbox, ready_tx);
                    if let Err(error) = result
                        && let Ok(mut state) = mailbox.lock()
                    {
                        state.error = Some(format!("외부 변경 감시에 실패했습니다: {error:#}"));
                    }
                }));
                ready_rx
                    .recv()
                    .context("변경 감시 스레드가 종료되었습니다.")?
                    .map_err(anyhow::Error::msg)?;
            }
            Ok(watcher)
        }

        pub fn drain(&self) -> (Option<Batch>, Option<String>) {
            let mut state = self
                .mailbox
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            (state.batch.take(), state.error.take())
        }
    }

    impl Drop for Watcher {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            for thread in self.threads.drain(..) {
                let _ = thread.join();
            }
        }
    }

    fn watch(
        path: &Path,
        root: &Path,
        file: File,
        stop: &AtomicBool,
        mailbox: &Mutex<Mailbox>,
        ready: std::sync::mpsc::SyncSender<Result<(), String>>,
    ) -> Result<()> {
        let mut filter = Filter::new(path)?;
        let handle = HANDLE(file.as_raw_handle());
        let event = unsafe { CreateEventW(None, false, false, None)? };
        let result = (|| -> Result<()> {
            let mut first = Some(ready);
            // u32 storage provides the alignment required by FILE_NOTIFY_INFORMATION.
            let mut buffer = vec![0u32; 16 * 1024];
            loop {
                let mut overlapped = OVERLAPPED {
                    hEvent: event,
                    ..Default::default()
                };
                let start = unsafe {
                    ReadDirectoryChangesW(
                        handle,
                        buffer.as_mut_ptr().cast(),
                        (buffer.len() * 4) as u32,
                        true,
                        FILE_NOTIFY_CHANGE_FILE_NAME
                            | FILE_NOTIFY_CHANGE_DIR_NAME
                            | FILE_NOTIFY_CHANGE_LAST_WRITE
                            | FILE_NOTIFY_CHANGE_SIZE,
                        None,
                        Some(&mut overlapped),
                        None,
                    )
                };
                if let Some(ready) = first.take() {
                    let _ = ready.send(start.as_ref().map(|_| ()).map_err(ToString::to_string));
                }
                start?;
                loop {
                    if stop.load(Ordering::Acquire) {
                        unsafe {
                            let _ = CancelIoEx(handle, Some(&overlapped));
                            let mut bytes = 0;
                            let _ = GetOverlappedResult(handle, &overlapped, &mut bytes, true);
                        }
                        return Ok(());
                    }
                    let status = unsafe { WaitForSingleObject(event, 100) };
                    if status == WAIT_TIMEOUT {
                        continue;
                    }
                    if status != WAIT_OBJECT_0 {
                        unsafe {
                            let _ = CancelIoEx(handle, Some(&overlapped));
                            let mut bytes = 0;
                            let _ = GetOverlappedResult(handle, &overlapped, &mut bytes, true);
                        }
                        anyhow::bail!("변경 감시 대기에 실패했습니다.");
                    }
                    break;
                }
                let mut bytes = 0;
                if let Err(error) =
                    unsafe { GetOverlappedResult(handle, &overlapped, &mut bytes, false) }
                {
                    if error.code() == windows::core::HRESULT::from_win32(ERROR_NOTIFY_ENUM_DIR.0) {
                        publish(mailbox, Scope::Full);
                        continue;
                    }
                    return Err(error.into());
                }
                if bytes == 0 {
                    publish(mailbox, Scope::Full);
                    continue;
                }
                let bytes = &unsafe {
                    std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), bytes as usize)
                };
                let mut offset = 0;
                while offset + 12 <= bytes.len() {
                    let read =
                        |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
                    let next = read(offset);
                    let action = read(offset + 4);
                    let length = read(offset + 8);
                    if offset + 12 + length > bytes.len() {
                        publish(mailbox, Scope::Full);
                        break;
                    }
                    let name: Vec<u16> = bytes[offset + 12..offset + 12 + length]
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();
                    use std::os::windows::ffi::OsStringExt;
                    let path = root.join(std::ffi::OsString::from_wide(&name));
                    // Child creation also changes directory timestamps. Its own event is enough.
                    if !(action == 3 && path.is_dir())
                        && let Some(scope) = filter.classify(&path)
                    {
                        publish(mailbox, scope);
                    }
                    if next == 0 {
                        break;
                    }
                    offset += next;
                }
            }
        })();
        unsafe {
            let _ = CloseHandle(event);
        }
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn repository() -> (tempfile::TempDir, git2::Repository) {
            let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
            let repo = git2::Repository::init(temp.path()).unwrap();
            (temp, repo)
        }

        fn wait(watcher: &Watcher, expected: Scope) {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let (batch, error) = watcher.drain();
                assert!(error.is_none(), "{error:?}");
                if batch.is_some_and(|batch| batch.scope == expected) {
                    return;
                }
                assert!(Instant::now() < deadline, "missing {expected:?} event");
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        #[test]
        fn real_watcher_filters_build_output_and_detects_files_index_refs_and_shutdown() {
            let (temp, repo) = repository();
            let root = temp.path();
            std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
            std::fs::create_dir(root.join("target")).unwrap();
            std::fs::write(root.join("target/tracked"), b"tracked").unwrap();
            let mut initial_index = repo.index().unwrap();
            initial_index.add_path(Path::new("target/tracked")).unwrap();
            initial_index.write().unwrap();
            let watcher = Watcher::start(root).unwrap();
            for i in 0..1000 {
                std::fs::write(root.join(format!("target/build-{i}")), b"ignored").unwrap();
            }
            std::fs::write(root.join("target/.gitignore"), b"generated build output").unwrap();
            std::thread::sleep(Duration::from_millis(300));
            let (batch, error) = watcher.drain();
            assert!(error.is_none(), "{error:?}");
            assert!(
                batch.is_none(),
                "ignored build output triggered refresh: {batch:?}"
            );
            std::fs::write(root.join("target/tracked"), b"changed tracked ignored file").unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::write(root.join("file.txt"), b"first").unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::rename(root.join("file.txt"), root.join("renamed.txt")).unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::remove_file(root.join("renamed.txt")).unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::write(root.join("tracked.txt"), b"tracked").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("tracked.txt")).unwrap();
            index.write().unwrap();
            wait(&watcher, Scope::Stage);
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
            repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
                .unwrap();
            wait(&watcher, Scope::Full);
            std::fs::write(root.join(".gitignore"), "target/\ntracked.txt\n").unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::write(root.join("tracked.txt"), b"tracked but ignored").unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::write(root.join(".gitignore"), "").unwrap();
            wait(&watcher, Scope::Stage);
            std::fs::write(root.join("target/build-0"), b"now visible").unwrap();
            wait(&watcher, Scope::Stage);
            let started = Instant::now();
            drop(watcher);
            assert!(started.elapsed() < Duration::from_secs(2));
        }

        #[test]
        fn linked_worktree_watches_own_index_and_common_references() {
            let (temp, repo) = repository();
            std::fs::write(temp.path().join("file"), "first").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("file")).unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
            let id = repo
                .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
                .unwrap();
            let worktree_path = temp.path().join("linked");
            let worktree = repo.worktree("linked", &worktree_path, None).unwrap();
            let linked = git2::Repository::open(worktree.path()).unwrap();
            let watcher = Watcher::start(worktree.path()).unwrap();
            let filter = Filter::new(worktree.path()).unwrap();
            assert_eq!(filter.gitdirs.len(), 2);
            let mut linked_index = linked.index().unwrap();
            linked_index.write().unwrap();
            wait(&watcher, Scope::Stage);
            repo.reference("refs/heads/external", id, false, "external")
                .unwrap();
            wait(&watcher, Scope::Full);
            drop(watcher);
        }

        #[test]
        fn metadata_classification_ignores_objects_logs_and_locks() {
            for path in [
                "objects/ab/123",
                "logs/HEAD",
                "index.lock",
                "refs/heads/main.lock",
            ] {
                assert_eq!(metadata_scope(Path::new(path)), None);
            }
            for path in [
                "HEAD",
                "refs/heads/main",
                "refs/remotes/origin/main",
                "packed-refs",
                "MERGE_HEAD",
            ] {
                assert_eq!(metadata_scope(Path::new(path)), Some(Scope::Full));
            }
            assert_eq!(metadata_scope(Path::new("index")), Some(Scope::Stage));
        }
    }
}

#[cfg(windows)]
pub use platform::Watcher;

#[cfg(not(windows))]
pub struct Watcher;

#[cfg(not(windows))]
impl Watcher {
    pub fn start(_: &std::path::Path) -> anyhow::Result<Self> {
        Ok(Self)
    }
    pub fn drain(&self) -> (Option<Batch>, Option<String>) {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_and_continuous_stream_are_bounded() {
        let start = Instant::now();
        let mut state = Scheduler::default();
        state.push(Batch::event(Scope::Stage, start));
        assert_eq!(
            state.begin(start + Duration::from_millis(499), true, false),
            None
        );
        assert_eq!(
            state.begin(start + Duration::from_millis(500), true, false),
            Some(Scope::Stage)
        );
        state.finish();
        for ms in (600..2600).step_by(100) {
            state.push(Batch::event(
                Scope::Stage,
                start + Duration::from_millis(ms),
            ));
            assert_eq!(
                state.begin(start + Duration::from_millis(ms), true, false),
                None
            );
        }
        assert_eq!(
            state.begin(start + Duration::from_millis(2600), true, false),
            Some(Scope::Stage)
        );
    }

    #[test]
    fn inactive_busy_and_inflight_changes_coalesce_without_loss() {
        let start = Instant::now();
        let now = start + Duration::from_secs(3);
        let mut state = Scheduler::default();
        state.push(Batch::event(Scope::Stage, start));
        assert_eq!(state.begin(now, false, false), None);
        assert_eq!(state.begin(now, true, true), None);
        assert_eq!(state.begin(now, true, false), Some(Scope::Stage));
        for _ in 0..1000 {
            state.push(Batch::event(Scope::Full, now));
        }
        assert_eq!(state.begin(now + Duration::from_secs(3), true, false), None);
        state.finish();
        assert_eq!(
            state.begin(now + Duration::from_secs(3), true, false),
            Some(Scope::Full)
        );
        state.finish();
        assert_eq!(state.begin(now + Duration::from_secs(4), true, false), None);
    }
}
