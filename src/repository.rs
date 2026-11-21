mod commit_history_actions { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commit_history_actions.rs")); }
pub use commit_history_actions::{CommitHistoryRequest, ResetMode, HistoryActionFailure, revert_commit, reset_commit, is_revert_state, continue_revert, abort_revert};
mod rebase_actions { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/rebase_actions.rs")); }
pub use rebase_actions::{RebaseRequest, RebaseFailure, is_rebase_state, start_rebase, continue_rebase, abort_rebase};
mod stash_actions { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/stash_actions.rs")); }
pub use stash_actions::{create_stash, apply_stash, StashApplyFailure};
mod tag_actions { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tag_actions.rs")); }
pub use tag_actions::create_tag;
mod reference_actions { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/reference_actions.rs")); }
pub use reference_actions::{ReferenceDeletion, prepare_reference_deletion, delete_reference, delete_reference_with_remote_tag};
mod branch_actions { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/branch_actions.rs")); }
pub use branch_actions::{BranchOutcome, checkout_reference, checkout_commit_branch, create_branch, create_tracking_branch, push_branch, push_branch_with_lease};
mod committing {include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/committing.rs"));}
pub use committing::{CommitDraft,CommitRequest,CommitOutcome,prepare_commit,commit_staged};
mod synchronization { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/synchronization.rs")); }
pub use synchronization::{SyncMode, synchronize, merge_commit, abort_merge, MergeFailure, MergeFailureKind};
use anyhow::{Context, Result, bail};
use git2::{Cred, CredentialType, Direction, Repository, RepositoryOpenFlags};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Create,
    Open,
    Clone,
}
impl Operation {
    pub fn title(self) -> &'static str {
        match self {
            Self::Create => "Create Repository",
            Self::Open => "Open Repository",
            Self::Clone => "Clone Repository",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Request {
    pub operation: Operation,
    pub source: String,
    pub path: String,
    pub name: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceKind {
    Local,
    Remote,
    Tag,
    Other,
}

#[derive(Clone, Debug)]
pub struct CommitReference {
    pub full_name: String,
    pub name: String,
    pub label: String,
    pub is_remote: bool,
}

impl CommitReference {
    pub fn kind(&self) -> ReferenceKind {
        if self.is_remote {
            ReferenceKind::Remote
        } else if self.full_name.starts_with("refs/heads/") {
            ReferenceKind::Local
        } else if self.full_name.starts_with("refs/tags/") {
            ReferenceKind::Tag
        } else {
            ReferenceKind::Other
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Commit {
    pub id: String,
    pub parents: Vec<String>,
    pub message: String,
    pub author: String,
    pub date: String,
    pub reference: String,
    pub references: Vec<CommitReference>,
}
#[derive(Clone, Debug)]
pub struct ChangedFile {
    pub status: String,
    pub path: String,
}
#[derive(Clone, Debug, Default)]
pub struct RepositoryWorkState {
    pub operation: Option<git2::RepositoryState>,
    pub has_conflicts: bool,
    pub error: Option<String>,
}

pub fn work_state(path: &Path) -> Result<RepositoryWorkState> {
    read_work_state(&open_exact(path)?)
}

fn read_work_state(repo: &Repository) -> Result<RepositoryWorkState> {
    let operation = repo.state();
    Ok(RepositoryWorkState {
        operation: (operation != git2::RepositoryState::Clean).then_some(operation),
        has_conflicts: !repo.is_bare() && repo.index()?.has_conflicts(),
        error: None,
    })
}
#[derive(Clone, Debug, Default)]
pub struct Project {
    pub work_state: RepositoryWorkState,
    pub name: String,
    pub path: Option<PathBuf>,
    pub workdir: Option<PathBuf>,
    pub branch: String,
    pub head: Option<String>,
    pub branches: Vec<String>,
    pub branch_upstreams: std::collections::BTreeMap<String, String>,
    pub remotes: Vec<String>,
    pub remote_names: Vec<String>,
    pub commits: Vec<Commit>,
    pub files: Vec<ChangedFile>,
    pub more_commits: bool,
    pub history_cursor: Option<HistoryCursor>,
}
pub const INITIAL_HISTORY_ROWS: usize = 1500;
pub const HISTORY_PAGE_ROWS: usize = 200;

#[derive(Clone, Debug)]
pub struct HistoryCursor(std::sync::Arc<std::sync::mpsc::Sender<HistoryPageCommand>>);
#[derive(Clone, Debug)]
pub struct HistoryPage {
    pub commits: Vec<Commit>,
    pub more: bool,
}
#[derive(Debug)]
struct HistoryPageCommand {
    offset: usize,
    reply: std::sync::mpsc::Sender<Result<HistoryPage, String>>,
}
impl HistoryCursor {
    pub fn same_session(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
pub fn history_page(cursor: &HistoryCursor, offset: usize) -> Result<HistoryPage> {
    let (reply, result) = std::sync::mpsc::channel();
    cursor
        .0
        .send(HistoryPageCommand { offset, reply })
        .map_err(|_| anyhow::anyhow!("커밋 조회 세션이 종료되었습니다. 저장소를 다시 여세요."))?;
    result
        .recv()
        .context("커밋 추가 조회가 중단되었습니다.")?
        .map_err(anyhow::Error::msg)
}
fn commit_row(
    commit: &git2::Commit<'_>,
    references: &mut std::collections::HashMap<String, Vec<CommitReference>>,
) -> Commit {
    let id = commit.id().to_string();
    let time = commit.time();
    let date = gix::date::Time::new(time.seconds(), time.offset_minutes() * 60)
        .format(gix::date::time::format::ISO8601)
        .map(|s| s.chars().take(16).collect::<String>().replace('T', " "))
        .unwrap_or_else(|_| time.seconds().to_string());
    let commit_references = references.remove(&id).unwrap_or_default();
    Commit {
        parents: commit.parent_ids().map(|id| id.to_string()).collect(),
        message: commit.message().unwrap_or("메시지 없음").into(),
        author: commit.author().name().unwrap_or("알 수 없음").into(),
        date,
        reference: commit_references
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        references: commit_references,
        id,
    }
}
fn start_history(
    path: &Path,
    tips: Vec<git2::Oid>,
    target: Option<git2::Oid>,
    mut references: std::collections::HashMap<String, Vec<CommitReference>>,
) -> Result<(HistoryPage, HistoryCursor)> {
    let path = path.to_path_buf();
    let (commands, requests) = std::sync::mpsc::channel::<HistoryPageCommand>();
    let (initial, first) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("commit-history".into())
        .spawn(move || {
            let run = || -> Result<()> {
                let repo = open_exact(&path)?;
                let mut walk = repo.revwalk()?;
                walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
                for tip in tips {
                    walk.push(tip)?;
                }
                let mut walk = walk.peekable();
                let mut target_loaded = target.is_none();
                let mut read = |limit: usize| -> Result<HistoryPage> {
                    let mut commits = Vec::new();
                    while commits.len() < limit || !target_loaded {
                        let Some(id) = walk.next() else {
                            break;
                        };
                        let commit = repo.find_commit(id?)?;
                        target_loaded |= target == Some(commit.id());
                        commits.push(commit_row(&commit, &mut references));
                    }
                    Ok(HistoryPage {
                        commits,
                        more: walk.peek().is_some(),
                    })
                };
                let page = read(INITIAL_HISTORY_ROWS)?;
                let mut next_offset = page.commits.len();
                if initial.send(Ok(page)).is_err() {
                    return Ok(());
                }
                // 취소 후 같은 오프셋으로 재요청해도 페이지를 건너뛰지 않는다.
                let mut cached: Option<(usize, HistoryPage)> = None;
                let mut failure: Option<String> = None;
                while let Ok(command) = requests.recv() {
                    let result = if let Some((offset, page)) = &cached
                        && *offset == command.offset
                    {
                        Ok(page.clone())
                    } else if let Some(error) = &failure {
                        Err(error.clone())
                    } else if command.offset != next_offset {
                        Err("커밋 조회 위치가 변경되었습니다. 저장소를 다시 여세요.".into())
                    } else {
                        match read(HISTORY_PAGE_ROWS) {
                            Ok(page) => {
                                next_offset += page.commits.len();
                                cached = Some((command.offset, page.clone()));
                                Ok(page)
                            }
                            Err(error) => {
                                let error = format!("커밋을 더 읽을 수 없습니다: {error:#}");
                                failure = Some(error.clone());
                                Err(error)
                            }
                        }
                    };
                    let _ = command.reply.send(result);
                }
                Ok(())
            };
            if let Err(error) = run() {
                let _ = initial.send(Err(format!("{error:#}")));
            }
        })
        .context("커밋 조회 작업을 시작할 수 없습니다.")?;
    let page = first
        .recv()
        .context("커밋 최초 조회가 중단되었습니다.")?
        .map_err(anyhow::Error::msg)?;
    Ok((page, HistoryCursor(std::sync::Arc::new(commands))))
}

pub fn name_from_path(path: &str) -> String {
    path.trim()
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.ends_with(':'))
        .unwrap_or_default()
        .into()
}
pub fn selected_repository_path(operation: Operation, folder: &Path) -> PathBuf {
    if operation == Operation::Open { folder.to_path_buf() } else { folder.join(".git") }
}

pub fn suggested_name(operation: Operation, path: &str) -> String {
    let path = path.trim().trim_end_matches(['/', '\\']);
    let last = name_from_path(path);
    let git_name = last.strip_prefix(".git").is_some_and(|suffix| suffix.bytes().all(|b| b.is_ascii_digit()));
    if operation != Operation::Open || git_name {
        return path.rsplit_once(['/', '\\']).map(|(parent, _)| name_from_path(parent)).unwrap_or_default();
    }
    last
}

pub fn execute(request: Request) -> Result<Project> {
    run(request)
}

pub fn stash_display_label(index: usize, description: &str) -> String {
    format!("{index}: {description}")
}

pub fn panel_references(path: &Path, stashes: bool) -> Result<Vec<String>> {
    let mut repo = open_exact(path)?;
    if stashes {
        let mut names = Vec::new();
        repo.stash_foreach(|index, name, _| {
            names.push(stash_display_label(index, name));
            true
        })?;
        Ok(names)
    } else {
        let tags = repo.tag_names(None)?;
        (0..tags.len())
            .map(|index| {
                tags.get(index)?
                    .map(str::to_owned)
                    .context("태그 이름을 읽을 수 없습니다.")
            })
            .collect()
    }
}

#[cfg(test)]
pub fn commit_files(path: &Path, id: &str) -> Result<Vec<ChangedFile>> {
    let repo = open_exact(path)?;
    let commit = repo.find_commit(git2::Oid::from_str(id)?)?;
    let tree = commit.tree()?;
    let parent = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };
    let diff = repo.diff_tree_to_tree(parent.as_ref(), Some(&tree), None)?;
    diff.deltas()
        .map(|delta| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .context("변경 파일 경로를 읽을 수 없습니다.")?;
            Ok(ChangedFile {
                status: match delta.status() {
                    git2::Delta::Added => "A",
                    git2::Delta::Deleted => "D",
                    git2::Delta::Renamed => "R",
                    git2::Delta::Typechange => "T",
                    _ => "M",
                }
                .into(),
                path: path.to_string_lossy().into_owned(),
            })
        })
        .collect()
}

fn ensure_unlinked_repository_path(path: &Path) -> Result<()> {
    // 연결된 경로를 정규화해 다른 폴더를 삭제하지 않도록 원본 구성 요소부터 검사합니다.
    for ancestor in path.ancestors() {
        let metadata =
            std::fs::symlink_metadata(ancestor).context("삭제 대상 경로를 확인하지 못했습니다.")?;
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = metadata.file_type().is_symlink();
        anyhow::ensure!(!linked, "연결된 경로의 저장소는 삭제할 수 없습니다.");
    }
    Ok(())
}

pub fn delete_repository_files(path: &Path) -> Result<()> {
    anyhow::ensure!(path.is_absolute(), "삭제 대상은 절대 경로여야 합니다.");
    ensure_unlinked_repository_path(path)?;
    let selected_repo = open_exact(path)?;
    let selected = path.canonicalize()?;
    let gitdir = selected_repo.path().canonicalize()?;
    let root = selected_repo.workdir().map(Path::canonicalize).transpose()?;
    let target = if let Some(root) = root {
        anyhow::ensure!(root.join(".git").canonicalize().ok().as_ref() == Some(&gitdir),
            "사용자 지정 또는 외부 Git 디렉터리의 작업 폴더는 삭제할 수 없습니다. 항목만 삭제하세요.");
        ensure_unlinked_repository_path(&root.join(".git"))?;
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if entry.path().canonicalize().ok().as_ref() != Some(&gitdir) && is_git_directory(&entry.path()) {
                bail!("다른 저장소와 공유하는 작업 폴더는 삭제할 수 없습니다. 항목만 삭제하세요.");
            }
        }
        anyhow::ensure!(selected == root || selected == gitdir, "저장소 루트만 삭제할 수 있습니다.");
        root
    } else { selected };
    drop(selected_repo);
    let path = target.as_path();
    ensure_unlinked_repository_path(path)?;
    let target = path
        .canonicalize()
        .context("삭제 대상 폴더를 찾을 수 없습니다.")?;
    anyhow::ensure!(
        target.parent().is_some() && target.is_dir(),
        "루트 또는 파일은 삭제할 수 없습니다."
    );
    let repo = open_exact(&target)?;
    let root = repo.workdir().unwrap_or(repo.path()).canonicalize()?;
    anyhow::ensure!(root == target, "저장소 루트 폴더만 삭제할 수 있습니다.");
    // 비베어 저장소의 .git을 별도 베어 저장소로 해석하는 경우도 거절합니다.
    anyhow::ensure!(
        target
            .file_name()
            .is_none_or(|name| !name.to_string_lossy().eq_ignore_ascii_case(".git")),
        "Git 메타데이터 폴더는 삭제 대상으로 사용할 수 없습니다."
    );
    drop(repo);
    std::fs::remove_dir_all(&target)
        .context("저장소 폴더를 영구 삭제하지 못했습니다. 일부 파일은 이미 삭제되었을 수 있습니다.")
}

pub fn avatar_repository(path: &Path) -> Result<Option<String>> {
    let repo = open_exact(path)?;
    let remotes = repo.remotes()?;
    let mut names: Vec<_> = remotes
        .iter()
        .filter_map(|name| name.ok().flatten())
        .collect();
    names.sort_by_key(|name| *name != "origin");
    let mut hosted = None;
    for name in names {
        let remote = repo.find_remote(name)?;
        let Ok(url) = remote.url() else { continue };
        if let Some(source) = crate::avatar::repository_from_remote(url) {
            return Ok(Some(source));
        }
        if hosted.is_none() {
            hosted = crate::avatar::web_repository_from_remote(url);
        }
    }
    Ok(hosted)
}

fn git_command(repo: &Repository) -> Result<std::process::Command> {
    let root = repo.workdir().context("작업 폴더가 없습니다.")?.canonicalize()?;
    let gitdir = repo.path().canonicalize()?;
    let root = crate::path_display::display_path(&root);
    let gitdir = crate::path_display::display_path(&gitdir);
    let mut command = std::process::Command::new("git");
    command
        .arg("--git-dir").arg(gitdir)
        .arg("--work-tree").arg(&root)
        .arg("-C").arg(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    Ok(command)
}

fn open_exact(path: &Path) -> Result<Repository> {
    Repository::open_ext(
        path,
        RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )
    .context("선택한 경로에 유효한 Git 저장소가 없습니다.")
}
fn credentials() -> git2::RemoteCallbacks<'static> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|url, username, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) {
            return Cred::ssh_key_from_agent(username.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            let profiles=crate::auth_profiles::load().map_err(|_|git2::Error::from_str("인증 프로필을 읽을 수 없습니다."))?;
            let resolved=crate::auth_profiles::resolve(&profiles,&crate::auth_profiles::SystemSecrets,url,username)
                .map_err(|_|git2::Error::from_str("저장된 인증을 읽을 수 없습니다."))?;
            match resolved {
                Some(crate::auth_profiles::ResolvedAuth::Direct{username,secret})=>return Cred::userpass_plaintext(&username,secret.expose()),
                Some(crate::auth_profiles::ResolvedAuth::Gcm{username,url: scope})=>{
                    let target=crate::credentials::LoginTarget::parse(&scope).and_then(|target|target.for_account(&username))
                        .map_err(|_|git2::Error::from_str("저장된 인증 주소가 올바르지 않습니다."))?;
                    let gcm=crate::credentials::Gcm::detect(&crate::settings_git::GitContext::new(None))
                        .ok_or_else(||git2::Error::from_str("Git Credential Manager를 찾을 수 없습니다."))?;
                    let (account,secret)=crate::credentials::saved_credentials(&gcm,&target,&username)
                        .map_err(|_|git2::Error::from_str("저장된 인증을 가져올 수 없습니다. 환경설정에서 다시 로그인하세요."))?;
                    return Cred::userpass_plaintext(&account,secret.expose());
                },
                None=>return Cred::credential_helper(&git2::Config::open_default()?,url,username),
            }
        }
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username.unwrap_or("git"));
        }
        Cred::default()
    });
    callbacks
}
fn validate_source(source: &str) -> Result<()> {
    if source.is_empty() || source.chars().any(char::is_control) {
        bail!("소스 경로를 입력하세요.");
    }
    let local = Path::new(source);
    if local.exists() {
        open_exact(local)
            .map_err(|_| anyhow::anyhow!("소스 경로에 유효한 Git 저장소가 없습니다."))?;
    } else {
        let url = url::Url::parse(source).ok();
        let supported = url.as_ref().is_some_and(|u| {
            matches!(u.scheme(), "http" | "https" | "ssh" | "git" | "file")
                && (u.host_str().is_some() || u.scheme() == "file")
        });
        let scp = source.split_once(':').is_some_and(|(host, path)| {
            host.contains('@') && !path.is_empty() && !host.contains(['/', '\\'])
        });
        if !supported && !scp {
            bail!("소스 경로 또는 Git 주소가 올바르지 않습니다.");
        }
    }
    let mut remote = git2::Remote::create_detached(source)
        .map_err(|_| anyhow::anyhow!("소스 주소가 올바르지 않습니다."))?;
    remote
        .connect_auth(Direction::Fetch, Some(credentials()), None)
        .map_err(|_| {
            anyhow::anyhow!(
                "소스 저장소에 접근할 수 없습니다. 주소·접근 권한·네트워크를 확인하세요."
            )
        })?;
    remote.disconnect().ok();
    Ok(())
}
fn run(request: Request) -> Result<Project> {
    let source = request.source.trim();
    if request.operation == Operation::Clone {
        validate_source(source)?;
    }
    let raw_path = request.path.trim();
    let name = request.name.trim();
    if raw_path.is_empty() || raw_path.chars().any(char::is_control) {
        bail!("저장소 경로를 입력하세요.");
    }
    if name.is_empty() || name.chars().any(char::is_control) {
        bail!("저장소 이름을 한 줄로 입력하세요.");
    }
    let path = Path::new(raw_path);
    if path.exists() && !path.is_dir() {
        bail!("저장소 경로는 폴더여야 합니다.");
    }
    match request.operation {
        Operation::Create => {
            let workdir = new_repository_workdir(path)?;
            std::fs::create_dir_all(&workdir).context("작업 폴더를 만들지 못했습니다.")?;
            let mut options = git2::RepositoryInitOptions::new();
            options.bare(true).no_reinit(true).mkdir(true).mkpath(true);
            let repo = Repository::init_opts(path, &options).context("Git 저장소를 생성하지 못했습니다.")?;
            attach_workdir(&repo, &workdir)?;
        }
        Operation::Open => {
            open_exact(path)?;
        }
        Operation::Clone => {
            let workdir = new_repository_workdir(path)?;
            if workdir.exists() {
                for entry in std::fs::read_dir(&workdir).context("대상 폴더에 접근할 수 없습니다.")? {
                    let entry = entry?;
                    // 다른 저장소의 메타데이터만 허용하며 작업 파일을 덮어쓰지 않는다.
                    if !entry.file_type()?.is_dir() || !is_git_directory(&entry.path()) {
                        bail!("복제할 작업 폴더가 비어 있지 않습니다.");
                    }
                }
            }
            std::fs::create_dir_all(&workdir).context("작업 폴더를 만들지 못했습니다.")?;
            clone_to_git_directory(source, path, &workdir).map_err(|_| {
                anyhow::anyhow!("저장소를 복제하지 못했습니다. 소스 접근 권한 및 대상 경로를 확인하세요.")
            })?;
        }
    }
    if request.operation != Operation::Open {
        hide_git_directory(path).context("Git 디렉터리는 만들어졌지만 숨김 속성을 적용하지 못했습니다.")?;
    }
    snapshot(path, name)
}

#[cfg(windows)]
fn hide_git_directory(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, GetFileAttributesW,
            INVALID_FILE_ATTRIBUTES, SetFileAttributesW,
        },
        core::PCWSTR,
    };
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let path = PCWSTR(wide.as_ptr());
    let attributes = unsafe { GetFileAttributesW(path) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(std::io::Error::last_os_error()).context("Git 디렉터리 속성을 읽지 못했습니다.");
    }
    if attributes & FILE_ATTRIBUTE_HIDDEN.0 == 0 {
        unsafe { SetFileAttributesW(path, FILE_FLAGS_AND_ATTRIBUTES(attributes | FILE_ATTRIBUTE_HIDDEN.0)) }
            .context("Git 디렉터리에 숨김 속성을 설정하지 못했습니다.")?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn hide_git_directory(_path: &Path) -> Result<()> {
    Ok(())
}

/// 작업 폴더 경로와 구분하여 새 메타데이터 경로를 검증한다.
fn new_repository_workdir(path: &Path) -> Result<PathBuf> {
    if path.symlink_metadata().is_ok() {
        bail!("지정한 Git 디렉터리 경로가 이미 존재합니다.");
    }
    let absolute = std::path::absolute(path)?;
    anyhow::ensure!(absolute.file_name().is_some(), "Git 디렉터리의 이름을 지정하세요.");
    let workdir = absolute.parent().context("부모 작업 폴더를 지정하세요.")?;
    anyhow::ensure!(workdir.parent().is_some(), "드라이브 루트에는 저장소를 생성할 수 없습니다.");
    Ok(workdir.to_path_buf())
}

fn attach_workdir(repo: &Repository, workdir: &Path) -> Result<()> {
    // update_gitlink=false로 기존 .git이나 다른 저장소 연결 파일을 건드리지 않는다.
    repo.set_workdir(workdir, false)?;
    let mut config = repo.config()?;
    config.set_bool("core.bare", false)?;
    config.set_bool("core.logallrefupdates", true)?;
    config.set_str("core.worktree", workdir.to_str().context("작업 폴더 경로를 읽을 수 없습니다.")?)?;
    Ok(())
}

fn clone_to_git_directory(source: &str, path: &Path, workdir: &Path) -> Result<()> {
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(credentials());
    let repo = git2::build::RepoBuilder::new().bare(true).fetch_options(fetch).clone(source, path)?;
    let head_name = repo.find_reference("HEAD")?.symbolic_target().ok().flatten().map(str::to_owned);
    let branches: Vec<_> = repo.references_glob("refs/heads/*")?
        .map(|reference| {
            let reference = reference?;
            Ok((reference.name()?.to_owned(), reference.target().context("브랜치 대상을 읽을 수 없습니다.")?))
        }).collect::<Result<_>>()?;
    for (name, id) in branches {
        let branch = name.strip_prefix("refs/heads/").unwrap();
        repo.reference(&format!("refs/remotes/origin/{branch}"), id, true, "clone")?;
        if head_name.as_deref() != Some(&name) { repo.find_reference(&name)?.delete()?; }
    }
    repo.config()?.set_str("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*")?;
    attach_workdir(&repo, workdir)?;
    // bare 상태에서 연 핸들의 캐시를 갱신한다.
    let repo = open_exact(path)?;
    if let Some(head) = &head_name {
        let branch = head.strip_prefix("refs/heads/").context("기본 브랜치를 읽을 수 없습니다.")?;
        match repo.find_branch(branch, git2::BranchType::Local) {
            Ok(mut local) => {
                local.set_upstream(Some(&format!("origin/{branch}")))?;
                repo.reference_symbolic("refs/remotes/origin/HEAD", &format!("refs/remotes/origin/{branch}"), true, "clone")?;
            }
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    match repo.head() {
        Ok(_) => { repo.checkout_head(Some(git2::build::CheckoutBuilder::new().safe()))?; }
        Err(error) if matches!(error.code(), git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub fn is_git_directory(path: &Path) -> bool {
    Repository::open_ext(path, RepositoryOpenFlags::NO_SEARCH | RepositoryOpenFlags::NO_DOTGIT,
        std::iter::empty::<&Path>()).is_ok()
}

pub fn git_metadata_directories(workdir: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for entry in std::fs::read_dir(workdir)? {
        let path = entry?.path();
        if is_git_directory(&path) { directories.push(path); }
    }
    Ok(directories)
}

pub fn repository_identity(path: &Path) -> PathBuf {
    open_exact(path).ok().and_then(|repo| repo.path().canonicalize().ok())
        .unwrap_or_else(|| path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

pub fn worktree_directory(path: &Path) -> Result<PathBuf> {
    let repo = open_exact(path)?;
    repo.workdir().context("작업 디렉터리가 없습니다.")?.canonicalize().context("작업 폴더를 확인할 수 없습니다.")
}

pub fn commit_snapshot(path: &Path, name: &str, id: &str) -> Result<Project> {
    let repo = open_exact(path)?; let id = git2::Oid::from_str(id)?; repo.find_commit(id)?;
    snapshot_with_target(path, name, Some(id))
}

pub fn reference_snapshot(path: &Path, name: &str, reference: &str) -> Result<Project> {
    let repo = open_exact(path)?;
    let target = repo.find_reference(reference)?.peel_to_commit()?.id();
    snapshot_with_target(path, name, Some(target))
}

fn snapshot(path: &Path, name: &str) -> Result<Project> {
    snapshot_with_target(path, name, None)
}

pub fn remote_branch_label<'a>(label: &'a str, remote_names: &[String]) -> &'a str {
    remote_names.iter()
        .filter_map(|remote| label.strip_prefix(&format!("{remote}/")).map(|branch| (remote.len(), branch)))
        .max_by_key(|(length, _)| *length)
        .map(|(_, branch)| branch)
        .or_else(|| label.split_once('/').map(|(_, branch)| branch))
        .unwrap_or(label)
}

fn snapshot_with_target(path: &Path, name: &str, target: Option<git2::Oid>) -> Result<Project> {
    let repo = open_exact(path)?;
    let root = repo
        .path()
        .canonicalize()
        .context("저장소 경로를 확인할 수 없습니다.")?;
    let head = match repo.head() {
        Ok(head) => Some(head),
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
            ) =>
        {
            None
        }
        Err(error) => return Err(error).context("??? HEAD? ?? ?????."),
    };
    let head_id = head
        .as_ref()
        .and_then(|h| h.target())
        .map(|id| id.to_string());
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand().ok())
        .map(str::to_owned)
        .or_else(|| {
            repo.find_reference("HEAD").ok().and_then(|h| {
                h.symbolic_target()
                    .ok()
                    .flatten()
                    .map(|s| s.trim_start_matches("refs/heads/").to_owned())
            })
        })
        .unwrap_or_else(|| "HEAD".into());
    let mut project = Project {
        work_state: read_work_state(&repo)?,
        name: name.into(),
        path: Some(root),
        workdir: repo.workdir().map(Path::canonicalize).transpose()?,
        branch,
        head: head_id,
        ..Default::default()
    };
    let mut remote_names = Vec::new();
    for name in repo.remotes()?.iter() {
        if let Some(name) = name? {
            remote_names.push(name.to_owned());
        }
    }
    project.remote_names = remote_names.clone();
    let mut references = std::collections::HashMap::<String, Vec<CommitReference>>::new();
    for item in repo.references()? {
        let reference = item?;
        let label = reference.shorthand().unwrap_or_default().to_owned();
        if reference.is_branch() {
            project.branches.push(label.clone());
            match repo.branch_upstream_name(reference.name()?) {
                Ok(upstream) => {
                    if let Some(remote) = upstream.as_str()?.strip_prefix("refs/remotes/") {
                        project.branch_upstreams.insert(label.clone(), remote.to_owned());
                    }
                }
                Err(error) if error.code() == git2::ErrorCode::NotFound => {},
                Err(error) => return Err(error).context("브랜치 추적 정보를 읽을 수 없습니다."),
            }

        }
        if reference.is_remote() {
            project.remotes.push(label.clone());
        }
        if let Ok(object) = reference.peel(git2::ObjectType::Commit) {
            references
                .entry(object.id().to_string())
                .or_default()
                .push(CommitReference {
                    full_name: reference.name()?.to_owned(),
                    label: if reference.is_remote() {
                        remote_branch_label(&label, &remote_names).to_owned()
                    } else {
                        label.clone()
                    },
                    name: label,
                    is_remote: reference.is_remote(),
                });
        }
    }
    project.branches.sort();
    project.remotes.sort();
    if target.is_some()
        || project.head.is_some()
        || !project.branches.is_empty()
        || !project.remotes.is_empty()
    {
        let mut tips = Vec::new();
        if let Some(head) = project.head.as_ref() {
            tips.push(git2::Oid::from_str(head)?);
        }
        for pattern in ["refs/heads/*", "refs/remotes/*"] {
            for reference in repo.references_glob(pattern)? {
                if let Ok(commit) = reference?.peel_to_commit() {
                    tips.push(commit.id());
                }
            }
        }
        if let Some(target) = target {
            tips.push(target);
        }
        let (page, cursor) = start_history(
            project.path.as_deref().expect("저장소 경로"),
            tips,
            target,
            references,
        )?;
        project.commits = page.commits;
        project.more_commits = page.more;
        project.history_cursor = page.more.then_some(cursor);
    }

    if !repo.is_bare() {
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        let workdir = repo.workdir().context("작업 폴더가 없습니다.")?;
        let metadata_dirs = git_metadata_directories(workdir)?;
        for entry in repo.statuses(Some(&mut options))?.iter() {
            if let Ok(path) = entry.path() {
                let absolute = workdir.join(path);
                if metadata_dirs.iter().any(|dir| absolute.starts_with(dir)) { continue; }
            }
            let status = entry.status();
            let label = if status.is_conflicted() {
                "!"
            } else if status.is_wt_new() || status.is_index_new() {
                "A"
            } else if status.is_wt_deleted() || status.is_index_deleted() {
                "D"
            } else if status.is_wt_renamed() || status.is_index_renamed() {
                "R"
            } else {
                "M"
            };
            project.files.push(ChangedFile {
                status: label.into(),
                path: entry.path().unwrap_or("표시할 수 없는 경로").into(),
            });
        }
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_operation_snapshot_tracks_merge_conflict_resolution_and_cleanup() {
        let dir = tempdir(); seed(dir.path());
        let repo = Repository::open(dir.path()).unwrap();
        assert!(work_state(dir.path()).unwrap().operation.is_none());
        let base = repo.head().unwrap().peel_to_commit().unwrap().id();
        let make_commit = |text: &str| {
            std::fs::write(dir.path().join("tracked.txt"), text).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("tracked.txt")).unwrap(); index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, text, &tree, &[&parent]).unwrap()
        };
        let theirs = make_commit("incoming\n");
        repo.reset(repo.find_commit(base).unwrap().as_object(), git2::ResetType::Hard, None).unwrap();
        make_commit("ours\n");
        repo.merge(&[&repo.find_annotated_commit(theirs).unwrap()], None, None).unwrap();
        let loaded = snapshot(dir.path(), "fixture").unwrap();
        assert_eq!(loaded.work_state.operation, Some(git2::RepositoryState::Merge));
        assert!(loaded.work_state.has_conflicts);
        std::fs::write(dir.path().join("tracked.txt"), "resolved\n").unwrap();
        let mut index = repo.index().unwrap();
        index.read(true).unwrap(); index.add_path(Path::new("tracked.txt")).unwrap(); index.write().unwrap();
        let resolved = work_state(dir.path()).unwrap();
        assert_eq!(resolved.operation, Some(git2::RepositoryState::Merge));
        assert!(!resolved.has_conflicts);
        repo.cleanup_state().unwrap();
        assert!(work_state(dir.path()).unwrap().operation.is_none());
        assert!(work_state(&dir.path().join("missing")).is_err());
    }
    #[test]
    fn remote_branch_labels_remove_only_the_configured_remote_prefix() {
        let remotes = vec!["origin".into(), "team".into(), "team/upstream".into()];
        for (full, expected) in [
            ("origin/main", "main"),
            ("origin/feature/login", "feature/login"),
            ("team/upstream/feature/login", "feature/login"),
            ("team/topic", "topic"),
            ("unknown/feature/login", "feature/login"),
            ("main", "main"),
            ("", ""),
        ] {
            assert_eq!(remote_branch_label(full, &remotes), expected);
        }
    }

    #[test]
    fn branch_upstreams_follow_configuration_and_exclude_local_tracking() {
        let dir = tempdir();
        seed(dir.path());
        let repo = Repository::open(dir.path()).unwrap();
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.remote("origin", dir.path().to_str().unwrap()).unwrap();
        repo.reference("refs/remotes/origin/different", commit.id(), true, "fixture").unwrap();
        let mut tracked = repo.branch("local", &commit, false).unwrap();
        tracked.set_upstream(Some("origin/different")).unwrap();
        repo.branch("untracked", &commit, false).unwrap();
        let mut local = repo.branch("local-tracking", &commit, false).unwrap();
        local.set_upstream(Some("local")).unwrap();
        let project = snapshot(dir.path(), "fixture").unwrap();
        assert_eq!(project.branch_upstreams.get("local").map(String::as_str), Some("origin/different"));
        assert!(!project.branch_upstreams.contains_key("untracked"));
        assert!(!project.branch_upstreams.contains_key("local-tracking"));
        tracked.set_upstream(None).unwrap();
        assert!(!snapshot(dir.path(), "fixture").unwrap().branch_upstreams.contains_key("local"));
    }

    #[test]
    fn github_remote_prefers_origin_then_other_github_remotes() {
        let dir = tempdir();
        let repo = Repository::init(dir.path()).unwrap();
        assert_eq!(avatar_repository(dir.path()).unwrap(), None);
        repo.remote("origin", "https://git.example.test/owner/project.git")
            .unwrap();
        assert_eq!(
            avatar_repository(dir.path()).unwrap().as_deref(),
            Some("https://git.example.test/owner/project.git")
        );
        repo.remote("upstream", "git@github.com:upstream/project.git")
            .unwrap();
        assert_eq!(
            avatar_repository(dir.path()).unwrap().as_deref(),
            Some("upstream/project")
        );
        repo.remote_set_url("origin", "https://github.com/fork/project.git")
            .unwrap();
        assert_eq!(
            avatar_repository(dir.path()).unwrap().as_deref(),
            Some("fork/project")
        );
    }

    #[test]
    fn permanent_delete_removes_exact_repository_and_preserves_sibling() {
        let dir = tempdir();
        let target = dir.path().join("delete-me");
        seed(&target);
        std::fs::write(target.join("untracked.txt"), "work").unwrap();
        let sibling = dir.path().join("keep.txt");
        std::fs::write(&sibling, "keep").unwrap();
        let target = target.canonicalize().unwrap();
        assert!(target.starts_with(dir.path().canonicalize().unwrap()));
        delete_repository_files(&target).unwrap();
        assert!(!target.exists());
        assert_eq!(std::fs::read_to_string(sibling).unwrap(), "keep");
    }

    #[cfg(windows)]
    #[test]
    fn permanent_delete_does_not_follow_junctions() {
        let dir = tempdir();
        let target = dir.path().join("repository");
        let outside = dir.path().join("outside");
        seed(&target);
        seed(&outside);
        let link = dir.path().join("alias");
        let create = |link: &Path, destination: &Path| {
            let output = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(link.to_string_lossy().replace('/', "\\"))
                .arg(destination.to_string_lossy().replace('/', "\\"))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        create(&link, &target);
        assert!(delete_repository_files(&link).is_err());
        assert!(target.join("tracked.txt").exists());
        create(&target.join("external"), &outside);
        let target = target.canonicalize().unwrap();
        assert!(target.starts_with(dir.path().canonicalize().unwrap()));
        delete_repository_files(&target).unwrap();
        assert!(outside.join("tracked.txt").exists());
        assert!(outside.join(".git").exists());
    }

    #[cfg(windows)]
    #[test]
    fn permanent_delete_locked_file_reports_failure() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempdir();
        let target = dir.path().join("locked");
        seed(&target);
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(target.join("tracked.txt"))
            .unwrap();
        assert!(delete_repository_files(&target.canonicalize().unwrap()).is_err());
        assert!(target.exists());
        drop(lock);
    }

    #[test]
    fn permanent_delete_rejects_non_root_and_non_repository_paths() {
        let dir = tempdir();
        let target = dir.path().join("keep");
        seed(&target);
        let child = target.join("child");
        std::fs::create_dir(&child).unwrap();
        for path in [
            dir.path().to_path_buf(),
            child,
            target.join("missing"),
        ] {
            assert!(delete_repository_files(&path.canonicalize().unwrap_or(path)).is_err());
        }
        assert!(delete_repository_files(Path::new(".")).is_err());
        assert!(target.join("tracked.txt").exists());
        assert!(target.join(".git").exists());
    }

    #[test]
    fn reference_kind_uses_full_namespace_instead_of_label() {
        for (full_name, kind) in [
            ("refs/heads/shared", ReferenceKind::Local),
            ("refs/remotes/origin/shared", ReferenceKind::Remote),
            ("refs/tags/shared", ReferenceKind::Tag),
            ("refs/heads/origin/tags/v1", ReferenceKind::Local),
            ("refs/tags/origin/main", ReferenceKind::Tag),
            ("refs/notes/commits", ReferenceKind::Other),
            ("", ReferenceKind::Other),
        ] {
            let reference = CommitReference {
                full_name: full_name.into(),
                name: "same".into(),
                label: "same".into(),
                is_remote: full_name.starts_with("refs/remotes/"),
            };
            assert_eq!(reference.kind(), kind);
        }
    }

    #[test]
    fn reference_snapshot_loads_exact_targets_beyond_initial_limit_and_tag_only_history() {
        let dir = tempdir();
        seed(dir.path());
        let repo = Repository::open(dir.path()).unwrap();
        let first = repo.head().unwrap().peel_to_commit().unwrap();
        let first_id = first.id();
        let tree = first.tree().unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let mut latest = first_id;
        for _ in 0..INITIAL_HISTORY_ROWS + 10 {
            let parent = repo.find_commit(latest).unwrap();
            latest = repo
                .commit(Some("HEAD"), &sig, &sig, "next", &tree, &[&parent])
                .unwrap();
        }
        let orphan = repo
            .commit(None, &sig, &sig, "tag only", &tree, &[])
            .unwrap();
        repo.reference("refs/heads/shared", first_id, true, "fixture")
            .unwrap();
        repo.reference("refs/remotes/origin/shared", latest, true, "fixture")
            .unwrap();
        let object = repo.find_object(orphan, None).unwrap();
        repo.tag_lightweight("shared", &object, false).unwrap();
        let old_object = repo.find_object(first_id, None).unwrap();
        repo.tag("annotated", &old_object, &sig, "old commit", false)
            .unwrap();
        let initial = snapshot(dir.path(), "fixture").unwrap();
        assert_eq!(initial.commits.len(), INITIAL_HISTORY_ROWS);
        assert!(initial.more_commits);
        assert!(
            !initial
                .commits
                .iter()
                .any(|commit| commit.id == first_id.to_string())
        );
        for (name, target) in [
            ("refs/heads/shared", first_id),
            ("refs/remotes/origin/shared", latest),
            ("refs/tags/shared", orphan),
            ("refs/tags/annotated", first_id),
        ] {
            let loaded = reference_snapshot(dir.path(), "fixture", name).unwrap();
            let commit = loaded
                .commits
                .iter()
                .find(|commit| commit.id == target.to_string())
                .unwrap();
            assert!(
                commit
                    .references
                    .iter()
                    .any(|reference| reference.full_name == name)
            );
        }
        let blob = repo.blob(b"not a commit").unwrap();
        let blob = repo.find_object(blob, None).unwrap();
        repo.tag_lightweight("blob", &blob, false).unwrap();
        assert!(reference_snapshot(dir.path(), "fixture", "refs/tags/blob").is_err());
        assert!(reference_snapshot(dir.path(), "fixture", "refs/heads/missing").is_err());
        assert_eq!(repo.head().unwrap().target(), Some(latest));
    }

    #[test]
    fn commit_references_keep_git_kind_and_shorten_only_remote_names() {
        let dir = tempdir();
        seed(dir.path());
        let repo = Repository::open(dir.path()).unwrap();
        let head = repo.head().unwrap().target().unwrap();
        for remote in ["origin", "upstream", "team", "team/upstream"] {
            repo.remote(remote, "https://example.invalid/repository.git")
                .unwrap();
        }
        for name in [
            "refs/remotes/origin/main",
            "refs/remotes/upstream/main",
            "refs/remotes/upstream/feature/login,extra",
            "refs/remotes/team/upstream/feature/login",
            "refs/remotes/archive/feature/login",
            "refs/heads/origin/topic",
            "refs/heads/remotes/test",
            "refs/tags/origin/release",
        ] {
            repo.reference(name, head, true, "fixture").unwrap();
        }
        let project = snapshot(dir.path(), "fixture").unwrap();
        let commit = project
            .commits
            .iter()
            .find(|c| c.id == head.to_string())
            .unwrap();
        for (name, label, remote) in [
            ("origin/main", "main", true),
            ("upstream/main", "main", true),
            ("upstream/feature/login,extra", "feature/login,extra", true),
            ("team/upstream/feature/login", "feature/login", true),
            ("archive/feature/login", "feature/login", true),
            ("origin/topic", "origin/topic", false),
            ("remotes/test", "remotes/test", false),
            ("origin/release", "origin/release", false),
        ] {
            let reference = commit.references.iter().find(|r| r.name == name).unwrap();
            assert_eq!(reference.label, label);
            assert_eq!(reference.is_remote, remote);
            assert!(commit.reference.contains(name));
        }
        assert!(project.remotes.contains(&"origin/main".to_owned()));
        assert!(project.remotes.contains(&"upstream/main".to_owned()));
        assert!(!project.remotes.contains(&"origin/topic".to_owned()));
    }

    fn tempdir() -> tempfile::TempDir {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/repository-tests");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }
    fn request(operation: Operation, path: &std::path::Path) -> Request {
        Request {
            operation,
            source: String::new(),
            path: if operation == Operation::Open { path.to_path_buf() } else { path.join(".git") }.to_string_lossy().into(),
            name: "테스트 저장소".into(),
        }
    }
    fn seed(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        std::fs::write(path.join("tracked.txt"), "original").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();
    }
    #[test]
    fn stash_display_names_preserve_description_without_reference_wrapper() {
        assert_eq!(stash_display_label(0, "On main: named stash"), "0: On main: named stash");
        assert_eq!(stash_display_label(12, "WIP on feature/topic: message: details"), "12: WIP on feature/topic: message: details");
    }

    #[test]
    fn panel_lists_tags_stashes_and_empty_states() {
        let dir = tempdir();
        seed(dir.path());
        assert!(panel_references(dir.path(), false).unwrap().is_empty());
        assert!(panel_references(dir.path(), true).unwrap().is_empty());
        let mut repo = Repository::open(dir.path()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap().id();
        let object = repo.find_object(head, None).unwrap();
        repo.tag_lightweight("v1.0", &object, false).unwrap();
        drop(object);
        std::fs::write(dir.path().join("tracked.txt"), "modified").unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        repo.stash_save(&sig, "saved work", None).unwrap();
        assert_eq!(panel_references(dir.path(), false).unwrap(), ["v1.0"]);
        let stashes = panel_references(dir.path(), true).unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].starts_with("0: ") && stashes[0].contains("saved work"));
        assert_eq!(repo.head().unwrap().target(), Some(head));
    }

    #[test]
    fn history_files_follow_root_and_later_commit_without_worktree_changes() {
        let dir = tempdir();
        seed(dir.path());
        let repo = Repository::open(dir.path()).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let files = commit_files(dir.path(), &parent.id().to_string()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!((&*files[0].status, &*files[0].path), ("A", "tracked.txt"));
        std::fs::remove_file(dir.path().join("tracked.txt")).unwrap();
        std::fs::write(dir.path().join("new.txt"), "new").unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("tracked.txt")).unwrap();
        index.add_path(Path::new("new.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let id = repo
            .commit(Some("HEAD"), &sig, &sig, "next", &tree, &[&parent])
            .unwrap();
        let files = commit_files(dir.path(), &id.to_string()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|f| f.path == "tracked.txt" && f.status == "D")
        );
        assert!(files.iter().any(|f| f.path == "new.txt" && f.status == "A"));
        assert!(repo.statuses(None).unwrap().is_empty());
        assert!(commit_files(dir.path(), "invalid").is_err());
        assert!(commit_files(dir.path(), &git2::Oid::ZERO_SHA1.to_string()).is_err());
    }

    #[test]
    fn panel_reads_reject_non_repository_paths() {
        let dir = tempdir();
        assert!(panel_references(dir.path(), false).is_err());
        assert!(panel_references(dir.path(), true).is_err());
        assert!(commit_files(dir.path(), &git2::Oid::ZERO_SHA1.to_string()).is_err());
    }

    #[test]
    fn create_missing_parents_and_reject_empty_fields_or_file_targets() {
        let dir = tempdir();
        let path = dir.path().join("parent/child");
        let project = execute(request(Operation::Create, &path)).unwrap();
        assert_eq!(project.path, Some(path.join(".git").canonicalize().unwrap()));
        let mut req = request(Operation::Open, &path);
        req.path.clear();
        assert!(execute(req).is_err());
        let file = dir.path().join("file");
        std::fs::write(&file, "keep").unwrap();
        for operation in [Operation::Create, Operation::Open] {
            assert!(execute(request(operation, &file)).is_err());
        }
        assert_eq!(std::fs::read_to_string(file).unwrap(), "keep");
    }

    #[test]
    fn clone_empty_source_into_existing_empty_directory() {
        let dir = tempdir();
        let source = dir.path().join("empty-source");
        git2::Repository::init(&source).unwrap();
        let target = dir.path().join("empty-target");
        std::fs::create_dir(&target).unwrap();
        let mut req = request(Operation::Clone, &target);
        req.source = url::Url::from_directory_path(&source).unwrap().to_string();
        let project = execute(req).unwrap();
        assert!(project.commits.is_empty());
        assert!(git2::Repository::open(&target).is_ok());
    }

    #[test]
    fn invalid_source_error_does_not_expose_credentials_or_create_target() {
        let dir = tempdir();
        let target = dir.path().join("untouched");
        let mut req = request(Operation::Clone, &target);
        req.source = "unsupported://user:secret@invalid/repo".into();
        let message = execute(req).unwrap_err().to_string();
        assert!(!message.contains("secret"));
        assert!(!target.exists());
    }

    #[test]
    fn create_open_existing_and_preserve_files() {
        let dir = tempdir();
        let path = dir.path().join("한글 저장소");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("keep.txt"), "keep").unwrap();
        let created = execute(request(Operation::Create, &path)).unwrap();
        assert!(created.commits.is_empty());
        assert_eq!(created.name, "테스트 저장소");
        assert!(execute(request(Operation::Create, &path)).is_err());
        assert_eq!(
            std::fs::read_to_string(path.join("keep.txt")).unwrap(),
            "keep"
        );
        assert!(execute(request(Operation::Open, &path)).is_ok());
    }
    #[test]
    fn parent_repository_is_not_mistaken_for_selected_folder() {
        let dir = tempdir();
        seed(dir.path());
        let child = dir.path().join("nested");
        std::fs::create_dir(&child).unwrap();
        assert!(execute(request(Operation::Open, &child)).is_err());
        assert!(execute(request(Operation::Create, &child)).is_ok());
    }
    #[test]
    fn clone_validates_source_before_target_and_preserves_nonempty_target() {
        let dir = tempdir();
        let source = dir.path().join("source");
        seed(&source);
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join(".hidden"), "keep").unwrap();
        let mut req = request(Operation::Clone, &target);
        req.source = dir.path().join("invalid").to_string_lossy().into();
        assert!(
            execute(req.clone())
                .unwrap_err()
                .to_string()
                .contains("소스")
        );
        req.source = source.to_string_lossy().into();
        assert!(execute(req).unwrap_err().to_string().contains("비어"));
        assert_eq!(
            std::fs::read_to_string(target.join(".hidden")).unwrap(),
            "keep"
        );
    }
    #[test]
    fn clone_loads_real_history_and_changes() {
        let dir = tempdir();
        let source = dir.path().join("source");
        seed(&source);
        let target = dir.path().join("clone");
        let mut req = request(Operation::Clone, &target);
        req.source = source.to_string_lossy().into();
        let project = execute(req).unwrap();
        assert_eq!(project.commits.len(), 1);
        assert_eq!(project.commits[0].message, "initial commit");
        assert_eq!(
            std::fs::read_to_string(target.join("tracked.txt")).unwrap(),
            "original"
        );
        std::fs::write(target.join("new.txt"), "new").unwrap();
        let opened = execute(request(Operation::Open, &target)).unwrap();
        assert!(opened.files.iter().any(|f| f.path == "new.txt"));
        assert!(!opened.branches.is_empty());
    }
    #[test]
    fn invalid_fields_fake_git_and_bare_repository() {
        let dir = tempdir();
        let path = dir.path().join("fake");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join(".git"), "not git").unwrap();
        assert!(execute(request(Operation::Create, &path)).is_err());
        assert!(execute(request(Operation::Open, &path)).is_err());
        let mut req = request(Operation::Open, &path);
        req.name = " ".into();
        assert!(execute(req).is_err());
        let bare = dir.path().join("bare.git");
        git2::Repository::init_bare(&bare).unwrap();
        assert!(
            execute(request(Operation::Open, &bare))
                .unwrap()
                .commits
                .is_empty()
        );
    }
    #[cfg(windows)]
    #[test]
    fn hidden_attribute_preserves_other_attributes_is_idempotent_and_reports_missing_path() {
        use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};
        use windows::{core::PCWSTR, Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, SetFileAttributesW,
        }};
        let dir = tempdir();
        let path = dir.path().join("사용자 지정");
        std::fs::create_dir(&path).unwrap();
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let original = std::fs::metadata(&path).unwrap().file_attributes() | FILE_ATTRIBUTE_ARCHIVE.0;
        unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(original)) }.unwrap();
        hide_git_directory(&path).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().file_attributes(), original | FILE_ATTRIBUTE_HIDDEN.0);
        hide_git_directory(&path).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().file_attributes(), original | FILE_ATTRIBUTE_HIDDEN.0);
        assert!(hide_git_directory(&dir.path().join("missing")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn opening_existing_git_directory_does_not_change_hidden_attribute() {
        use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};
        use windows::{core::PCWSTR, Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, SetFileAttributesW,
        }};
        let dir = tempdir();
        seed(dir.path());
        let gitdir = dir.path().join(".git");
        let wide: Vec<u16> = gitdir.as_os_str().encode_wide().chain(Some(0)).collect();
        let visible = std::fs::metadata(&gitdir).unwrap().file_attributes() & !FILE_ATTRIBUTE_HIDDEN.0;
        unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(visible)) }.unwrap();
        execute(request(Operation::Open, &gitdir)).unwrap();
        assert_eq!(std::fs::metadata(&gitdir).unwrap().file_attributes(), visible);
    }

    #[cfg(windows)]
    #[test]
    fn new_git_directories_are_hidden_for_create_and_clone_without_hiding_worktree() {
        use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
        let dir = tempdir();
        let source = dir.path().join("source");
        seed(&source);
        for operation in [Operation::Create, Operation::Clone] {
            for name in [".git", ".git2", "사용자 지정"] {
                let root = dir.path().join(format!("{operation:?}-{name}"));
                let gitdir = root.join(name);
                let mut req = request(operation, &root);
                req.path = gitdir.to_string_lossy().into_owned();
                req.source = source.to_string_lossy().into_owned();
                execute(req).unwrap();
                assert_ne!(std::fs::metadata(&gitdir).unwrap().file_attributes() & FILE_ATTRIBUTE_HIDDEN.0, 0,
                    "{operation:?}: {name}");
                assert_eq!(std::fs::metadata(&root).unwrap().file_attributes() & FILE_ATTRIBUTE_HIDDEN.0, 0);
                if operation == Operation::Clone {
                    assert_eq!(std::fs::metadata(root.join("tracked.txt")).unwrap().file_attributes() & FILE_ATTRIBUTE_HIDDEN.0, 0);
                }
            }
        }
    }

    #[test]
    fn explicit_git_directories_create_independent_repositories_in_one_worktree() {
        let dir = tempdir();
        let root = dir.path().join("한글 프로젝트");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("keep.txt"), "keep").unwrap();
        let mut identities = Vec::new();
        for name in [".git", ".git2"] {
            let gitdir = root.join(name);
            let mut req = request(Operation::Create, &gitdir);
            req.path = gitdir.to_string_lossy().into_owned();
            let project = execute(req).unwrap();
            assert!(gitdir.join("HEAD").is_file());
            assert!(!gitdir.join(".git").exists());
            assert_eq!(project.path, Some(gitdir.canonicalize().unwrap()));
            let repo = open_exact(&gitdir).unwrap();
            assert_eq!(repo.workdir().unwrap().canonicalize().unwrap(), root.canonicalize().unwrap());
            identities.push(project.path);
        }
        assert_ne!(identities[0], identities[1]);
        assert_eq!(std::fs::read_to_string(root.join("keep.txt")).unwrap(), "keep");
    }

    #[test]
    fn explicit_clone_uses_parent_worktree_and_remote_tracking() {
        let dir = tempdir();
        let source = dir.path().join("source");
        seed(&source);
        let source_repo = open_exact(&source).unwrap();
        let head = source_repo.head().unwrap().peel_to_commit().unwrap();
        source_repo.branch("other", &head, false).unwrap();
        for name in [".git", ".git2"] {
            let root = dir.path().join(format!("clone-{name}"));
            let gitdir = root.join(name);
            let mut req = request(Operation::Clone, &gitdir);
            req.path = gitdir.to_string_lossy().into_owned();
            req.source = source.to_string_lossy().into_owned();
            let project = execute(req).unwrap();
            assert_eq!(project.path, Some(gitdir.canonicalize().unwrap()));
            assert_eq!(std::fs::read_to_string(root.join("tracked.txt")).unwrap(), "original");
            let repo = open_exact(&gitdir).unwrap();
            let exports = crate::history_temp::Session::default();
            let exported = crate::file_history::export_revision(&gitdir, &project.commits[0].id, b"tracked.txt", &exports).unwrap();
            assert_eq!(std::fs::read(&exported).unwrap(), b"original");
            exports.close();
            assert!(!repo.is_bare());
            assert!(repo.find_reference("refs/remotes/origin/other").is_ok());
            assert!(repo.find_branch("other", git2::BranchType::Local).is_err());
            assert!(repo.find_branch(&project.branch, git2::BranchType::Local).unwrap().upstream().is_ok());
            std::fs::write(root.join("tracked.txt"), "changed").unwrap();
            let files = crate::diff_git::load_stage(&gitdir).unwrap();
            assert!(files.iter().any(|file| file.path == b"tracked.txt"));
            assert!(!files.iter().any(|file| file.label.starts_with(name)));
            let file = files.iter().find(|file| file.path == b"tracked.txt").unwrap();
            crate::diff_git::toggle_stage(&gitdir, file).unwrap();
            assert_eq!(repo.index().unwrap().get_path(Path::new("tracked.txt"), 0).unwrap().id, repo.blob(b"changed").unwrap());
        }
    }

    #[test]
    fn shared_worktree_metadata_is_hidden_and_cannot_be_staged_discarded_or_deleted() {
        let dir = tempdir();
        let root = dir.path().join("project");
        seed(&root);
        let second = root.join(".git2");
        let mut req = request(Operation::Create, &second);
        req.path = second.to_string_lossy().into_owned();
        execute(req).unwrap();
        for gitdir in [root.join(".git"), second.clone()] {
            let project = execute(request(Operation::Open, &gitdir)).unwrap();
            assert!(project.files.iter().all(|file| !file.path.starts_with(".git")));
            let files = crate::diff_git::load_stage(&gitdir).unwrap();
            assert!(files.iter().all(|file| !file.label.starts_with(".git")));
            assert!(delete_repository_files(&gitdir.canonicalize().unwrap()).is_err());
        }
        let files = crate::diff_git::load_stage(&second).unwrap();
        let first_index = std::fs::read(root.join(".git/index")).unwrap();
        let tracked = files.iter().find(|file| file.path == b"tracked.txt").unwrap();
        crate::diff_git::toggle_stage(&second, tracked).unwrap();
        assert_eq!(std::fs::read(root.join(".git/index")).unwrap(), first_index);
        let mut forged = files.iter().find(|file| file.path == b"tracked.txt").unwrap().clone();
        forged.path = b".git2/config".to_vec();
        forged.label = ".git2/config".into();
        let config = std::fs::read(second.join("config")).unwrap();
        assert!(crate::diff_git::toggle_stage(&second, &forged).is_err());
        assert!(crate::diff_git::set_worktree_content(&second, &forged, &config, b"bad").is_err());
        assert_eq!(std::fs::read(second.join("config")).unwrap(), config);
        assert_eq!(std::fs::read_to_string(root.join("tracked.txt")).unwrap(), "original");
    }

    #[test]
    fn direct_default_git_directory_deletion_removes_only_its_unshared_worktree() {
        let dir = tempdir();
        let root = dir.path().join("delete");
        seed(&root);
        let gitdir = root.join(".git").canonicalize().unwrap();
        assert!(gitdir.starts_with(dir.path().canonicalize().unwrap()));
        delete_repository_files(&gitdir).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn clone_detached_source_checks_out_its_head_in_parent_worktree() {
        let dir = tempdir();
        let source = dir.path().join("source");
        seed(&source);
        let repo = open_exact(&source).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let head = repo.commit(None, &sig, &sig, "detached", &parent.tree().unwrap(), &[&parent]).unwrap();
        repo.set_head_detached(head).unwrap();
        let root = dir.path().join("clone");
        let gitdir = root.join(".git2");
        let mut req = request(Operation::Clone, &gitdir);
        req.path = gitdir.to_string_lossy().into_owned();
        req.source = source.to_string_lossy().into_owned();
        let project = execute(req).unwrap();
        assert_eq!(project.head, Some(head.to_string()));
        assert_eq!(std::fs::read(root.join("tracked.txt")).unwrap(), b"original");
        assert!(open_exact(&gitdir).unwrap().head_detached().unwrap());
    }

    #[test]
    fn clone_can_share_empty_worktree_without_overwriting_existing_git_directory() {
        let dir = tempdir();
        let source = dir.path().join("source");
        seed(&source);
        let root = dir.path().join("target");
        let first = Repository::init(&root).unwrap();
        let original_config = std::fs::read(first.path().join("config")).unwrap();
        let second = root.join(".git2");
        let mut req = request(Operation::Clone, &second);
        req.path = second.to_string_lossy().into_owned();
        req.source = source.to_string_lossy().into_owned();
        execute(req).unwrap();
        assert_eq!(std::fs::read(first.path().join("config")).unwrap(), original_config);
        assert!(root.join("tracked.txt").is_file());
    }

    #[test]
    fn explicit_create_never_reuses_existing_git_directory() {
        let dir = tempdir();
        let target = dir.path().join(".git2");
        std::fs::create_dir(&target).unwrap();
        let mut req = request(Operation::Create, &target);
        req.path = target.to_string_lossy().into_owned();
        assert!(execute(req).is_err());
        assert!(std::fs::read_dir(&target).unwrap().next().is_none());
    }

    #[test]
    fn names_follow_final_folder() {
        assert_eq!(name_from_path("D:\\work\\repo\\"), "repo");
        assert_eq!(name_from_path("/work/한글/"), "한글");
        assert_eq!(name_from_path("D:\\"), "");
        assert_eq!(name_from_path(""), "");
    }
}

#[cfg(test)]
mod pagination_tests {
    use super::*;
    fn fixture(count: usize) -> (tempfile::TempDir, Vec<git2::Oid>) {
        let base = std::env::current_dir()
            .unwrap()
            .join("target/history-pagination-tests");
        std::fs::create_dir_all(&base).unwrap();
        let dir = tempfile::tempdir_in(base).unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let tree_id = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Pager", "pager@example.invalid").unwrap();
        let mut ids = Vec::new();
        for n in 0..count {
            let parent = ids.last().map(|id| repo.find_commit(*id).unwrap());
            let parents: Vec<_> = parent.iter().collect();
            let id = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("commit {n}"),
                    &tree,
                    &parents,
                )
                .unwrap();
            ids.push(id);
        }
        (dir, ids)
    }
    #[test]
    fn history_paging_reads_1500_then_200_without_duplicates_or_restart() {
        let (dir, ids) = fixture(1905);
        let mut project = snapshot(dir.path(), "pages").unwrap();
        assert_eq!(project.commits.len(), INITIAL_HISTORY_ROWS);
        let cursor = project.history_cursor.clone().unwrap();
        // 새 HEAD가 생겨도 현재 탐색은 최초 스냅샷에서 계속되어야 한다.
        let repo = Repository::open(dir.path()).unwrap();
        let parent = repo.find_commit(ids[1904]).unwrap();
        let tree = parent.tree().unwrap();
        let sig = git2::Signature::now("Pager", "pager@example.invalid").unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "new after snapshot",
            &tree,
            &[&parent],
        )
        .unwrap();
        assert!(history_page(&cursor, 1501).is_err());
        // 결과 수신을 취소한 뒤에도 같은 오프셋에서 재전달한다.
        let (reply, canceled) = std::sync::mpsc::channel();
        cursor
            .0
            .send(HistoryPageCommand {
                offset: 1500,
                reply,
            })
            .unwrap();
        drop(canceled);
        let first = history_page(&cursor, 1500).unwrap();
        let duplicate = history_page(&cursor, 1500).unwrap();
        assert_eq!(
            first.commits.iter().map(|c| &c.id).collect::<Vec<_>>(),
            duplicate.commits.iter().map(|c| &c.id).collect::<Vec<_>>()
        );
        assert_eq!(first.commits.len(), HISTORY_PAGE_ROWS);
        project.commits.extend(first.commits);
        assert_eq!(project.commits.len(), 1700);
        let second = history_page(&cursor, 1700).unwrap();
        assert!(second.more);
        project.commits.extend(second.commits);
        assert_eq!(project.commits.len(), 1900);
        assert!(history_page(&cursor, 1500).is_err());
        let last = history_page(&cursor, 1900).unwrap();
        assert_eq!(last.commits.len(), 5);
        assert!(!last.more);
        project.commits.extend(last.commits);
        assert_eq!(
            project
                .commits
                .iter()
                .map(|c| c.id.clone())
                .collect::<Vec<_>>(),
            ids.iter()
                .rev()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
        assert!(!history_page(&cursor, 1905).unwrap().more);
        assert!(history_page(&cursor, 1905).unwrap().commits.is_empty());
        // 초기 경계와 오래된 참조 확장에서도 다음 항목을 유실하지 않는다.
        for (tip, expected, more) in [(ids[1499], 1500, false), (ids[1500], 1500, true)] {
            let (page, cursor) =
                start_history(dir.path(), vec![tip], None, Default::default()).unwrap();
            assert_eq!(page.commits.len(), expected);
            assert_eq!(page.more, more);
            if more {
                assert_eq!(history_page(&cursor, expected).unwrap().commits.len(), 1);
            }
        }
        let (page, _) = start_history(
            dir.path(),
            vec![ids[1904]],
            Some(ids[0]),
            Default::default(),
        )
        .unwrap();
        assert_eq!(page.commits.len(), 1905);
        assert!(!page.more);
    }
    #[test]
    fn empty_history_and_closed_cursor_are_explicit() {
        let (dir, _) = fixture(0);
        let project = snapshot(dir.path(), "empty").unwrap();
        assert!(project.commits.is_empty() && !project.more_commits);
        assert!(project.history_cursor.is_none());
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        let cursor = HistoryCursor(std::sync::Arc::new(sender));
        assert!(
            history_page(&cursor, 0)
                .unwrap_err()
                .to_string()
                .contains("종료")
        );
    }
}

fn index_conflict_paths(repo: &git2::Repository) -> Result<std::collections::BTreeSet<PathBuf>> {
    let mut index = repo.index()?;
    index.read(true)?;
    conflict_paths_from_index(&index)
}

fn conflict_paths_from_index(index: &git2::Index) -> Result<std::collections::BTreeSet<PathBuf>> {
    use gix::bstr::ByteSlice;
    let mut paths = std::collections::BTreeSet::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            paths.insert(gix::path::from_bstr(entry.path.as_bstr())?.to_path_buf());
        }
    }
    Ok(paths)
}

