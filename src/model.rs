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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRow {
    Group(bool),
    Folder { path: Vec<u8> },
    File(usize),
}

impl FileRow {
    pub fn file_index(&self) -> Option<usize> {
        match self {
            Self::File(index) => Some(*index),
            _ => None,
        }
    }
}

pub fn file_tree_rows(files: &[FileChange]) -> Vec<FileRow> {
    #[derive(Default)]
    struct Folder {
        children: BTreeMap<Vec<u8>, Folder>,
        files: Vec<usize>,
    }
    fn flatten(folder: &Folder, path: &[u8], rows: &mut Vec<FileRow>) {
        for (name, child) in &folder.children {
            let mut path = path.to_vec();
            if !path.is_empty() {
                path.push(b'/');
            }
            path.extend(name);
            rows.push(FileRow::Folder { path: path.clone() });
            flatten(child, &path, rows);
        }
        rows.extend(folder.files.iter().copied().map(FileRow::File));
    }
    let mut root = Folder::default();
    let mut order: Vec<_> = (0..files.len()).collect();
    order.sort_by(|a, b| files[*a].path.cmp(&files[*b].path));
    for index in order {
        let mut folder = &mut root;
        let parts: Vec<_> = files[index].path.split(|byte| *byte == b'/').collect();
        for part in &parts[..parts.len().saturating_sub(1)] {
            folder = folder.children.entry(part.to_vec()).or_default();
        }
        folder.files.push(index);
    }
    let mut rows = Vec::new();
    flatten(&root, &[], &mut rows);
    rows
}

pub fn directory_stage_changes(files: &[FileChange], directory: &[u8]) -> BTreeMap<Vec<u8>, bool> {
    let mut prefix = directory.to_vec();
    prefix.push(b'/');
    let stage = files
        .iter()
        .any(|file| file.path.starts_with(&prefix) && !file.staged);
    files
        .iter()
        .filter(|file| file.path.starts_with(&prefix))
        .map(|file| (file.path.clone(), stage))
        .collect()
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
        self.click_visible(
            files,
            &(0..files.len()).collect::<Vec<_>>(),
            index,
            extend,
            control,
        )
    }
    pub fn click_visible(
        &mut self,
        files: &[FileChange],
        order: &[usize],
        index: usize,
        extend: bool,
        control: bool,
    ) -> Option<usize> {
        let file = files.get(index)?;
        let key = StageKey::of(file);
        if extend {
            let position = order.iter().position(|item| *item == index)?;
            let anchor = self
                .anchor
                .as_ref()
                .and_then(|anchor| {
                    order
                        .iter()
                        .position(|item| StageKey::of(&files[*item]) == *anchor)
                })
                .unwrap_or(position);
            if !control {
                self.selected.clear();
            }
            self.selected.extend(
                order[anchor.min(position)..=anchor.max(position)]
                    .iter()
                    .map(|index| StageKey::of(&files[*index])),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeState {
    pub root: PathBuf,
    pub marker_modified: std::time::SystemTime,
    pub head: String,
    pub current_branch: Option<String>,
    pub merge_heads: Vec<String>,
    pub conflicts: Vec<Vec<u8>>,
}

impl MergeState {
    pub fn same_operation(&self, other: &Self) -> bool {
        self.root == other.root
            && self.marker_modified == other.marker_modified
            && self.head == other.head
            && self.current_branch == other.current_branch
            && self.merge_heads == other.merge_heads
    }
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub stash_restore: Option<StashRestoreState>,
    pub unmerged: Vec<Vec<u8>>,
    pub revert: Option<RevertState>,
    pub rebase: Option<RebaseState>,
    pub merge: Option<MergeState>,
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

impl Snapshot {
    pub fn integration_active(&self) -> bool {
        self.merge.is_some()
            || self.rebase.is_some()
            || self.revert.is_some()
            || self.stash_restore.is_some()
            || !self.unmerged.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StashRestoreState {
    pub root: PathBuf,
    pub id: String,
    pub stash: String,
    pub head: String,
    pub current_branch: Option<String>,
    pub revision: u64,
    pub applying: bool,
    pub error: Option<String>,
    pub conflicts: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebaseState {
    pub root: PathBuf,
    pub backend: String,
    pub marker_modified: std::time::SystemTime,
    pub original_head: String,
    pub branch: String,
    pub branch_tip: Option<String>,
    pub onto: String,
    pub head: String,
    pub current_branch: Option<String>,
    pub current_commit: Option<String>,
    pub step: Option<usize>,
    pub total: Option<usize>,
    pub conflicts: Vec<Vec<u8>>,
}

impl RebaseState {
    pub fn same_session(&self, other: &Self) -> bool {
        self.root == other.root
            && self.backend == other.backend
            && self.marker_modified == other.marker_modified
            && self.original_head == other.original_head
            && self.branch == other.branch
            && self.branch_tip == other.branch_tip
            && self.onto == other.onto
    }
    pub fn same_operation(&self, other: &Self) -> bool {
        self.same_session(other)
            && self.head == other.head
            && self.current_branch == other.current_branch
            && self.current_commit == other.current_commit
            && self.step == other.step
            && self.total == other.total
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevertState {
    pub root: PathBuf,
    pub head: String,
    pub current_branch: Option<String>,
    pub sequence_head: Option<String>,
    pub session_modified: std::time::SystemTime,
    pub target: String,
    pub target_modified: Option<std::time::SystemTime>,
    pub todo: String,
    pub pending_count: usize,
    pub options: String,
    pub no_commit: Option<bool>,
    pub conflicts: Vec<Vec<u8>>,
}

impl RevertState {
    pub fn same_session(&self, other: &Self) -> bool {
        self.root == other.root
            && self.current_branch == other.current_branch
            && self.sequence_head == other.sequence_head
            && self.session_modified == other.session_modified
            && self.options == other.options
    }
    pub fn same_operation(&self, other: &Self) -> bool {
        self.same_session(other)
            && self.head == other.head
            && self.target == other.target
            && self.target_modified == other.target_modified
            && self.todo == other.todo
            && self.pending_count == other.pending_count
            && self.no_commit == other.no_commit
    }
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
    pub conflict: Option<crate::conflict::Preview>,
    pub partial: Option<crate::partial_stage::Preview>,
}

impl Diff {
    pub fn conflict_at_row(&self, row: usize) -> Option<usize> {
        self.conflict
            .as_ref()?
            .region_for_line(self.lines.get(row)?.new?)
    }
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
    fn tree_rows_preserve_original_indices_duplicates_and_nested_paths() {
        let files = [
            file("z.txt", false),
            file("소스/sub/b.rs", false),
            file("소스/a.rs", true),
            file("소스/a.rs", false),
        ];
        let rows = file_tree_rows(&files);
        assert_eq!(
            rows,
            vec![
                FileRow::Folder {
                    path: "소스".as_bytes().to_vec(),
                },
                FileRow::Folder {
                    path: "소스/sub".as_bytes().to_vec(),
                },
                FileRow::File(1),
                FileRow::File(2),
                FileRow::File(3),
                FileRow::File(0),
            ]
        );
        assert_eq!(file_tree_rows(&files), rows);
        assert!(file_tree_rows(&[]).is_empty());
    }

    #[test]
    fn tree_uses_raw_paths_even_when_display_labels_collide() {
        let mut files = [file("a/file", false), file("b/file", false)];
        files[0].path = vec![0xfe, b'/', b'x'];
        files[1].path = vec![0xff, b'/', b'x'];
        files[0].label = "same".into();
        files[1].label = "same".into();
        let rows = file_tree_rows(&files);
        assert_eq!(
            rows.iter()
                .filter_map(FileRow::file_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            file_tree_rows(&files)
                .iter()
                .filter_map(FileRow::file_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn directory_stage_targets_all_descendants_and_deduplicates_partial_changes() {
        let files = [
            file("src/a", true),
            file("src/a", false),
            file("src/deep/b", true),
            file("src2/c", false),
            file("src", false),
            file("root", false),
        ];
        assert_eq!(
            directory_stage_changes(&files, b"src"),
            BTreeMap::from([(b"src/a".to_vec(), true), (b"src/deep/b".to_vec(), true)])
        );
        assert_eq!(
            directory_stage_changes(&[file("src/a", true), file("src/deep/b", true)], b"src"),
            BTreeMap::from([(b"src/a".to_vec(), false), (b"src/deep/b".to_vec(), false)])
        );
        assert!(directory_stage_changes(&files, b"missing").is_empty());
        assert!(directory_stage_changes(&[], b"src").is_empty());
    }

    #[test]
    fn visible_selection_follows_tree_order_and_excludes_hidden_files() {
        let files = [
            file("root", false),
            file("dir/a", true),
            file("hidden/x", false),
            file("dir/b", false),
        ];
        let mut selection = StageSelection::default();
        let order = [1, 3, 0];
        selection.click_visible(&files, &order, 1, false, false);
        selection.click_visible(&files, &order, 0, true, false);
        assert!(
            selection.contains(&files[0])
                && selection.contains(&files[1])
                && selection.contains(&files[3])
        );
        assert!(!selection.contains(&files[2]));
        assert_eq!(selection.changes(&files).len(), 3);
        selection.select_all(&files);
        assert!(selection.contains(&files[2]));
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
