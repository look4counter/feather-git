#[path = "backend_worker.rs"]
mod worker;
pub(crate) use worker::ReadSlot;

use std::{cell::Cell, sync::OnceLock, thread::ThreadId, time::Duration};

pub const HELP: &str = "Feather Git\n\n사용법: cargo run -- [--backend-delay-ms <0~60000>]\n\n  --backend-delay-ms <밀리초>  각 백엔드 I/O 요청 전에 추가 지연 (기본 0)\n  --help, -h                 도움말 출력\n\n예: cargo run -- --backend-delay-ms 1500\n중첩 호출은 한 번만 지연합니다. 동기 UI 호출도 지연하므로 화면 멈춤이 드러납니다.\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub delay_ms: u64,
    pub help: bool,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut options = Self::default();
        let mut seen = false;
        while let Some(arg) = args.next() {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }
            let value = if arg == "--backend-delay-ms" {
                args.next().ok_or("--backend-delay-ms 값이 필요합니다.")?
            } else if let Some(value) = arg.strip_prefix("--backend-delay-ms=") {
                value.to_owned()
            } else {
                return Err(format!("알 수 없는 옵션입니다: {arg}"));
            };
            if seen {
                return Err("--backend-delay-ms는 한 번만 지정하세요.".into());
            }
            seen = true;
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("지연 시간은 0~60000 사이 정수여야 합니다.".into());
            }
            options.delay_ms = value
                .parse::<u64>()
                .ok()
                .filter(|value| *value <= 60_000)
                .ok_or("지연 시간은 0~60000 사이 정수여야 합니다.")?;
        }
        Ok(options)
    }
}

#[derive(Clone, Copy)]
struct Policy {
    delay: Duration,
    ui_thread: ThreadId,
}
static POLICY: OnceLock<Policy> = OnceLock::new();
thread_local! { static ENTERED: Cell<bool> = const { Cell::new(false) }; }

pub fn initialize(options: Options) {
    let _ = POLICY.set(Policy {
        delay: Duration::from_millis(options.delay_ms),
        ui_thread: std::thread::current().id(),
    });
}

struct RequestGuard;
impl Drop for RequestGuard {
    fn drop(&mut self) {
        ENTERED.with(|entered| entered.set(false));
    }
}

fn policy() -> Option<Policy> {
    #[cfg(test)]
    if let Some(policy) = TEST_POLICY.with(Cell::get) {
        return Some(policy);
    }
    POLICY.get().copied()
}

pub(crate) fn run<T>(operation: &'static str, work: impl FnOnce() -> T) -> T {
    worker::checkpoint();
    let Some(policy) = policy().filter(|policy| !policy.delay.is_zero()) else {
        return work();
    };
    if ENTERED.with(|entered| entered.replace(true)) {
        return work();
    }
    let _guard = RequestGuard;
    let ui = std::thread::current().id() == policy.ui_thread;
    eprintln!(
        "[backend-delay] {operation}: {}ms · {}",
        policy.delay.as_millis(),
        if ui {
            "UI 스레드 동기 호출"
        } else {
            "백그라운드 호출"
        }
    );
    #[cfg(test)]
    TEST_CALLS.with(|calls| calls.borrow_mut().push((operation, ui)));
    worker::delay(policy.delay);
    work()
}

pub(crate) fn checkpoint() {
    worker::checkpoint();
}

pub fn spawn<T: Send + 'static>(
    executor: &gpui::BackgroundExecutor,
    work: impl FnOnce() -> T + Send + 'static,
) -> gpui::Task<T> {
    worker::spawn(executor, work)
}

// 기존 공개 서명은 유지하고 I/O 경계만 한 곳에서 등록합니다.
macro_rules! module {
    ($name:ident, $file:literal, [$($export:ident),* $(,)?], {
        $( $(#[$attribute:meta])* $vis:vis fn $function:ident($($argument:ident: $ty:ty),* $(,)?) $(-> $result:ty)?; )*
    }) => {
        mod $name {
            mod implementation { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/", $file)); }
            // 공통 API의 명시적 재노출은 현재 모듈의 사용 여부와 무관하게 유지합니다.
            #[allow(unused_imports)]
            pub use implementation::{$($export),*};
            $( $(#[$attribute])* $vis fn $function($($argument: $ty),*) $(-> $result)? {
                $crate::backend::run(concat!(stringify!($name), "::", stringify!($function)),
                    || implementation::$function($($argument),*))
            } )*
        }
    };
}
pub(crate) use module;

macro_rules! install {
    () => {
        $crate::backend::module!(
            repository,
            "repository.rs",
            [
                Operation,
                Request,
                ReferenceKind,
                CommitReference,
                Commit,
                ChangedFile,
                Project,
                name_from_path,
                suggested_name,
                selected_repository_path,
                is_git_directory,
                remote_branch_label,
                stash_display_label,
                HistoryCursor,
                HistoryPage,
                INITIAL_HISTORY_ROWS,
                HISTORY_PAGE_ROWS,
                SyncMode,
                CommitDraft,
                CommitRequest,
                CommitOutcome,
                BranchOutcome,
                ReferenceDeletion,
                RepositoryWorkState,
                CommitHistoryRequest,
                ResetMode,
                HistoryActionFailure,
                is_revert_state,
                RebaseRequest,
                RebaseFailure,
                is_rebase_state,
                MergeFailure,
                MergeFailureKind,
                StashApplyFailure
            ],
            {
                pub fn git_metadata_directories(workdir: &std::path::Path) -> anyhow::Result<Vec<std::path::PathBuf>>;
                pub fn repository_identity(path: &std::path::Path) -> std::path::PathBuf;
                pub fn worktree_directory(path: &std::path::Path) -> anyhow::Result<std::path::PathBuf>;
                pub fn execute(request: Request) -> anyhow::Result<Project>;
                pub fn synchronize(path: &std::path::Path, mode: SyncMode) -> anyhow::Result<()>;
                pub fn work_state(path: &std::path::Path) -> anyhow::Result<RepositoryWorkState>;
                pub fn continue_revert(path: &std::path::Path, expected_head: &str) -> anyhow::Result<()>;
                pub fn abort_revert(path: &std::path::Path, expected_head: &str) -> anyhow::Result<()>;
                pub fn revert_commit(path: &std::path::Path, request: &CommitHistoryRequest, no_commit: bool) -> anyhow::Result<()>;
                pub fn reset_commit(path: &std::path::Path, request: &CommitHistoryRequest, mode: ResetMode) -> anyhow::Result<()>;
                pub fn start_rebase(path: &std::path::Path, request: &RebaseRequest) -> anyhow::Result<()>;
                pub fn continue_rebase(path: &std::path::Path, expected_head: &str) -> anyhow::Result<()>;
                pub fn abort_rebase(path: &std::path::Path, expected_head: &str) -> anyhow::Result<()>;
                pub fn abort_merge(path: &std::path::Path, expected_head: &str) -> anyhow::Result<()>;
                pub fn merge_commit(path: &std::path::Path, id: &str) -> anyhow::Result<()>;
                pub fn checkout_commit_branch(path: &std::path::Path, reference: &str, expected: &str) -> anyhow::Result<()>;
                pub fn checkout_reference(
                    path: &std::path::Path,
                    reference: &str,
                ) -> anyhow::Result<()>;
                pub fn create_tracking_branch(
                    path: &std::path::Path,
                    name: &str,
                    remote: &str,
                ) -> anyhow::Result<BranchOutcome>;
                pub fn prepare_reference_deletion(
                    path: &std::path::Path,
                    reference: &str,
                ) -> anyhow::Result<ReferenceDeletion>;
                pub fn delete_reference_with_remote_tag(
                    path: &std::path::Path,
                    request: ReferenceDeletion,
                    delete_remote_tag: bool,
                ) -> anyhow::Result<()>;
                pub fn delete_reference(
                    path: &std::path::Path,
                    request: ReferenceDeletion,
                ) -> anyhow::Result<()>;
                pub fn apply_stash(
                    path: &std::path::Path,
                    request: ReferenceDeletion,
                ) -> anyhow::Result<()>;
                pub fn create_stash(
                    path: &std::path::Path,
                    name: &str,
                    keep_index: bool,
                ) -> anyhow::Result<()>;
                pub fn create_tag(
                    path: &std::path::Path,
                    id: &str,
                    name: &str,
                    push: bool,
                ) -> anyhow::Result<Option<String>>;
                pub fn push_branch(path: &std::path::Path) -> anyhow::Result<()>;
                pub fn push_branch_with_lease(path: &std::path::Path) -> anyhow::Result<()>;
                pub fn create_branch(
                    path: &std::path::Path,
                    name: &str,
                    checkout: bool,
                ) -> anyhow::Result<BranchOutcome>;
                pub fn prepare_commit(path: &std::path::Path) -> anyhow::Result<CommitDraft>;
                pub fn commit_staged(
                    path: &std::path::Path,
                    request: CommitRequest,
                ) -> anyhow::Result<CommitOutcome>;
                pub fn avatar_repository(path: &std::path::Path) -> anyhow::Result<Option<String>>;
                pub fn delete_repository_files(path: &std::path::Path) -> anyhow::Result<()>;
                pub fn panel_references(
                    path: &std::path::Path,
                    stashes: bool,
                ) -> anyhow::Result<Vec<String>>;
                #[cfg(test)]
                pub fn commit_files(
                    path: &std::path::Path,
                    id: &str,
                ) -> anyhow::Result<Vec<ChangedFile>>;
                pub fn history_page(
                    cursor: &HistoryCursor,
                    offset: usize,
                ) -> anyhow::Result<HistoryPage>;
                pub fn commit_snapshot(path: &std::path::Path, name: &str, id: &str) -> anyhow::Result<Project>;
                pub fn reference_snapshot(
                    path: &std::path::Path,
                    name: &str,
                    reference: &str,
                ) -> anyhow::Result<Project>;
            }
        );
        $crate::backend::module!(file_history, "file_history.rs", [FileHistoryEntry], {
            pub fn export_revision(
                root: &std::path::Path,
                id: &str,
                raw: &[u8],
                session: &crate::history_temp::Session,
            ) -> anyhow::Result<std::path::PathBuf>;
            pub fn load(
                root: &std::path::Path,
                raw: &[u8],
            ) -> anyhow::Result<Vec<FileHistoryEntry>>;
        });
        $crate::backend::module!(
            diff_git,
            "diff_git.rs",
            [MAX_BYTES, DiscardPreview, DiscardResult],
            {
                pub(crate) fn open(path: &std::path::Path) -> anyhow::Result<gix::Repository>;
                pub(crate) fn read_content(
                    repo: &gix::Repository,
                    file: &crate::diff_model::FileChange,
                    content: &crate::diff_model::Content,
                ) -> anyhow::Result<Vec<u8>>;
                pub fn load_stage(
                    path: &std::path::Path,
                ) -> anyhow::Result<Vec<crate::diff_model::FileChange>>;
                pub fn load_history(
                    path: &std::path::Path,
                    id: &str,
                ) -> anyhow::Result<Vec<crate::diff_model::FileChange>>;
                pub fn load_detail(
                    path: &std::path::Path,
                    id: &str,
                ) -> anyhow::Result<crate::diff_model::CommitDetail>;
                #[cfg(test)]
                pub fn diff(
                    path: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                ) -> anyhow::Result<crate::diff_model::Diff>;
                pub fn diff_with_encoding(
                    path: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                    encoding: crate::text_encoding::TextEncoding,
                ) -> anyhow::Result<crate::diff_model::Diff>;
                pub fn set_index_content(
                    path: &std::path::Path,
                    raw: &[u8],
                    content: Option<&[u8]>,
                    mode: u32,
                    expected: Option<(gix::ObjectId, u32)>,
                ) -> anyhow::Result<()>;
                pub fn prepare_discard(
                    root: &std::path::Path,
                    files: &[crate::diff_model::FileChange],
                ) -> anyhow::Result<DiscardPreview>;
                pub fn discard_selected(
                    root: &std::path::Path,
                    preview: DiscardPreview,
                ) -> DiscardResult;
                pub fn set_worktree_content(
                    root: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                    source: &[u8],
                    replacement: &[u8],
                ) -> anyhow::Result<()>;
                pub fn toggle_stage(
                    root: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                ) -> anyhow::Result<()>;
                #[cfg(test)]
                pub fn stage_resolved(
                    root: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                    preview: &crate::diff_model::Diff,
                ) -> anyhow::Result<()>;
            }
        );
        $crate::backend::module!(
            partial_stage,
            "partial_stage.rs",
            [Preview, eligible, hunk_rows],
            {
                pub fn discard(
                    root: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                    preview: &Preview,
                    lines: &[crate::diff_model::DiffLine],
                    selected: &[usize],
                ) -> anyhow::Result<()>;
                pub fn apply(
                    root: &std::path::Path,
                    file: &crate::diff_model::FileChange,
                    preview: &Preview,
                    lines: &[crate::diff_model::DiffLine],
                    selected: &[usize],
                ) -> anyhow::Result<()>;
            }
        );
        $crate::backend::module!(conflict, "conflict.rs", [Choice, Region, Preview, parse], {
            pub fn apply(
                root: &std::path::Path,
                raw: &[u8],
                preview: &Preview,
                index: usize,
                choice: Choice,
            ) -> anyhow::Result<()>;
        });
        $crate::backend::module!(
            session,
            "session.rs",
            [
                RECENT_LIMIT,
                RepositoryEntry,
                Session,
                settings_path,
                remember_normalized
            ],
            {
                #[cfg(test)]
                pub fn remember(recent: &mut Vec<RepositoryEntry>, entry: RepositoryEntry);
                pub fn read(path: &std::path::Path) -> anyhow::Result<Session>;
                pub fn write(path: &std::path::Path, session: &Session) -> anyhow::Result<()>;
                pub fn read_bytes(path: &std::path::Path) -> std::io::Result<Vec<u8>>;
                pub fn write_bytes(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()>;
            }
        );
    };
}
pub(crate) use install;

#[cfg(test)]
thread_local! {
    static TEST_POLICY: Cell<Option<Policy>> = const { Cell::new(None) };
    static TEST_CALLS: std::cell::RefCell<Vec<(&'static str, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn with_test_delay<T>(
    delay_ms: u64,
    ui_thread: ThreadId,
    work: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<Policy>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_POLICY.with(|policy| policy.set(self.0));
        }
    }
    let old = TEST_POLICY.with(|policy| {
        policy.replace(Some(Policy {
            delay: Duration::from_millis(delay_ms),
            ui_thread,
        }))
    });
    let _restore = Restore(old);
    TEST_CALLS.with(|calls| calls.borrow_mut().clear());
    work()
}
#[cfg(test)]
pub(crate) fn test_calls() -> Vec<(&'static str, bool)> {
    TEST_CALLS.with(|calls| calls.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::mpsc,
        time::Instant,
    };

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }
    fn configured<T>(delay_ms: u64, ui_thread: ThreadId, work: impl FnOnce() -> T) -> T {
        struct Restore(Option<Policy>);
        impl Drop for Restore {
            fn drop(&mut self) {
                TEST_POLICY.with(|policy| policy.set(self.0));
            }
        }
        let old = TEST_POLICY.with(|policy| {
            policy.replace(Some(Policy {
                delay: Duration::from_millis(delay_ms),
                ui_thread,
            }))
        });
        let _restore = Restore(old);
        TEST_CALLS.with(|calls| calls.borrow_mut().clear());
        work()
    }
    fn calls() -> Vec<(&'static str, bool)> {
        TEST_CALLS.with(|calls| calls.borrow().clone())
    }

    #[test]
    fn parses_default_zero_max_equals_and_help() {
        assert_eq!(Options::parse(args(&[])).unwrap(), Options::default());
        assert_eq!(
            Options::parse(args(&["--backend-delay-ms", "0"]))
                .unwrap()
                .delay_ms,
            0
        );
        assert_eq!(
            Options::parse(args(&["--backend-delay-ms=60000"]))
                .unwrap()
                .delay_ms,
            60000
        );
        assert_eq!(
            Options::parse(args(&["--backend-delay-ms", "1500"]))
                .unwrap()
                .delay_ms,
            1500
        );
        for flag in ["--help", "-h"] {
            assert!(Options::parse(args(&[flag])).unwrap().help);
        }
    }
    #[test]
    fn rejects_missing_invalid_out_of_range_duplicate_and_unknown_options() {
        for values in [
            vec!["--backend-delay-ms"],
            vec!["--backend-delay-ms="],
            vec!["--backend-delay-ms=-1"],
            vec!["--backend-delay-ms=1.5"],
            vec!["--backend-delay-ms=60001"],
            vec!["--backend-delay-ms=18446744073709551616"],
            vec!["--backend-delay-ms=+1"],
            vec!["--backend-delay-ms=1", "--backend-delay-ms", "2"],
            vec!["--unknown"],
        ] {
            assert!(Options::parse(args(&values)).is_err(), "{values:?}");
        }
    }
    #[test]
    fn default_and_zero_do_not_delay_or_instrument_requests() {
        assert!(policy().is_none());
        assert_eq!(run("disabled", || 17), 17);
        configured(0, std::thread::current().id(), || {
            assert_eq!(run("zero", || 23), 23);
            assert!(calls().is_empty());
        });
    }
    #[test]
    fn delay_precedes_work_and_preserves_result() {
        configured(8, std::thread::current().id(), || {
            let start = Instant::now();
            let value = run("timed", || {
                assert!(start.elapsed() >= Duration::from_millis(8));
                Ok::<_, &'static str>(31)
            });
            assert_eq!(value, Ok(31));
            assert_eq!(calls(), vec![("timed", true)]);
        });
    }
    #[test]
    fn nested_requests_delay_once_and_errors_and_panics_restore_guard() {
        configured(1, std::thread::current().id(), || {
            assert_eq!(
                run("outer", || run("nested", || Err::<(), _>("failure"))),
                Err("failure")
            );
            assert!(
                catch_unwind(AssertUnwindSafe(|| run("panic", || panic!("expected")))).is_err()
            );
            assert_eq!(run("after-panic", || 7), 7);
            assert_eq!(
                calls(),
                vec![("outer", true), ("panic", true), ("after-panic", true)]
            );
        });
    }
    #[test]
    fn concurrent_workers_are_independent_and_report_background_thread() {
        let ui = std::thread::current().id();
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut workers = Vec::new();
        let mut releases = Vec::new();
        for _ in 0..2 {
            let ready = ready_tx.clone();
            let (release, wait) = mpsc::channel();
            releases.push(release);
            workers.push(std::thread::spawn(move || {
                configured(2, ui, || {
                    run("worker", || {
                        ready.send(()).unwrap();
                        let _ = wait.recv();
                    });
                    calls()
                })
            }));
        }
        let first = ready_rx.recv_timeout(Duration::from_secs(3));
        let second = ready_rx.recv_timeout(Duration::from_secs(3));
        for release in releases {
            let _ = release.send(());
        }
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert!(first.is_ok() && second.is_ok());
        assert!(
            results
                .iter()
                .all(|calls| calls == &vec![("worker", false)])
        );
    }
    #[test]
    fn actual_gateways_cover_backend_io_without_nested_delays() {
        use crate::{
            diff_model::{Content, Diff, FileChange},
            repository::{Operation, Request},
        };
        let root = std::env::current_dir()
            .unwrap()
            .join("target/backend-throttle-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let missing = dir.path().join("missing");
        let path = dir.path().join("settings.json");
        let repository_path = dir.path().join("repository");
        git2::Repository::init(&repository_path).unwrap();
        let repo = crate::diff_git::open(&repository_path).unwrap();
        let file = FileChange {
            path: b"file.txt".to_vec(),
            label: "file.txt".into(),
            status: "M".into(),
            staged: false,
            before: Content::Empty,
            after: Content::Worktree,
        };
        let check = |name, work: &mut dyn FnMut()| {
            configured(1, std::thread::current().id(), || {
                work();
                assert_eq!(calls(), vec![(name, true)]);
            })
        };
        check("repository::execute", &mut || {
            let _ = crate::repository::execute(Request {
                operation: Operation::Open,
                path: missing.display().to_string(),
                source: String::new(),
                name: "test".into(),
            });
        });
        check("repository::panel_references", &mut || {
            let _ = crate::repository::panel_references(&missing, false);
        });
        check("repository::commit_files", &mut || {
            let _ = crate::repository::commit_files(&missing, "bad");
        });
        check("repository::reference_snapshot", &mut || {
            let _ = crate::repository::reference_snapshot(&missing, "test", "refs/heads/main");
        });
        check("diff_git::open", &mut || {
            let _ = crate::diff_git::open(&missing);
        });
        check("diff_git::load_stage", &mut || {
            let _ = crate::diff_git::load_stage(&missing);
        });
        check("diff_git::load_history", &mut || {
            let _ = crate::diff_git::load_history(&missing, "bad");
        });
        check("diff_git::load_detail", &mut || {
            let _ = crate::diff_git::load_detail(&missing, "bad");
        });
        check("diff_git::diff", &mut || {
            let _ = crate::diff_git::diff(&missing, &file);
        });
        check("diff_git::read_content", &mut || {
            assert!(
                crate::diff_git::read_content(&repo, &file, &Content::Empty)
                    .unwrap()
                    .is_empty()
            );
        });
        check("diff_git::set_index_content", &mut || {
            let _ = crate::diff_git::set_index_content(&missing, b"file.txt", None, 0o100644, None);
        });
        check("diff_git::stage_resolved", &mut || {
            let _ = crate::diff_git::stage_resolved(&missing, &file, &Diff::default());
        });
        check("partial_stage::apply", &mut || {
            let _ = crate::partial_stage::apply(
                &missing,
                &file,
                &crate::partial_stage::Preview {
                    before: String::new(),
                    after: String::new(),
                },
                &[],
                &[],
            );
        });
        check("conflict::apply", &mut || {
            let preview = crate::conflict::parse("plain").unwrap();
            let _ = crate::conflict::apply(
                &missing,
                b"file.txt",
                &preview,
                0,
                crate::conflict::Choice::Both,
            );
        });
        check("session::read", &mut || {
            assert!(crate::session::read(&path).is_ok());
        });
        check("session::write", &mut || {
            crate::session::write(&path, &crate::session::Session::default()).unwrap();
        });
        check("session::read_bytes", &mut || {
            assert!(crate::session::read_bytes(&path).is_ok());
        });
        check("session::write_bytes", &mut || {
            crate::session::write_bytes(&path, b"{}").unwrap();
        });
        check("session::remember", &mut || {
            let mut recent = Vec::new();
            crate::session::remember(
                &mut recent,
                crate::session::RepositoryEntry {
                    name: "test".into(),
                    path: repository_path.clone(),
                },
            );
        });
        configured(1, std::thread::current().id(), || {
            run("outer-backend", || {
                let _ = crate::session::read(&path);
                let _ = crate::diff_git::open(&missing);
            });
            assert_eq!(calls(), vec![("outer-backend", true)]);
        });
    }
    #[test]
    fn pure_helpers_are_not_throttled() {
        configured(1, std::thread::current().id(), || {
            assert_eq!(crate::repository::name_from_path("path/name"), "name");
            assert!(crate::partial_stage::hunk_rows(&[], 0).is_empty());
            assert!(crate::conflict::parse("plain text").is_ok());
            let _ = crate::session::settings_path();
            assert!(calls().is_empty());
        });
    }
}
