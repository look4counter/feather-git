use crate::{
    diff_git,
    diff_model::FileChange,
    history::PreparedHistory,
    repository::{self, ChangedFile, Commit, Project},
    session::{self, Session},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, OnceLock, mpsc},
};

pub struct FileLocation {
    pub path: PathBuf,
}

/// OS 동작에 필요한 파일 상태 확인은 작업 스레드에서 수행합니다.
pub fn prepare_file_location(
    root: &Path,
    raw: &[u8],
    reveal: bool,
) -> Result<FileLocation, String> {
    let workdir = repository::worktree_directory(root).unwrap_or_else(|_| root.to_path_buf());
    let path = crate::path_display::absolute_file_path(&workdir, raw)?;
    if !reveal {
        std::fs::metadata(&path)
            .map_err(|error| format!("{}: {error}", crate::path_display::display_path(&path)))?;
        return Ok(FileLocation { path });
    }
    for parent in path.ancestors().skip(1) {
        match std::fs::metadata(parent) {
            Ok(metadata) if metadata.is_dir() => {
                return Ok(FileLocation {
                    path: parent.into(),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "{}: {error}",
                    crate::path_display::display_path(parent)
                ));
            }
        }
    }
    Err("파일 위치를 열 수 없습니다.".into())
}

#[derive(Clone)]
pub struct PanelRequest {
    pub revision: u64,
    pub project: usize,
    pub path: Option<PathBuf>,
    pub historical: bool,
    pub commit: Option<String>,
    pub graph: Option<Vec<Commit>>,
    pub selected_files: Vec<(Vec<u8>, bool)>,
}
pub struct PanelResult {
    pub work_state: Result<repository::RepositoryWorkState, String>,
    pub request: PanelRequest,
    pub files: Result<Vec<FileChange>, String>,
    pub history: Option<PreparedHistory>,
    pub displayed_files: Result<Vec<ChangedFile>, String>,
}
impl PanelRequest {
    pub fn execute(self) -> PanelResult {
        let files = match (&self.path, self.historical, &self.commit) {
            (Some(path), true, Some(commit)) => diff_git::load_history(path, commit),
            (Some(path), false, _) => diff_git::load_stage(path),
            (Some(_), true, None) => Ok(Vec::new()),
            (None, _, _) => Err(anyhow::anyhow!("저장소 경로가 없습니다.")),
        }
        .map_err(|error| format!("목록을 읽을 수 없습니다: {error:#}"));
        let history = self
            .graph
            .as_ref()
            .map(|commits| PreparedHistory::new(commits));
        let displayed_files = files
            .as_ref()
            .map(|files| changed_files(files, self.historical))
            .map_err(Clone::clone);
        let work_state = self.path.as_ref().ok_or_else(|| "저장소 경로가 없습니다.".to_owned())
            .and_then(|path| repository::work_state(path).map_err(|error| format!("{error:#}")));
        PanelResult {
            work_state,
            request: self,
            files,
            history,
            displayed_files,
        }
    }
}

#[derive(Clone)]
pub struct RepositoryListRequest {
    pub revision: u64,
    pub project: usize,
    pub path: PathBuf,
    pub stashes: bool,
}

pub struct RepositoryListResult {
    pub request: RepositoryListRequest,
    pub names: Result<Vec<String>, String>,
}

impl RepositoryListRequest {
    pub fn execute(self) -> RepositoryListResult {
        let names = repository::panel_references(&self.path, self.stashes)
            .map_err(|error| format!("목록을 읽을 수 없습니다: {error:#}"));
        RepositoryListResult {
            request: self,
            names,
        }
    }
}

pub fn changed_files(files: &[FileChange], historical: bool) -> Vec<ChangedFile> {
    files
        .iter()
        .map(|file| ChangedFile {
            status: if file.staged && !historical {
                format!("S·{}", file.status)
            } else {
                file.status.clone()
            },
            path: file.label.clone(),
        })
        .collect()
}

pub struct FileStageResult {
    pub succeeded: usize,
    pub selected_files: Vec<(Vec<u8>, bool)>,
    pub errors: Vec<String>,
}

pub fn stage_selection(files: &[FileChange]) -> Vec<FileChange> {
    let mut selected: Vec<FileChange> = Vec::new();
    let mut positions = std::collections::HashMap::new();
    for file in files {
        if let Some(&index) = positions.get(&file.path) {
            if !file.staged {
                selected[index] = file.clone();
            }
        } else {
            positions.insert(file.path.clone(), selected.len());
            selected.push(file.clone());
        }
    }
    selected
}

pub fn toggle_selected_files(root: &Path, files: &[FileChange]) -> FileStageResult {
    let mut result = FileStageResult {
        succeeded: 0,
        selected_files: Vec::new(),
        errors: Vec::new(),
    };
    for file in stage_selection(files) {
        match diff_git::toggle_stage(root, &file) {
            Ok(()) => {
                result.succeeded += 1;
                result.selected_files.push((file.path, !file.staged));
            }
            Err(error) => {
                result.errors.push(format!("{}: {error:#}", file.label));
                result.selected_files.push((file.path, file.staged));
            }
        }
    }
    result
}

#[derive(Clone)]
pub struct ReferenceRequest {
    pub revision: u64,
    pub project: usize,
    pub path: PathBuf,
    pub name: String,
    pub full_name: String,
    pub row: usize,
    pub tab: usize,
}
impl ReferenceRequest {
    pub fn execute(&self) -> Result<(Project, PreparedHistory), String> {
        let project = repository::reference_snapshot(&self.path, &self.name, &self.full_name)
            .map_err(|error| format!("커밋을 선택할 수 없습니다: {error:#}"))?;
        let history = PreparedHistory::new(&project.commits);
        Ok((project, history))
    }
}

type SaveResult = Result<(), String>;
type Write = dyn Fn(&Path, &Session) -> SaveResult + Send + Sync;
struct SaveJob {
    path: PathBuf,
    session: Session,
    complete: mpsc::Sender<SaveResult>,
}
#[derive(Default)]
struct WriterState {
    pending: Option<SaveJob>,
    running: bool,
}
pub struct SessionWriter {
    state: Mutex<WriterState>,
    drained: Condvar,
    write: Box<Write>,
}
impl Default for SessionWriter {
    fn default() -> Self {
        Self::with_write(|path, session| {
            session::write(path, session).map_err(|error| format!("작업 공간 저장 실패: {error:#}"))
        })
    }
}
impl SessionWriter {
    fn with_write(write: impl Fn(&Path, &Session) -> SaveResult + Send + Sync + 'static) -> Self {
        Self {
            state: Mutex::new(WriterState::default()),
            drained: Condvar::new(),
            write: Box::new(write),
        }
    }
    pub fn enqueue(
        self: &Arc<Self>,
        path: PathBuf,
        session: Session,
    ) -> mpsc::Receiver<SaveResult> {
        let (complete, result) = mpsc::channel();
        let mut state = self.state.lock().expect("세션 작성기 잠금");
        state.pending = Some(SaveJob {
            path,
            session,
            complete,
        });
        if !state.running {
            state.running = true;
            let writer = self.clone();
            std::thread::spawn(move || writer.drain());
        }
        result
    }
    fn drain(&self) {
        loop {
            let job = {
                let mut state = self.state.lock().expect("세션 작성기 잠금");
                match state.pending.take() {
                    Some(job) => job,
                    None => {
                        state.running = false;
                        self.drained.notify_all();
                        return;
                    }
                }
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (self.write)(&job.path, &job.session)
            }))
            .unwrap_or_else(|_| Err("작업 공간 저장 작업이 중단되었습니다.".into()));
            let _ = job.complete.send(result);
        }
    }
    pub fn flush(&self) {
        let mut state = self.state.lock().expect("세션 작성기 잠금");
        while state.running {
            state = self.drained.wait(state).expect("세션 저장 완료 대기");
        }
    }
}
pub fn shared_writer() -> Arc<SessionWriter> {
    static WRITER: OnceLock<Arc<SessionWriter>> = OnceLock::new();
    WRITER
        .get_or_init(|| Arc::new(SessionWriter::default()))
        .clone()
}

pub struct RestoredWorkspace {
    pub saved: Session,
    pub projects: Vec<Project>,
    pub project: usize,
    pub viewer_files: Result<Vec<FileChange>, String>,
    pub warning: Option<String>,
    pub history: PreparedHistory,
}

impl RestoredWorkspace {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        let saved = session::read(path).map_err(|error| format!("{error:#}"))?;
        let mut projects: Vec<Project> = Vec::new();
        let mut failures = Vec::new();
        for entry in &saved.open {
            match repository::execute(repository::Request {
                operation: repository::Operation::Open,
                source: String::new(),
                name: entry.name.clone(),
                path: entry.path.to_string_lossy().into_owned(),
            }) {
                Ok(project) => {
                    if !projects.iter().any(|old| old.path == project.path) {
                        projects.push(project);
                    }
                }
                Err(error) => failures.push(format!("{}: {error:#}", entry.name)),
            }
        }
        let active = saved
            .active
            .as_ref()
            .map(|path| path.canonicalize().unwrap_or_else(|_| path.clone()));
        let project = projects
            .iter()
            .position(|project| project.path == active)
            .unwrap_or_else(|| projects.len().saturating_sub(1));
        let viewer_files = match projects
            .get(project)
            .and_then(|project| project.path.as_ref())
        {
            Some(path) => diff_git::load_stage(path)
                .map_err(|error| format!("목록을 읽을 수 없습니다: {error:#}")),
            None => Ok(Vec::new()),
        };
        let warning = (!failures.is_empty()).then(|| {
            format!(
                "일부 저장소를 복원하지 못했습니다: {}",
                failures.join(" / ")
            )
        });
        let history = projects
            .get(project)
            .map(|p| PreparedHistory::new(&p.commits))
            .unwrap_or_else(PreparedHistory::empty);
        Ok(Self {
            history,
            saved,
            projects,
            project,
            viewer_files,
            warning,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    #[test]
    fn git_directory_identity_restores_two_repositories_and_file_locations_use_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        let mut projects = Vec::new();
        for name in [".git", ".git2"] {
            let project = repository::execute(repository::Request {
                operation: repository::Operation::Create, source: String::new(),
                path: root.join(name).to_string_lossy().into_owned(), name: "project".into(),
            }).unwrap();
            projects.push(project);
        }
        std::fs::write(root.join("file.txt"), "keep").unwrap();
        let entries: Vec<_> = projects.iter().map(|project| session::RepositoryEntry {
            name: project.name.clone(), path: project.path.clone().unwrap(),
        }).collect();
        let path = dir.path().join("workspace.json");
        session::write(&path, &Session {
            open: entries.clone(), recent: entries.clone(), active: Some(entries[0].path.clone()),
            ..Default::default()
        }).unwrap();
        let restored = RestoredWorkspace::read(&path).unwrap();
        assert_eq!(restored.projects.len(), 2);
        assert_eq!(restored.project, 0);
        assert_eq!(restored.saved.recent.len(), 2);
        for project in &restored.projects {
            let gitdir = project.path.as_ref().unwrap();
            assert_eq!(prepare_file_location(gitdir, b"file.txt", false).unwrap().path,
                root.canonicalize().unwrap().join("file.txt"));
            assert_eq!(execute_directory_action(gitdir, DirectoryAction::Explorer).unwrap(),
                Some(PathBuf::from(crate::path_display::display_path(&root.canonicalize().unwrap()))));
        }
    }

    #[test]
    fn file_locations_open_exact_files_and_reveal_missing_files_in_existing_parent() {
        let parent = std::env::current_dir()
            .unwrap()
            .join("target/file-location-tests");
        std::fs::create_dir_all(&parent).unwrap();
        let dir = tempfile::tempdir_in(parent).unwrap();
        let root = dir.path();
        let name = "한 글[1].txt";
        std::fs::write(root.join(name), "content").unwrap();
        let target = super::prepare_file_location(root, name.as_bytes(), false).unwrap();
        assert_eq!(target.path, root.join(name));
        let target = super::prepare_file_location(root, name.as_bytes(), true).unwrap();
        assert_eq!(target.path, root);
        assert!(super::prepare_file_location(root, b"missing/nested/file.txt", false).is_err());
        let target = super::prepare_file_location(root, b"missing/nested/file.txt", true).unwrap();
        assert_eq!(target.path, root);
        assert!(super::prepare_file_location(root, b"../outside", true).is_err());
    }

    #[test]
    fn batch_stage_toggles_all_files_prefers_stage_for_duplicates_and_reports_failures() {
        let fixture_root = std::env::current_dir()
            .unwrap()
            .join("target/batch-stage-tests");
        std::fs::create_dir_all(&fixture_root).unwrap();
        let dir = tempfile::tempdir_in(fixture_root).unwrap();
        let root = dir.path();
        let repo = git2::Repository::init(root).unwrap();
        repo.config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        for name in ["a.txt", "b.txt"] {
            std::fs::write(root.join(name), "base\n").unwrap();
        }
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "base", &tree, &[])
            .unwrap();
        std::fs::write(root.join("a.txt"), "staged\n").unwrap();
        index.add_path(Path::new("a.txt")).unwrap();
        index.write().unwrap();
        std::fs::write(root.join("a.txt"), "all a changes\n").unwrap();
        std::fs::write(root.join("b.txt"), "all b changes\n").unwrap();
        let files = diff_git::load_stage(root).unwrap();
        assert_eq!(files.len(), 3);
        let selected = stage_selection(&files);
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|file| !file.staged));
        let result = toggle_selected_files(root, &files);
        assert_eq!(result.succeeded, 2);
        assert!(result.errors.is_empty());
        assert!(result.selected_files.iter().all(|(_, staged)| *staged));
        let staged = diff_git::load_stage(root).unwrap();
        assert_eq!(staged.len(), 2);
        assert!(staged.iter().all(|file| file.staged));
        let result = toggle_selected_files(root, &staged);
        assert_eq!(result.succeeded, 2);
        assert!(result.errors.is_empty());
        let unstaged = diff_git::load_stage(root).unwrap();
        assert_eq!(unstaged.len(), 2);
        assert!(unstaged.iter().all(|file| !file.staged));
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        index.add_path(Path::new("a.txt")).unwrap();
        index.write().unwrap();
        let result = toggle_selected_files(root, &unstaged);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("a.txt"));
        assert_eq!(
            std::fs::read(root.join("a.txt")).unwrap(),
            b"all a changes\n"
        );
        assert_eq!(
            std::fs::read(root.join("b.txt")).unwrap(),
            b"all b changes\n"
        );
        assert!(
            diff_git::load_stage(root)
                .unwrap()
                .iter()
                .all(|file| file.staged)
        );
    }

    #[test]
    fn repository_list_requests_only_query_references_for_success_and_error() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/repository-list-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repository_path = dir.path().join("repository");
        git2::Repository::init(&repository_path).unwrap();
        let ui = std::thread::current().id();
        for (path, missing) in [(repository_path, false), (dir.path().join("missing"), true)] {
            for stashes in [false, true] {
                let request = RepositoryListRequest {
                    revision: 7,
                    project: 3,
                    path: path.clone(),
                    stashes,
                };
                let result = std::thread::spawn(move || {
                    crate::backend::with_test_delay(1, ui, || {
                        let result = request.execute();
                        assert_eq!(
                            crate::backend::test_calls(),
                            vec![("repository::panel_references", false)]
                        );
                        result
                    })
                })
                .join()
                .unwrap();
                assert_eq!(result.names.is_err(), missing);
                assert_eq!(result.request.revision, 7);
                assert_eq!(result.request.project, 3);
                assert_eq!(result.request.stashes, stashes);
            }
        }
    }

    fn snapshot(name: &str) -> Session {
        Session {
            active: Some(PathBuf::from(name)),
            ..Default::default()
        }
    }

    #[test]
    fn writer_is_serial_merges_pending_values_and_flushes_latest_after_ui_drop() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let (entered, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let wait = Mutex::new(wait);
        let writes = seen.clone();
        let executing = active.clone();
        let writer = Arc::new(SessionWriter::with_write(move |_, session| {
            assert_eq!(executing.fetch_add(1, Ordering::SeqCst), 0);
            let value = session.active.as_ref().unwrap().display().to_string();
            writes.lock().unwrap().push(value.clone());
            if value == "first" {
                entered.send(()).unwrap();
                wait.lock().unwrap().recv().unwrap();
            }
            executing.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        }));
        let first = writer.enqueue(PathBuf::from("settings"), snapshot("first"));
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        let superseded = writer.enqueue(PathBuf::from("settings"), snapshot("middle"));
        let last = writer.enqueue(PathBuf::from("settings"), snapshot("last"));
        assert!(matches!(
            superseded.recv_timeout(Duration::from_secs(1)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ));
        // UI result receivers may disappear; the worker still persists the latest state.
        drop(first);
        release.send(()).unwrap();
        writer.flush();
        assert_eq!(last.recv().unwrap(), Ok(()));
        assert_eq!(*seen.lock().unwrap(), vec!["first", "last"]);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn writer_reports_errors_and_panics_and_can_process_later_saves() {
        let counter = AtomicUsize::new(0);
        let writer = Arc::new(SessionWriter::with_write(move |_, _| {
            match counter.fetch_add(1, Ordering::SeqCst) {
                0 => Err("write failure".into()),
                1 => panic!("test write panic"),
                _ => Ok(()),
            }
        }));
        let first = writer.enqueue(PathBuf::from("settings"), snapshot("first"));
        assert_eq!(
            first.recv_timeout(Duration::from_secs(3)).unwrap(),
            Err("write failure".into())
        );
        let second = writer.enqueue(PathBuf::from("settings"), snapshot("second"));
        assert!(
            second
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .is_err()
        );
        let third = writer.enqueue(PathBuf::from("settings"), snapshot("third"));
        assert_eq!(third.recv_timeout(Duration::from_secs(3)).unwrap(), Ok(()));
        writer.flush();
    }

    #[test]
    fn writer_runs_on_another_thread_and_submission_does_not_wait_for_io() {
        let ui = std::thread::current().id();
        let (entered, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let wait = Mutex::new(wait);
        let writer = Arc::new(SessionWriter::with_write(move |_, _| {
            entered.send(std::thread::current().id()).unwrap();
            wait.lock().unwrap().recv().unwrap();
            Ok(())
        }));
        let result = writer.enqueue(PathBuf::from("settings"), Session::default());
        assert_ne!(ready.recv_timeout(Duration::from_secs(3)).unwrap(), ui);
        assert!(matches!(result.try_recv(), Err(mpsc::TryRecvError::Empty)));
        release.send(()).unwrap();
        assert_eq!(result.recv_timeout(Duration::from_secs(3)).unwrap(), Ok(()));
        writer.flush();
    }
}

#[derive(Clone)]
pub struct HistoryPageRequest {
    pub project: usize,
    pub offset: usize,
    pub cursor: repository::HistoryCursor,
    pub builder: crate::graph::GraphBuilder,
}
pub struct HistoryPageResult {
    pub request: HistoryPageRequest,
    pub result: Result<(repository::HistoryPage, PreparedHistory), String>,
}
impl HistoryPageRequest {
    pub fn matches(&self, other: &Self) -> bool {
        self.project == other.project
            && self.offset == other.offset
            && self.cursor.same_session(&other.cursor)
    }
    pub fn execute(self) -> HistoryPageResult {
        let result = repository::history_page(&self.cursor, self.offset)
            .map(|page| {
                let prepared = PreparedHistory::with_builder(&page.commits, self.builder.clone());
                (page, prepared)
            })
            .map_err(|error| format!("커밋을 더 읽을 수 없습니다: {error:#}"));
        HistoryPageResult {
            request: self,
            result,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryAction {
    GitBash,
    PowerShell,
    Explorer,
}

pub fn prepare_repository_directory(root: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::metadata(root)
        .map_err(|error| format!("저장소 폴더를 확인할 수 없습니다: {error}"))?;
    if !metadata.is_dir() {
        return Err("저장소 경로가 폴더가 아닙니다.".into());
    }
    Ok(PathBuf::from(crate::path_display::display_path(root)))
}

#[cfg(windows)]
fn configure_repository_shell(command: &mut std::process::Command, selected: &Path) -> Result<(), String> {
    let repo = match git2::Repository::open_ext(selected, git2::RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>()) {
        Ok(repo) => repo,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(()),
        Err(error) => return Err(format!("터미널에서 열 저장소를 확인할 수 없습니다: {error}")),
    };
    let gitdir = repo.path().canonicalize().map_err(|error| error.to_string())?;
    command.env("GIT_DIR", crate::path_display::display_path(&gitdir))
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR");
    if let Some(workdir) = repo.workdir() {
        let workdir = workdir.canonicalize().map_err(|error| error.to_string())?;
        command.current_dir(PathBuf::from(crate::path_display::display_path(&workdir)))
            .env("GIT_WORK_TREE", crate::path_display::display_path(&workdir));
    } else {
        command.current_dir(PathBuf::from(crate::path_display::display_path(&gitdir)))
            .env_remove("GIT_WORK_TREE");
    }
    Ok(())
}

#[cfg(windows)]
fn powershell_command(root: &Path) -> Result<std::process::Command, String> {
    use std::os::windows::process::CommandExt;
    let system =
        std::env::var_os("SystemRoot").ok_or("Windows 시스템 폴더를 확인할 수 없습니다.")?;
    let executable = PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = std::process::Command::new(executable);
    command
        .args(["-NoLogo", "-NoProfile", "-NoExit"])
        .current_dir(root);
    configure_repository_shell(&mut command, root)?;
    command.creation_flags(windows::Win32::System::Threading::CREATE_NEW_CONSOLE.0);
    Ok(command)
}

#[cfg(windows)]
fn git_bash_candidates(path_entries: &[PathBuf], install_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for entry in path_entries.iter().filter(|path| path.is_absolute()) {
        candidates.push(entry.join("git-bash.exe"));
        if entry.join("git.exe").is_file() {
            candidates.extend(
                entry
                    .ancestors()
                    .skip(1)
                    .take(2)
                    .map(|parent| parent.join("git-bash.exe")),
            );
        }
    }
    candidates.extend(install_dirs.iter().map(|root| root.join("git-bash.exe")));
    candidates
}

#[cfg(windows)]
fn find_git_bash(candidates: Vec<PathBuf>) -> Result<PathBuf, String> {
    candidates
        .into_iter()
        .find(|path| path.is_absolute() && path.is_file())
        .ok_or_else(|| {
            "Git Bash를 찾을 수 없습니다. Git for Windows 설치와 PATH 설정을 확인하세요.".into()
        })
}

#[cfg(windows)]
fn locate_git_bash() -> Result<PathBuf, String> {
    let paths = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut roots = ["ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|root| PathBuf::from(root).join("Git"))
        .collect::<Vec<_>>();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Programs/Git"));
    }
    find_git_bash(git_bash_candidates(&paths, &roots))
}

#[cfg(windows)]
fn git_bash_command(root: &Path, executable: &Path) -> Result<std::process::Command, String> {
    let mut command = std::process::Command::new(executable);
    // 런처의 기본 동작으로 현재 폴더를 유지하며 경로를 명령 인수로 만들지 않는다.
    command.current_dir(root);
    configure_repository_shell(&mut command, root)?;
    Ok(command)
}

pub fn execute_directory_action(
    root: &Path,
    action: DirectoryAction,
) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    let selected = root;
    let workdir = repository::worktree_directory(root).unwrap_or_else(|_| root.to_path_buf());
    let root = prepare_repository_directory(&workdir)?;
    if action == DirectoryAction::Explorer {
        return Ok(Some(root));
    }
    #[cfg(windows)]
    {
        let (mut command, label) = match action {
            DirectoryAction::GitBash => (git_bash_command(selected, &locate_git_bash()?)?, "Git Bash"),
            DirectoryAction::PowerShell => (powershell_command(selected)?, "PowerShell"),
            DirectoryAction::Explorer => return Ok(Some(root)),
        };
        command
            .spawn()
            .map_err(|error| format!("{label} 실행에 실패했습니다: {error}"))?;
        Ok(None)
    }
    #[cfg(not(windows))]
    {
        Err("이 플랫폼에서는 Git Bash/PowerShell 실행을 지원하지 않습니다.".into())
    }
}

#[cfg(test)]
mod directory_action_tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn shell_commands_keep_selected_git_directory_and_clear_other_git_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("한글 폴더 & 이름 (test)");
        let default = git2::Repository::init(&root).unwrap();
        let gitdir = root.join(".git2");
        let project = repository::execute(repository::Request {
            operation: repository::Operation::Create, source: String::new(),
            path: gitdir.to_string_lossy().into_owned(), name: "selected".into(),
        }).unwrap();
        let selected = project.path.unwrap();
        let commands = [powershell_command(&selected).unwrap(), git_bash_command(&selected, Path::new("git-bash.exe")).unwrap()];
        for command in commands {
            let env: std::collections::BTreeMap<_, _> = command.get_envs().collect();
            assert_eq!(env.get(std::ffi::OsStr::new("GIT_DIR")), Some(&Some(std::ffi::OsStr::new(&crate::path_display::display_path(&selected)))));
            assert_eq!(env.get(std::ffi::OsStr::new("GIT_WORK_TREE")), Some(&Some(std::ffi::OsStr::new(&crate::path_display::display_path(project.workdir.as_ref().unwrap())))));
            for key in ["GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
                assert_eq!(env.get(std::ffi::OsStr::new(key)), Some(&None));
            }
            let mut child = std::process::Command::new("git");
            child.current_dir(command.get_current_dir().unwrap());
            child.env("GIT_DIR", default.path()).env("GIT_INDEX_FILE", default.path().join("index"));
            for (key, value) in command.get_envs() {
                if let Some(value) = value { child.env(key, value); } else { child.env_remove(key); }
            }
            let output = child.args(["rev-parse", "--absolute-git-dir"]).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let resolved = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
            assert_eq!(resolved.canonicalize().unwrap(), selected);
        }
    }

    #[cfg(windows)]
    #[test]
    fn shell_commands_preserve_bare_repository_context_without_a_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init_bare(dir.path().join("bare.git")).unwrap();
        for command in [powershell_command(repo.path()).unwrap(), git_bash_command(repo.path(), Path::new("git-bash.exe")).unwrap()] {
            let env: std::collections::BTreeMap<_, _> = command.get_envs().collect();
            assert_eq!(env.get(std::ffi::OsStr::new("GIT_WORK_TREE")), Some(&None));
            let mut child = std::process::Command::new("git");
            child.current_dir(command.get_current_dir().unwrap()).env("GIT_WORK_TREE", dir.path());
            for (key, value) in command.get_envs() {
                if let Some(value) = value { child.env(key, value); } else { child.env_remove(key); }
            }
            let output = child.args(["rev-parse", "--is-bare-repository"]).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "true");
        }
    }

    #[test]
    fn directory_actions_validate_target_and_preserve_special_path_as_data() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("한글 폴더 & 이름 (test); value");
        std::fs::create_dir(&root).unwrap();
        assert_eq!(
            execute_directory_action(&root, DirectoryAction::Explorer).unwrap(),
            Some(root.clone())
        );
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "keep").unwrap();
        assert!(
            prepare_repository_directory(&file)
                .unwrap_err()
                .contains("폴더가 아닙니다")
        );
        assert!(
            execute_directory_action(&dir.path().join("missing"), DirectoryAction::GitBash)
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "keep");
    }
    #[cfg(windows)]
    #[test]
    fn powershell_keeps_path_out_of_arguments_and_uses_system_executable() {
        let root = PathBuf::from("D:/한글 폴더 & 이름 (test); value");
        let command = powershell_command(&root).unwrap();
        assert_eq!(command.get_current_dir(), Some(root.as_path()));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["-NoLogo", "-NoProfile", "-NoExit"]
        );
        let executable = PathBuf::from(command.get_program());
        assert!(executable.is_absolute());
        assert!(executable.ends_with("System32/WindowsPowerShell/v1.0/powershell.exe"));
    }
}

#[cfg(all(test, windows))]
mod git_bash_tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let base = std::env::current_dir()
            .unwrap()
            .join("target/git-bash-discovery-tests");
        std::fs::create_dir_all(&base).unwrap();
        tempfile::tempdir_in(base).unwrap()
    }
    #[test]
    fn discovers_custom_git_install_from_path_without_running_git() {
        let dir = fixture();
        let install = dir.path().join("custom Git");
        let cmd = install.join("cmd");
        std::fs::create_dir_all(&cmd).unwrap();
        std::fs::write(cmd.join("git.exe"), b"fixture").unwrap();
        let executable = install.join("git-bash.exe");
        std::fs::write(&executable, b"fixture").unwrap();
        assert_eq!(
            find_git_bash(git_bash_candidates(&[cmd], &[])).unwrap(),
            executable
        );
        assert_eq!(
            find_git_bash(git_bash_candidates(std::slice::from_ref(&install), &[])).unwrap(),
            executable
        );
    }
    #[test]
    fn falls_back_to_standard_user_install_and_reports_missing_launcher() {
        let dir = fixture();
        let system = dir.path().join("Program Files/Git");
        let user = dir.path().join("Local/Programs/Git");
        std::fs::create_dir_all(&user).unwrap();
        let executable = user.join("git-bash.exe");
        std::fs::write(&executable, b"fixture").unwrap();
        assert_eq!(
            find_git_bash(git_bash_candidates(&[], &[system, user])).unwrap(),
            executable
        );
        let directory = dir.path().join("directory/git-bash.exe");
        std::fs::create_dir_all(&directory).unwrap();
        let error = find_git_bash(vec![PathBuf::from("git-bash.exe"), directory]).unwrap_err();
        assert!(error.contains("Git Bash") && error.contains("설치"));
    }
    #[test]
    fn git_bash_keeps_special_path_in_working_directory_and_has_no_shell_arguments() {
        let root = PathBuf::from("D:/한글 폴더 '$value' & (test); value");
        let executable = PathBuf::from("C:/Program Files/Git/git-bash.exe");
        let command = git_bash_command(&root, &executable).unwrap();
        assert_eq!(command.get_program(), executable.as_os_str());
        assert_eq!(command.get_current_dir(), Some(root.as_path()));
        assert_eq!(command.get_args().count(), 0);
    }
}

#[derive(Clone)]
pub struct SyncRequest {
    pub revision: u64,
    pub project: usize,
    pub path: PathBuf,
    pub name: String,
    pub mode: repository::SyncMode,
}
pub struct SyncResult {
    pub conflict_paths: Vec<PathBuf>,
    pub request: SyncRequest,
    pub outcome: Result<(), String>,
    pub refreshed: Result<(Project, PreparedHistory), String>,
}
impl SyncRequest {
    pub fn execute(self) -> SyncResult {
        let mut conflict_paths = Vec::new();
        let outcome = repository::synchronize(&self.path, self.mode).map_err(|error| {
            if let Some(details) = error.downcast_ref::<repository::HistoryActionFailure>() {
                conflict_paths = details.conflict_paths.clone();
            }
            if let Some(details) = error.downcast_ref::<repository::RebaseFailure>() {
                conflict_paths = details.conflict_paths.clone();
            }
            if let Some(details) = error.downcast_ref::<repository::MergeFailure>() {
                conflict_paths = details.conflict_paths.clone();
            }
            format!("{error:#}")
        });
        let refreshed = refreshed_project(&self.path, &self.name);
        SyncResult {
            conflict_paths,
            request: self,
            outcome,
            refreshed,
        }
    }
}

fn refreshed_project(path: &Path, name: &str) -> Result<(Project, PreparedHistory), String> {
    repository::execute(repository::Request {
        operation: repository::Operation::Open,
        source: String::new(),
        path: path.to_string_lossy().into_owned(),
        name: name.into(),
    })
    .map(|project| {
        let history = PreparedHistory::new(&project.commits);
        (project, history)
    })
    .map_err(|error| format!("저장소 상태를 다시 읽지 못했습니다: {error:#}"))
}
#[derive(Clone)]
pub struct CommitWorkRequest {
    pub revision: u64,
    pub project: usize,
    pub path: PathBuf,
    pub name: String,
    pub commit: repository::CommitRequest,
}
pub struct CommitWorkResult {
    pub request: CommitWorkRequest,
    pub outcome: Result<repository::CommitOutcome, String>,
    pub refreshed: Result<(Project, PreparedHistory), String>,
}
impl CommitWorkRequest {
    pub fn execute(self) -> CommitWorkResult {
        let outcome = repository::commit_staged(&self.path, self.commit.clone())
            .map_err(|error| format!("{error:#}"));
        let refreshed = refreshed_project(&self.path, &self.name);
        CommitWorkResult {
            request: self,
            outcome,
            refreshed,
        }
    }
}

#[cfg(test)]
mod sync_request_tests {
    use super::*;
    #[test]
    fn failed_sync_still_returns_a_fresh_history_and_repository_snapshot() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/sync-request-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("repo");
        let repo = git2::Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let tree = repo
            .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let signature = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let id = repo
            .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        let request = SyncRequest {
            revision: 9,
            project: 2,
            path: path.clone(),
            name: "fixture".into(),
            mode: repository::SyncMode::Fetch,
        };
        let result = request.execute();
        assert!(result.outcome.is_err());
        assert_eq!(result.request.revision, 9);
        let (project, history) = result.refreshed.unwrap();
        assert_eq!(project.commits.len(), 1);
        assert_eq!(project.head, Some(id.to_string()));
        assert_eq!(
            crate::history::HistoryState::from_prepared(history).len(),
            1
        );
    }
}

#[cfg(test)]
mod commit_request_tests {
    use super::*;
    #[test]
    fn created_commit_is_refreshed_even_when_the_requested_push_fails() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/commit-work-tests");
        std::fs::create_dir_all(&root).unwrap();
        let temp = tempfile::tempdir_in(root).unwrap();
        let path = temp.path().join("repo");
        let repo = git2::Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        repo.config()
            .unwrap()
            .set_str("user.name", "Tester")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.invalid")
            .unwrap();
        std::fs::write(path.join("file.txt"), "content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let draft = repository::prepare_commit(&path).unwrap();
        let request = CommitWorkRequest {
            revision: 1,
            project: 0,
            path,
            name: "fixture".into(),
            commit: repository::CommitRequest {
                expected_head: draft.head,
                expected_reference: draft.reference,
                message: "initial".into(),
                amend: false,
                push: true,
            },
        };
        let result = request.execute();
        let outcome = result.outcome.unwrap();
        assert!(outcome.error.is_some());
        let (project, history) = result.refreshed.unwrap();
        assert_eq!(project.head, Some(outcome.id));
        assert_eq!(project.commits.len(), 1);
        assert_eq!(
            crate::history::HistoryState::from_prepared(history).len(),
            1
        );
    }
}

#[derive(Clone)]
pub enum ToolbarAction {
    ContinueRevert { expected_head: String },
    AbortRevert { expected_head: String },
    Refresh { selected_files: Vec<(Vec<u8>, bool)> },
    RevertCommit { request: repository::CommitHistoryRequest, no_commit: bool },
    ResetCommit { request: repository::CommitHistoryRequest, mode: repository::ResetMode },
    CheckoutCommitBranch { reference: String, id: String },
    RebaseCommit { request: repository::RebaseRequest },
    ContinueRebase { expected_head: String },
    AbortRebase { expected_head: String },
    AbortMerge { expected_head: String },
    MergeCommit { id: String },
    Push,
    ApplyStash {
        request: repository::ReferenceDeletion,
    },
    CreateStash {
        name: String,
        keep_index: bool,
    },
    CreateTag {
        id: String,
        name: String,
        push: bool,
    },
    DeleteReference {
        request: repository::ReferenceDeletion,
        remote_tag: bool,
    },
    Checkout {
        reference: String,
    },
    CreateBranch {
        name: String,
        checkout: bool,
    },
    CreateTrackingBranch {
        name: String,
        remote: String,
    },
}
#[derive(Clone)]
pub struct ToolbarRequest {
    pub revision: u64,
    pub project: usize,
    pub path: PathBuf,
    pub name: String,
    pub action: ToolbarAction,
}
pub struct ToolbarResult {
    pub conflict_paths: Vec<PathBuf>,
    pub request: ToolbarRequest,
    pub outcome: Result<Option<String>, String>,
    pub refreshed: Result<(Project, PreparedHistory), String>,
}
impl ToolbarRequest {
    pub fn execute(self) -> ToolbarResult {
        let mut conflict_paths = Vec::new();
        let outcome = match &self.action {
            ToolbarAction::ContinueRevert { expected_head } => repository::continue_revert(&self.path, expected_head).map(|()| None),
            ToolbarAction::AbortRevert { expected_head } => repository::abort_revert(&self.path, expected_head).map(|()| None),
            ToolbarAction::Refresh { .. } => Ok(None),
            ToolbarAction::RevertCommit { request, no_commit } => repository::revert_commit(&self.path, request, *no_commit).map(|()| None),
            ToolbarAction::ResetCommit { request, mode } => repository::reset_commit(&self.path, request, *mode).map(|()| None),
            ToolbarAction::DeleteReference {
                request,
                remote_tag,
            } => if *remote_tag {
                repository::delete_reference_with_remote_tag(&self.path, request.clone(), true)
            } else {
                repository::delete_reference(&self.path, request.clone())
            }
            .map(|()| None),
            ToolbarAction::CreateTag { id, name, push } => {
                repository::create_tag(&self.path, id, name, *push)
            }
            ToolbarAction::CreateStash { name, keep_index } => {
                repository::create_stash(&self.path, name, *keep_index).map(|()| None)
            }
            ToolbarAction::ApplyStash { request } => {
                repository::apply_stash(&self.path, request.clone()).map(|()| None)
            }
            ToolbarAction::RebaseCommit { request } => repository::start_rebase(&self.path, request).map(|()| None),
            ToolbarAction::ContinueRebase { expected_head } => repository::continue_rebase(&self.path, expected_head).map(|()| None),
            ToolbarAction::AbortRebase { expected_head } => repository::abort_rebase(&self.path, expected_head).map(|()| None),
            ToolbarAction::AbortMerge { expected_head } => repository::abort_merge(&self.path, expected_head).map(|()| None),
            ToolbarAction::MergeCommit { id } => repository::merge_commit(&self.path, id).map(|()| None),
            ToolbarAction::Push => repository::push_branch(&self.path).map(|()| None),
            ToolbarAction::CheckoutCommitBranch { reference, id } => repository::checkout_commit_branch(&self.path, reference, id).map(|()| None),
            ToolbarAction::Checkout { reference } => {
                repository::checkout_reference(&self.path, reference).map(|()| None)
            }
            ToolbarAction::CreateTrackingBranch { name, remote } => {
                repository::create_tracking_branch(&self.path, name, remote)
                    .map(|outcome| outcome.error)
            }
            ToolbarAction::CreateBranch { name, checkout } => {
                repository::create_branch(&self.path, name, *checkout).map(|outcome| outcome.error)
            }
        }
        .map_err(|error| {
            if let Some(details) = error.downcast_ref::<repository::RebaseFailure>() {
                conflict_paths = details.conflict_paths.clone();
            }
            if let Some(details) = error.downcast_ref::<repository::MergeFailure>() {
                conflict_paths = details.conflict_paths.clone();
            }
            if let Some(details) = error.downcast_ref::<repository::StashApplyFailure>() {
                conflict_paths = details.conflict_paths.clone();
            }
            format!("{error:#}")
        });
        let refreshed = refreshed_project(&self.path, &self.name);
        ToolbarResult {
            conflict_paths,
            request: self,
            outcome,
            refreshed,
        }
    }
}

#[cfg(test)]
mod toolbar_request_tests {
    use super::*;
    #[test]
    fn branch_creation_and_checkout_return_refreshed_repository_and_push_failure_still_refreshes() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/toolbar-request-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("repo");
        let repo = git2::Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let tree = repo
            .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let id = repo
            .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        let request = ToolbarRequest {
            revision: 1,
            project: 0,
            path: path.clone(),
            name: "fixture".into(),
            action: ToolbarAction::CreateBranch {
                name: "feature".into(),
                checkout: true,
            },
        };
        let result = request.execute();
        assert!(result.outcome.unwrap().is_none());
        let (project, history) = result.refreshed.unwrap();
        assert_eq!(project.head.as_deref(), Some(id.to_string().as_str()));
        assert!(project.branches.iter().any(|name| name == "feature"));
        assert_eq!(
            crate::history::HistoryState::from_prepared(history).len(),
            1
        );
        let request = ToolbarRequest {
            revision: 2,
            project: 0,
            path,
            name: "fixture".into(),
            action: ToolbarAction::Push,
        };
        let result = request.execute();
        assert!(result.outcome.is_err());
        assert!(result.refreshed.is_ok());
    }
    #[test]
    fn history_merge_request_refreshes_on_success_and_failure() {
        let root = std::env::current_dir().unwrap().join("target/merge-request-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("repo");
        let repo = git2::Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let tree = repo.find_tree(repo.treebuilder(None).unwrap().write().unwrap()).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let first = repo.commit(Some("HEAD"), &sig, &sig, "first", &tree, &[]).unwrap();
        let second = repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&repo.find_commit(first).unwrap()]).unwrap();
        repo.reset(repo.find_commit(first).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
        for (id, success) in [(second.to_string(), true), ("invalid".into(), false)] {
            let result = ToolbarRequest { revision: 1, project: 0, path: path.clone(), name: "fixture".into(), action: ToolbarAction::MergeCommit { id } }.execute();
            assert_eq!(result.outcome.is_ok(), success);
            let (project, _) = result.refreshed.unwrap();
            assert_eq!(project.head.as_deref(), Some(second.to_string().as_str()));
            assert_eq!(project.branch, "main");
            assert!(!repo.head_detached().unwrap());
        }
    }
    #[test]
    fn checkout_request_returns_current_head_and_history() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/checkout-request-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("repo");
        let repo = git2::Repository::init(&path).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let tree = repo
            .find_tree(repo.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let first = repo
            .commit(Some("HEAD"), &sig, &sig, "first", &tree, &[])
            .unwrap();
        let parent = repo.find_commit(first).unwrap();
        repo.tag_lightweight("v1", parent.as_object(), false)
            .unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
            .unwrap();
        let result = ToolbarRequest {
            revision: 1,
            project: 0,
            path,
            name: "fixture".into(),
            action: ToolbarAction::Checkout {
                reference: "refs/tags/v1".into(),
            },
        }
        .execute();
        assert!(result.outcome.unwrap().is_none());
        let (project, _) = result.refreshed.unwrap();
        assert_eq!(project.head.as_deref(), Some(first.to_string().as_str()));
        assert!(repo.head_detached().unwrap());
    }
}
