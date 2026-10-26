use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefTab {
    #[default]
    Local,
    Remote,
    Stash,
    Tag,
}

impl RefTab {
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap()
    }
    pub const ALL: [Self; 4] = [Self::Local, Self::Remote, Self::Stash, Self::Tag];
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Remote => "Remote",
            Self::Stash => "Stash",
            Self::Tag => "Tag",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileTab {
    #[default]
    Stage,
    History,
}

#[derive(Clone, Debug)]
pub struct Reference {
    pub name: String,
    pub revision: String,
    pub kind: RefTab,
    pub target: String,
}

#[derive(Clone, Debug, Default)]
pub struct Commit {
    pub id: String,
    pub title: String,
    pub author: String,
    pub time: i64,
    pub parent_ids: Vec<String>,
    pub marker: GraphMarker,
    pub input_edges: Vec<GraphEdge>,
    pub output_edges: Vec<GraphEdge>,
    pub branches: Vec<String>,
    pub time_offset: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphMarker {
    pub lane: usize,
    pub color: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub waiting_commit_id: String,
    pub from_lane: usize,
    pub to_lane: usize,
    pub color: usize,
    pub tip: bool,
}

#[derive(Clone, Debug)]
pub struct CommitDetail {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub author: String,
    pub author_email: String,
    pub author_time: i64,
    pub author_offset: i32,
    pub committer: String,
    pub committer_email: String,
    pub commit_time: i64,
    pub commit_offset: i32,
    pub message: String,
}

pub struct CommitSelection {
    pub history: Vec<FileChange>,
    pub detail: CommitDetail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRow {
    Group(bool),
    File(usize),
}

pub fn file_rows(files: &[FileChange], groups: bool) -> Vec<FileRow> {
    let mut rows = Vec::with_capacity(files.len() + 2);
    let mut last = None;
    for (index, file) in files.iter().enumerate() {
        if groups && last != Some(file.staged) {
            rows.push(FileRow::Group(file.staged));
            last = Some(file.staged);
        }
        rows.push(FileRow::File(index));
    }
    rows
}

#[derive(Clone, Debug)]
pub enum Content {
    Empty,
    Object(gix::ObjectId, u16),
    Worktree,
}

#[derive(Clone, Debug)]
pub struct FileChange {
    pub path: Vec<u8>,
    pub label: String,
    pub status: String,
    pub staged: bool,
    pub before: Content,
    pub after: Content,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub root: PathBuf,
    pub head: String,
    pub references: Vec<Reference>,
    pub commits: Vec<Commit>,
    pub stage: Vec<FileChange>,
    pub history: Vec<FileChange>,
    pub commit_detail: Option<CommitDetail>,
    pub current_branch: Option<String>,
    pub reference_rows: [Vec<usize>; 4],
    pub session: Option<crate::paging::HistorySession>,
    pub has_more: bool,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub kind: char,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub title: String,
    pub note: String,
    pub lines: Vec<DiffLine>,
}

pub struct Project {
    pub id: u64,
    pub path: PathBuf,
    pub name: String,
    pub ref_tab: RefTab,
    pub file_tab: FileTab,
    pub revision: Option<String>,
    pub commit: Option<String>,
    pub selected_file: Option<usize>,
    pub show_commit_detail: bool,
    pub snapshot: Snapshot,
    pub diff: Diff,
    pub loading: bool,
    pub error: Option<String>,
    pub generation: u64,
    pub diff_generation: u64,
    pub diff_loading: bool,
    pub page_loading: bool,
    pub page_error: Option<String>,
    pub page_generation: u64,
}

impl Project {
    pub fn new(id: u64, path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        Self {
            id,
            path,
            name,
            ref_tab: RefTab::Local,
            file_tab: FileTab::Stage,
            revision: None,
            commit: None,
            selected_file: None,
            show_commit_detail: false,
            snapshot: Snapshot::default(),
            diff: Diff::default(),
            loading: false,
            error: None,
            generation: 0,
            diff_generation: 0,
            diff_loading: false,
            page_loading: false,
            page_error: None,
            page_generation: 0,
        }
    }

    pub fn files(&self) -> &[FileChange] {
        match self.file_tab {
            FileTab::Stage => &self.snapshot.stage,
            FileTab::History => &self.snapshot.history,
        }
    }
}
