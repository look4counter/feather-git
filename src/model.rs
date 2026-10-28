use std::collections::{BTreeMap, BTreeSet};
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

impl Reference {
    pub fn is_remote_head(&self) -> bool {
        self.kind == RefTab::Remote && self.revision.ends_with("/HEAD")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Commit {
    pub id: String,
    pub title: String,
    pub author: String,
    pub author_email: String,
    pub time: i64,
    pub parent_ids: Vec<String>,
    pub marker: GraphMarker,
    pub input_edges: Vec<GraphEdge>,
    pub output_edges: Vec<GraphEdge>,
    pub branches: Vec<String>,
    pub tags: Vec<String>,
    pub time_offset: i32,
}

impl Commit {
    pub fn author_label(&self) -> String {
        format!("{} <{}>", self.author, self.author_email)
    }
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
        if groups && file.staged && last != Some(file.staged) {
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

impl FileChange {
    pub fn stage_status(&self) -> String {
        if self.status == "U" {
            return "UU".into();
        }
        if self.staged {
            format!("{} ", self.status)
        } else if self.status == "A" {
            "??".into()
        } else {
            format!(" {}", self.status)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StageKey {
    path: Vec<u8>,
    staged: bool,
}

impl StageKey {
    fn of(file: &FileChange) -> Self {
        Self {
            path: file.path.clone(),
            staged: file.staged,
        }
    }
}

#[derive(Default)]
pub struct StageSelection {
    selected: BTreeSet<StageKey>,
    anchor: Option<StageKey>,
}

impl StageSelection {
    pub fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }
    pub fn contains(&self, file: &FileChange) -> bool {
        self.selected.contains(&StageKey::of(file))
    }
    pub fn select_all(&mut self, files: &[FileChange]) {
        self.selected = files.iter().map(StageKey::of).collect();
        if !self
            .anchor
            .as_ref()
            .is_some_and(|anchor| self.selected.contains(anchor))
        {
            self.anchor = files.first().map(StageKey::of);
        }
    }
    pub fn click(
        &mut self,
        files: &[FileChange],
        index: usize,
        extend: bool,
        control: bool,
    ) -> Option<usize> {
        let file = files.get(index)?;
        let key = StageKey::of(file);
        if extend {
            let anchor = self
                .anchor
                .as_ref()
                .and_then(|anchor| files.iter().position(|file| StageKey::of(file) == *anchor))
                .unwrap_or(index);
            if !control {
                self.selected.clear();
            }
            self.selected.extend(
                files[anchor.min(index)..=anchor.max(index)]
                    .iter()
                    .map(StageKey::of),
            );
            if self.anchor.is_none() {
                self.anchor = Some(key);
            }
        } else {
            if !control {
                self.selected.clear();
                self.selected.insert(key.clone());
            } else if !self.selected.remove(&key) {
                self.selected.insert(key.clone());
            }
            self.anchor = Some(key);
        }
        if self.contains(file) {
            Some(index)
        } else {
            files.iter().position(|file| self.contains(file))
        }
    }
    pub fn changes(&self, files: &[FileChange]) -> BTreeMap<Vec<u8>, bool> {
        let mut changes = BTreeMap::new();
        for file in files.iter().filter(|file| self.contains(file)) {
            changes
                .entry(file.path.clone())
                .and_modify(|stage| *stage |= !file.staged)
                .or_insert(!file.staged);
        }
        changes
    }
    pub fn restore(&mut self, files: &[FileChange], changes: &BTreeMap<Vec<u8>, bool>) {
        let remap = |key: &StageKey| {
            let staged = changes.get(&key.path).copied().unwrap_or(key.staged);
            files
                .iter()
                .find(|file| file.path == key.path && file.staged == staged)
                .or_else(|| files.iter().find(|file| file.path == key.path))
                .map(StageKey::of)
        };
        self.selected = self.selected.iter().filter_map(remap).collect();
        self.anchor = self.anchor.as_ref().and_then(remap);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub root: PathBuf,
    pub head: String,
    pub head_commit: Option<String>,
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
    pub initialized: bool,
    pub id: u64,
    pub path: PathBuf,
    pub name: String,
    pub ref_tab: RefTab,
    pub file_tab: FileTab,
    pub revision: Option<String>,
    pub commit: Option<String>,
    pub selected_file: Option<usize>,
    pub stage_selection: StageSelection,
    pub show_commit_detail: bool,
    pub snapshot: Snapshot,
    pub diff: Diff,
    pub loading: bool,
    pub history_loading: bool,
    pub history_target: Option<String>,
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
            initialized: false,
            id,
            path,
            name,
            ref_tab: RefTab::Local,
            file_tab: FileTab::Stage,
            revision: None,
            commit: None,
            selected_file: None,
            stage_selection: StageSelection::default(),
            show_commit_detail: false,
            snapshot: Snapshot::default(),
            diff: Diff::default(),
            loading: false,
            history_loading: false,
            history_target: None,
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

    pub fn resolve_history_target(&mut self) -> Option<usize> {
        let target = self.history_target.as_ref()?;
        if let Some(index) = self
            .snapshot
            .commits
            .iter()
            .position(|commit| &commit.id == target)
        {
            self.history_target = None;
            return Some(index);
        }
        if !self.snapshot.has_more {
            self.page_error = Some(
                "선택한 브랜치의 커밋을 히스토리에서 찾지 못했습니다. 새로 조회해 주세요.".into(),
            );
            self.history_target = None;
        }
        None
    }

    pub fn history_in_progress(&self) -> bool {
        self.history_loading || self.page_loading
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    fn file(path: &str, staged: bool) -> FileChange {
        FileChange {
            path: path.as_bytes().to_vec(),
            label: path.into(),
            status: "M".into(),
            staged,
            before: Content::Empty,
            after: Content::Worktree,
        }
    }
    #[test]
    fn ranges_control_toggles_and_restoration_follow_file_identity() {
        let files = [
            file("a", false),
            file("b", false),
            file("c", false),
            file("d", false),
            file("e", false),
        ];
        let mut selection = StageSelection::default();
        selection.click(&files, 1, false, false);
        selection.click(&files, 3, true, false);
        assert_eq!(
            selection
                .changes(&files)
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            [b"b".to_vec(), b"c".to_vec(), b"d".to_vec()]
        );
        selection.click(&files, 0, true, false);
        assert!(selection.contains(&files[0]) && selection.contains(&files[1]));
        assert!(!selection.contains(&files[2]));
        selection.click(&files, 4, false, true);
        selection.click(&files, 1, false, true);
        assert!(!selection.contains(&files[1]));
        selection.click(&files, 2, true, true);
        let targets = selection.changes(&files);
        assert_eq!(targets.len(), 4);
        let updated = [
            file("a", true),
            file("b", true),
            file("c", true),
            file("d", false),
            file("e", true),
        ];
        selection.restore(&updated, &targets);
        assert!(selection.contains(&updated[0]) && selection.contains(&updated[4]));
        assert!(!selection.contains(&updated[3]));
        assert!(selection.changes(&updated).values().all(|staged| !staged));
        selection.restore(&updated[..4], &Default::default());
        assert_eq!(selection.changes(&updated).len(), 3);
        selection.clear();
        assert!(selection.changes(&updated).is_empty());
    }
    #[test]
    fn selecting_both_states_of_a_path_stages_it_once() {
        let files = [file("partial", true), file("partial", false)];
        let mut selection = StageSelection::default();
        selection.click(&files, 0, false, false);
        assert_eq!(
            selection.changes(&files).get(b"partial".as_slice()),
            Some(&false)
        );
        selection.click(&files, 1, true, false);
        assert_eq!(selection.changes(&files).len(), 1);
        assert_eq!(
            selection.changes(&files).get(b"partial".as_slice()),
            Some(&true)
        );
        let updated = [file("partial", true)];
        selection.restore(&updated, &selection.changes(&files));
        assert!(selection.contains(&updated[0]));
        selection.click(&updated, 0, false, true);
        assert!(selection.changes(&updated).is_empty());
    }
    #[test]
    fn select_all_includes_both_states_and_supports_deselection_ranges_and_empty_lists() {
        let files = [
            file("a", true),
            file("a", false),
            file("b", false),
            file("c", true),
        ];
        let mut selection = StageSelection::default();
        selection.click(&files, 1, false, false);
        selection.select_all(&files);
        assert!(files.iter().all(|file| selection.contains(file)));
        assert_eq!(
            selection.changes(&files),
            [
                (b"a".to_vec(), true),
                (b"b".to_vec(), true),
                (b"c".to_vec(), false)
            ]
            .into_iter()
            .collect()
        );
        selection.click(&files, 2, false, true);
        assert!(!selection.contains(&files[2]));
        selection.click(&files, 3, true, false);
        assert!(selection.contains(&files[2]) && selection.contains(&files[3]));
        assert!(!selection.contains(&files[0]));
        selection.select_all(&[]);
        assert!(selection.changes(&files).is_empty());
        selection.select_all(&files);
        let targets = selection.changes(&files);
        let updated = [file("a", true), file("b", true), file("c", false)];
        selection.restore(&updated, &targets);
        assert!(updated.iter().all(|file| selection.contains(file)));
    }
}
