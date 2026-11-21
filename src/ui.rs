use crate::alert::Alert;
use crate::diff_git;
use crate::diff_model::FileChange;
use crate::diff_viewer::{DiffViewer, ViewerChanged, ViewerDiscardRequest, ViewerTarget};
use crate::history::{HistoryState, PreparedHistory, PreparedHistoryRow};
use crate::history_grid::{HistoryColumns, MESSAGE_MIN_WIDTH};
use crate::loading::LOADING_DELAY;
use crate::path_display::display_path;
use crate::progress::{ProgressOverlay, StartupLoadingScreen};
#[cfg(test)]
use crate::repository::Commit;
use crate::repository::{self, ChangedFile, Operation, Project, ReferenceKind, Request};
use crate::repository_dialog::{DialogActivity, DialogEvent, RepositoryDialog};
use crate::session::{self, RepositoryEntry, Session};
use crate::theme::*;
use crate::virtual_list::{ListLoading, ListScrollHandle, SingleSelection, multi_virtual_list, panel_message};
use crate::workspace_backend::{
    self, PanelRequest, PanelResult, ReferenceRequest, RepositoryListRequest, RepositoryListResult,
    RestoredWorkspace, SessionWriter,
};
use gpui::{Context, Div, Window, div, prelude::*, px, relative, rgb};
use std::{cell::Cell, collections::HashMap, rc::Rc, time::Instant};
struct ReferenceDeletePrompt {
    project: usize,
    path: std::path::PathBuf,
    tab: RepositoryTab,
    index: usize,
    request: repository::ReferenceDeletion,
}

#[derive(Clone, Copy, Default)]
enum AbortOperation { #[default] Merge, Rebase, Revert }

#[derive(Clone)]
struct BranchCheckoutPrompt {
    project: usize,
    path: std::path::PathBuf,
    index: usize,
    commit: String,
    candidates: Vec<String>,
    selected: usize,
    scroll: ListScrollHandle,
}
#[derive(Clone)]
enum PendingGitAction {
    Toolbar(workspace_backend::ToolbarAction),
    Sync(repository::SyncMode),
}
#[derive(Clone)]
struct GitActionPrompt {
    project: usize,
    path: std::path::PathBuf,
    head: Option<String>,
    branch: String,
    target: Option<(usize, String)>,
    action: PendingGitAction,
}

struct CommitCompletion {
    commit_id: Option<String>,
    error: Option<String>,
}

const CUSTOM_TITLEBAR_HEIGHT: f32 = if cfg!(target_os = "windows") { 32. } else { 0. };

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum RepositoryTab {
    #[default]
    Local,
    Remote,
    Tags,
    Stashes,
    Search,
}
impl RepositoryTab {
    fn reference_name(self, name: &str) -> Option<String> {
        let prefix = match self {
            Self::Local => "refs/heads",
            Self::Remote => "refs/remotes",
            Self::Tags => "refs/tags",
            Self::Stashes | Self::Search => return None,
        };
        Some(format!("{prefix}/{name}"))
    }
}

#[derive(Clone, Copy)]
struct HistoryResize {
    project: usize,
    column: usize,
    start_x: f32,
    start_widths: [f32; 5],
}

#[derive(Clone, Copy)]
struct PanelResize {
    row: usize,
    project: usize,
    start_position: [f32; 2],
    start_split: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceScreen {
    Loading,
    Start,
    Project,
    Empty,
}

#[derive(Clone, Copy, Debug)]
struct FileMenuRequest {
    project: usize,
    revision: u64,
    historical: bool,
    row: usize,
}

struct DiscardPrompt {
    project: usize,
    root: std::path::PathBuf,
    preview: diff_git::DiscardPreview,
}

pub struct Workspace {
    repository_tab: RepositoryTab,
    file_history: bool,
    repository_selection: SingleSelection,
    file_selection: crate::virtual_list::MultiSelection,
    panel_references: Result<Vec<String>, String>,
    retained_repository_names: Option<Vec<String>>,
    retained_file_list: Option<(Vec<ChangedFile>, bool)>,
    repository_revision: u64,
    repository_pending: Option<RepositoryListRequest>,
    repository_reads: crate::backend::ReadSlot,
    repository_waiting: bool,
    repository_loading: ListLoading,
    history_files: Result<Vec<ChangedFile>, String>,
    viewer_files: Result<Vec<FileChange>, String>,
    viewer: Option<gpui::Entity<DiffViewer>>,
    viewer_refresh_pending: bool,
    viewer_subscription: Option<gpui::Subscription>,
    file_history_overlay: Option<gpui::Entity<crate::file_history_dialog::FileHistoryDialog>>,
    file_history_subscription: Option<gpui::Subscription>,
    file_history_previous_focus: Option<gpui::FocusHandle>,
    panel_focus: Option<[gpui::FocusHandle; 3]>,
    panel_focus_subscriptions: Vec<gpui::Subscription>,
    viewer_observer: Option<gpui::Subscription>,
    settings_panel: Option<gpui::Entity<crate::settings::SettingsPanel>>,
    settings_subscription: Option<gpui::Subscription>,
    settings_focus: Option<gpui::FocusHandle>,
    settings_previous_focus: Option<gpui::FocusHandle>,
    search_panel: Option<gpui::Entity<crate::search_panel::SearchPanel>>,
    search_subscription: Option<gpui::Subscription>,
    search_observer: Option<gpui::Subscription>,
    search_owner: Option<usize>,
    git_action_dialog: Option<gpui::Entity<crate::git_action_dialog::GitActionDialog>>,
    git_action_subscription: Option<gpui::Subscription>,
    git_action_previous_focus: Option<gpui::FocusHandle>,
    git_action_prompt: Option<GitActionPrompt>,
    encoding_dialog: Option<gpui::Entity<crate::encoding_dialog::EncodingDialog>>,
    encoding_subscription: Option<gpui::Subscription>,
    encoding_previous_focus: Option<gpui::FocusHandle>,
    branch_dialog: Option<gpui::Entity<crate::branch_dialog::BranchDialog>>,
    branch_subscription: Option<gpui::Subscription>,
    branch_previous_focus: Option<gpui::FocusHandle>,
    branch_owner: Option<(usize, std::path::PathBuf)>,
    branch_remote: Option<String>,
    tag_dialog_commit: Option<String>,
    stash_dialog: bool,
    commit_dialog: Option<gpui::Entity<crate::commit_dialog::CommitDialog>>,
    commit_subscription: Option<gpui::Subscription>,
    commit_previous_focus: Option<gpui::FocusHandle>,
    commit_owner: Option<(usize, std::path::PathBuf)>,
    commit_busy: bool,
    commit_revision: u64,
    selected_sync_mode: repository::SyncMode,
    sync_busy: bool,
    sync_operation: Option<&'static str>,
    sync_revision: u64,
    reference_delete: Option<ReferenceDeletePrompt>,
    reference_delete_focus: Option<[gpui::FocusHandle; 4]>,
    reference_delete_remote_tag: bool,
    reference_delete_previous_focus: Option<gpui::FocusHandle>,
    reference_delete_revision: u64,
    stash_apply_revision: u64,
    abort_operation: AbortOperation,
    merge_abort_confirmation: Option<(usize, std::path::PathBuf, String)>,
    merge_abort_focus: Option<gpui::FocusHandle>,
    merge_abort_buttons: Option<[gpui::FocusHandle; 2]>,
    merge_abort_previous_focus: Option<gpui::FocusHandle>,
    checkout_branch_prompt: Option<BranchCheckoutPrompt>,
    checkout_branch_focus: Option<gpui::FocusHandle>,
    checkout_branch_controls: Option<[gpui::FocusHandle; 3]>,
    checkout_branch_previous_focus: Option<gpui::FocusHandle>,
    checkout_confirmation: Option<(usize, std::path::PathBuf, String)>,
    checkout_focus: Option<gpui::FocusHandle>,
    checkout_buttons: Option<[gpui::FocusHandle; 2]>,
    checkout_previous_focus: Option<gpui::FocusHandle>,
    recent_selection: Option<usize>,
    recent_focus: Option<gpui::FocusHandle>,
    recent_scroll: gpui::ScrollHandle,
    footer_action_busy: bool,
    file_stage_busy: bool,
    discard_prompt: Option<DiscardPrompt>,
    line_discard: Option<ViewerDiscardRequest>,
    line_discard_subscription: Option<gpui::Subscription>,
    discard_focus: Option<gpui::FocusHandle>,
    discard_previous_focus: Option<gpui::FocusHandle>,
    projects: Vec<Project>,
    session_path: Option<std::path::PathBuf>,
    recent_repositories: Vec<RepositoryEntry>,
    restore_warning: Option<String>,
    save_error: Option<String>,
    repository_operation: Option<Operation>,
    recent_removal: Option<RepositoryEntry>,
    removal_focus: Option<gpui::FocusHandle>,
    removal_hold: crate::recent_repository_dialog::HoldToDelete,
    removal_observers: Vec<gpui::Subscription>,
    removal_previous_focus: Option<gpui::FocusHandle>,
    deleting_recent: Option<RepositoryEntry>,
    progress_focus: Option<gpui::FocusHandle>,
    delayed_progress: Option<(Instant, gpui::Task<()>)>,
    progress_previous_focus: Option<gpui::FocusHandle>,
    restoring: bool,
    panel_revision: u64,
    panel_pending: Option<PanelRequest>,
    panel_reads: crate::backend::ReadSlot,
    panel_waiting: bool,
    panel_loading: ListLoading,
    reference_revision: u64,
    reference_pending: Option<ReferenceRequest>,
    reference_reads: crate::backend::ReadSlot,
    commit_loading: ListLoading,
    save_revision: u64,
    save_pending: Option<(u64, std::sync::mpsc::Receiver<Result<(), String>>)>,
    session_writer: std::sync::Arc<SessionWriter>,
    repository_dialog: Option<gpui::Entity<RepositoryDialog>>,
    dialog_subscription: Option<gpui::Subscription>,
    dialog_previous_focus: Option<gpui::FocusHandle>,
    histories: HashMap<usize, HistoryState>,
    history_append_pending: Option<crate::workspace_backend::HistoryPageRequest>,
    history_append_inflight: Option<crate::workspace_backend::HistoryPageRequest>,
    history_append_reads: crate::backend::ReadSlot,
    panel_scrolls: HashMap<(&'static str, usize, usize), ListScrollHandle>,
    history_columns: HistoryColumns,
    history_resize: Option<HistoryResize>,
    panel_layout: crate::panel_layout::PanelLayout,
    panel_resize: Cell<Option<PanelResize>>,
    panel_width: Cell<f32>,
    panel_height: Cell<f32>,
    _panel_activation: Option<gpui::Subscription>,
    project: usize,
    open_projects: Vec<usize>,
    start_tab_open: bool,
    start_tab_active: bool,
    commit: usize,
    notice: String,
    alert: Option<Alert>,
    alert_focus: Option<gpui::FocusHandle>,
    previous_focus: Option<gpui::FocusHandle>,
}
impl Default for Workspace {
    fn default() -> Self {
        let mut workspace = Self {
            repository_tab: RepositoryTab::default(),
            file_history: false,
            repository_selection: SingleSelection::default(),
            file_selection: crate::virtual_list::MultiSelection::default(),
            panel_references: Ok(Vec::new()),
            retained_repository_names: None,
            retained_file_list: None,
            repository_revision: 0,
            repository_pending: None,
            repository_reads: crate::backend::ReadSlot::default(),
            repository_waiting: false,
            repository_loading: ListLoading::default(),
            history_files: Ok(Vec::new()),
            viewer_files: Ok(Vec::new()),
            viewer: None,
            viewer_refresh_pending: false,
            viewer_subscription: None,
            file_history_overlay: None,
            file_history_subscription: None,
            file_history_previous_focus: None,
            panel_focus: None,
            panel_focus_subscriptions: Vec::new(),
            viewer_observer: None,
            settings_panel: None,
            settings_subscription: None,
            settings_focus: None,
            settings_previous_focus: None,
            search_panel: None,
            search_subscription: None,
            search_observer: None,
            search_owner: None,
            git_action_dialog: None,
            git_action_subscription: None,
            git_action_previous_focus: None,
            git_action_prompt: None,
            encoding_dialog: None,
            encoding_subscription: None,
            encoding_previous_focus: None,
            branch_dialog: None,
            branch_subscription: None,
            branch_previous_focus: None,
            branch_owner: None,
            branch_remote: None,
            tag_dialog_commit: None,
            stash_dialog: false,
            commit_dialog: None,
            commit_subscription: None,
            commit_previous_focus: None,
            commit_owner: None,
            commit_busy: false,
            commit_revision: 0,
            selected_sync_mode: repository::SyncMode::Merge,
            sync_busy: false,
            sync_operation: None,
            sync_revision: 0,
            reference_delete: None,
            reference_delete_focus: None,
            reference_delete_remote_tag: false,
            reference_delete_previous_focus: None,
            reference_delete_revision: 0,
            stash_apply_revision: 0,
            abort_operation: AbortOperation::Merge,
            merge_abort_confirmation: None,
            merge_abort_focus: None,
            merge_abort_buttons: None,
            merge_abort_previous_focus: None,
            checkout_branch_prompt: None,
            checkout_branch_focus: None,
            checkout_branch_controls: None,
            checkout_branch_previous_focus: None,
            checkout_confirmation: None,
            checkout_focus: None,
            checkout_buttons: None,
            checkout_previous_focus: None,
            recent_selection: None,
            recent_focus: None,
            recent_scroll: gpui::ScrollHandle::new(),
            footer_action_busy: false,
            file_stage_busy: false,
            discard_prompt: None,
            line_discard: None,
            line_discard_subscription: None,
            discard_focus: None,
            discard_previous_focus: None,
            projects: Vec::new(),
            session_path: None,
            recent_repositories: Vec::new(),
            restore_warning: None,
            save_error: None,
            repository_operation: None,
            recent_removal: None,
            removal_focus: None,
            removal_hold: Default::default(),
            removal_observers: Vec::new(),
            removal_previous_focus: None,
            deleting_recent: None,
            progress_focus: None,
            delayed_progress: None,
            progress_previous_focus: None,
            restoring: false,
            panel_revision: 0,
            panel_pending: None,
            panel_reads: crate::backend::ReadSlot::default(),
            panel_waiting: false,
            panel_loading: ListLoading::default(),
            reference_revision: 0,
            reference_pending: None,
            reference_reads: crate::backend::ReadSlot::default(),
            commit_loading: ListLoading::default(),
            save_revision: 0,
            save_pending: None,
            session_writer: std::sync::Arc::new(SessionWriter::default()),
            repository_dialog: None,
            dialog_subscription: None,
            dialog_previous_focus: None,
            histories: HashMap::new(),
            history_append_pending: None,
            history_append_inflight: None,
            history_append_reads: crate::backend::ReadSlot::default(),
            panel_scrolls: HashMap::new(),
            history_columns: HistoryColumns::default(),
            history_resize: None,
            panel_layout: Default::default(),
            panel_resize: Cell::new(None),
            panel_width: Cell::new(0.),
            panel_height: Cell::new(0.),
            _panel_activation: None,
            project: 0,
            open_projects: Vec::new(),
            start_tab_open: true,
            start_tab_active: true,
            commit: 0,
            notice: String::new(),
            alert: None,
            alert_focus: None,
            previous_focus: None,
        };
        workspace.update_tab_notice();
        workspace
    }
}
fn discard_cancel_key(key: &str) -> bool {
    key == "escape"
}

fn copy_paths_shortcut(key: &gpui::Keystroke) -> bool {
    key.key == "c"
        && key.modifiers.control
        && key.modifiers.shift
        && !key.modifiers.alt
        && !key.modifiers.platform
        && !key.modifiers.function
}

fn select_all_files_shortcut(key: &gpui::Keystroke) -> bool {
    key.key == "a"
        && key.modifiers.control
        && !key.modifiers.shift
        && !key.modifiers.alt
        && !key.modifiers.platform
        && !key.modifiers.function
}

fn discard_shortcut(key: &gpui::Keystroke) -> bool {
    key.key == "d"
        && key.modifiers.control
        && !key.modifiers.shift
        && !key.modifiers.alt
        && !key.modifiers.platform
        && !key.modifiers.function
}

fn local_refresh_shortcut(key: &gpui::Keystroke) -> bool {
    key.key == "f5"
        && !key.modifiers.shift
        && !key.modifiers.control
        && !key.modifiers.alt
        && !key.modifiers.platform
        && !key.modifiers.function
}

fn plain_space(key: &gpui::Keystroke) -> bool {
    key.key == "space"
        && !key.modifiers.shift
        && !key.modifiers.control
        && !key.modifiers.alt
        && !key.modifiers.platform
        && !key.modifiers.function
}

fn row() -> Div {
    div().flex().items_center()
}
fn label(text: impl Into<gpui::SharedString>) -> Div {
    div().text_color(rgb(MUTED)).child(text.into())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReferenceIcon {
    Computer,
    Glyph(&'static str),
}

fn reference_icon(kind: ReferenceKind) -> Option<ReferenceIcon> {
    match kind {
        ReferenceKind::Local => Some(ReferenceIcon::Computer),
        ReferenceKind::Remote => Some(ReferenceIcon::Glyph("\u{2601}\u{fe0e}")),
        ReferenceKind::Tag => Some(ReferenceIcon::Glyph("\u{1f3f7}\u{fe0e}")),
        ReferenceKind::Other => None,
    }
}

fn computer_icon(color: u32) -> Div {
    div()
        .relative()
        .size(px(16.))
        .flex_shrink_0()
        .child(
            div()
                .absolute()
                .left(px(1.))
                .top(px(1.))
                .w(px(14.))
                .h(px(10.))
                .border_2()
                .border_color(rgb(color))
                .rounded(px(2.)),
        )
        .child(
            div()
                .absolute()
                .left(px(7.))
                .top(px(10.))
                .w(px(2.))
                .h(px(4.))
                .bg(rgb(color)),
        )
        .child(
            div()
                .absolute()
                .left(px(4.))
                .top(px(13.))
                .w(px(8.))
                .h(px(2.))
                .rounded_full()
                .bg(rgb(color)),
        )
}

fn reference_icon_view(kind: ReferenceKind, color: u32) -> Option<Div> {
    reference_icon(kind).map(|icon| match icon {
        ReferenceIcon::Computer => computer_icon(color),
        ReferenceIcon::Glyph(glyph) => div()
            .w(px(16.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(color))
            .child(glyph),
    })
}

fn badge(text: impl Into<gpui::SharedString>, color: u32) -> Div {
    badge_base(color).child(text.into())
}

fn badge_base(color: u32) -> Div {
    div()
        .px_2()
        .py(px(2.))
        .rounded_md()
        .bg(rgb(RAISED))
        .text_size(px(11.))
        .text_color(rgb(color))
}
fn reference_badge(kind: ReferenceKind, text: impl Into<gpui::SharedString>, color: u32) -> Div {
    badge_base(color)
        .flex()
        .items_center()
        .gap(px(4.))
        .when_some(
            reference_icon_view(kind, reference_badge_icon_color(kind, color)),
            |el, icon| el.child(icon),
        )
        .child(div().min_w_0().truncate().child(text.into()))
        .max_w(px(160.))
        .flex_shrink_0()
}

fn start_action_icon(index: usize) -> impl IntoElement {
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let strokes: &[&[(f32, f32)]] = match index {
                0 => &[&[(8., 3.), (8., 13.)], &[(3., 8.), (13., 8.)]],
                1 => &[
                    &[
                        (2., 13.),
                        (2., 3.),
                        (6., 3.),
                        (8., 5.),
                        (13., 5.),
                        (13., 7.),
                    ],
                    &[(2., 13.), (5., 7.), (15., 7.), (12., 13.), (2., 13.)],
                ],
                _ => &[
                    &[(5., 5.), (13., 5.), (13., 14.), (5., 14.), (5., 5.)],
                    &[(3., 11.), (2., 11.), (2., 2.), (10., 2.), (10., 3.)],
                ],
            };
            let mut path = gpui::PathBuilder::stroke(px(1.5));
            for stroke in strokes {
                for (i, &(x, y)) in stroke.iter().enumerate() {
                    let point = bounds.origin + gpui::point(px(x), px(y));
                    if i == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, window.text_style().color);
            }
        },
    )
    .size(px(16.))
    .flex_shrink_0()
}
fn commit_icon(background: u32) -> Div {
    div()
        .relative()
        .size(px(12.))
        .flex_shrink_0()
        .child(
            div()
                .absolute()
                .left_0()
                .top(px(5.5))
                .w_full()
                .h(px(1.))
                .bg(rgb(MUTED)),
        )
        .child(
            div()
                .absolute()
                .left(px(3.))
                .top(px(3.))
                .size(px(6.))
                .rounded_full()
                .border_1()
                .border_color(rgb(MUTED))
                .bg(rgb(background)),
        )
}

fn history_cell(width: f32) -> Div {
    div()
        .flex()
        .items_center()
        .w(px(width))
        .h_full()
        .min_w_0()
        .flex_shrink_0()
        .px(px(10.))
        .overflow_hidden()
}

fn file_status_color(status: &str) -> u32 {
    match status.strip_prefix("S·").unwrap_or(status) {
        "A" => ACCENT,
        "D" => 0xffa7b2,
        _ => 0xe6bc73,
    }
}

fn heading(title: &'static str, count: impl Into<gpui::SharedString>) -> Div {
    row()
        .h(px(36.))
        .flex_shrink_0()
        .px_5()
        .gap_3()
        .border_b_1()
        .border_color(rgb(LINE))
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
        .child(badge(count, MUTED))
}
pub(crate) fn commit_grid_row(
    data: PreparedHistoryRow,
    columns: [f32; 5],
    lanes: usize,
    show_graph: bool,
) -> Div {
    let PreparedHistoryRow {
        index,
        graph,
        title,
        author,
        date,
        hash,
        reference,
        references,
        checked_out,
    } = data;
    row()
        .w_full()
        .h_full()
        .flex_shrink_0()
        .text_size(px(12.))
        .when(show_graph, |row| {
            row.child(
                div()
                    .w(px(columns[0]))
                    .h_full()
                    .flex_shrink_0()
                    .child(graph.render(checked_out, lanes)),
            )
        })
        .child(
            history_cell(columns[1])
                .flex_1()
                .w_auto()
                .min_w(px(MESSAGE_MIN_WIDTH))
                .gap_2()
                .when(!reference.is_empty() || !references.is_empty(), |el| {
                    el.children(references.iter().map(|reference| {
                        let color = match reference.kind {
                            ReferenceKind::Remote => 0xffa7b2,
                            ReferenceKind::Local => ACCENT,
                            ReferenceKind::Tag => 0xb9a2ef,
                            _ if index == 0 => ACCENT,
                            _ => 0xb9a2ef,
                        };
                        reference_badge(reference.kind, reference.label.clone(), color)
                    }))
                })
                .child(div().min_w_0().truncate().child(title)),
        )
        .child(history_cell(columns[2]).child(label(author).w_full().truncate().text_size(px(12.))))
        .child(history_cell(columns[3]).child(label(date).w_full().truncate().text_size(px(12.))))
        .child(
            history_cell(columns[4]).child(
                label(hash)
                    .w_full()
                    .truncate()
                    .font_family("Consolas")
                    .text_size(px(12.)),
            ),
        )
}

pub(crate) fn commit_grid_header(
    columns: [f32; 5],
    show_graph: bool,
    measured_widths: Rc<Cell<[f32; 5]>>,
    on_resize: impl Fn(usize, f32, &mut Window, &mut gpui::App) + 'static,
) -> Div {
    let on_resize = Rc::new(on_resize);
    row()
        .w_full()
        .h(px(crate::virtual_list::ROW_HEIGHT))
        .border_b_1()
        .border_color(rgb(LINE))
        .flex_shrink_0()
        .text_size(px(11.))
        .text_color(rgb(MUTED))
        .children(
            crate::history_grid::visible_columns(show_graph).map(move |column| {
                let record_widths = measured_widths.clone();
                let on_resize = on_resize.clone();
                div()
                    .relative()
                    .w(px(columns[column]))
                    .h_full()
                    .flex_shrink_0()
                    .when(column == 1, |cell| {
                        cell.flex_1().w_auto().min_w(px(MESSAGE_MIN_WIDTH))
                    })
                    .child(
                        gpui::canvas(
                            move |bounds, _, _| {
                                let mut widths = record_widths.get();
                                widths[column] = f32::from(bounds.size.width);
                                record_widths.set(widths);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(
                        history_cell(columns[column])
                            .w_full()
                            .when(column == 0, |cell| cell.pr_0().overflow_hidden())
                            .child(
                                div()
                                    .truncate()
                                    .child(crate::history_grid::COLUMN_TITLES[column]),
                            ),
                    )
                    .when(column < 4, |cell| {
                        cell.child(
                            div()
                                .id(("history-column-resize", column))
                                .absolute()
                                .when(column == 0, |handle| {
                                    handle.left(px((columns[0] - 8.).max(0.)))
                                })
                                .when(column != 0, |handle| handle.right_0())
                                .top_0()
                                .w(px(8.))
                                .h_full()
                                .border_r_1()
                                .border_color(rgb(LINE))
                                .cursor(gpui::CursorStyle::ResizeLeftRight)
                                .hover(|style| style.bg(rgb(LINE)))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    move |event, window, cx| {
                                        on_resize(column, event.position.x.into(), window, cx)
                                    },
                                ),
                        )
                    })
            }),
        )
}

impl Workspace {
    pub fn load(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (mut state, task) = Self::start_restore(|| {
            crate::backend::spawn(
                cx.background_executor(),
                move || match session::settings_path() {
                    Ok(path) => {
                        let result = RestoredWorkspace::read(&path);
                        (Some(path), result)
                    }
                    Err(error) => (None, Err(format!("{error:#}"))),
                },
            )
        });
        cx.spawn_in(window, async move |this, cx| {
            let (path, result) = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.finish_restore(path, result);
                cx.notify();
            });
        })
        .detach();
        state._panel_activation = Some(cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.panel_resize.take();
            }
            cx.notify();
        }));
        state.session_writer = workspace_backend::shared_writer();
        state
    }

    fn start_restore<T>(schedule: impl FnOnce() -> T) -> (Self, T) {
        let mut state = Self {
            restoring: true,
            ..Self::default()
        };
        state.update_tab_notice();
        (state, schedule())
    }

    fn finish_restore(
        &mut self,
        path: Option<std::path::PathBuf>,
        result: Result<RestoredWorkspace, String>,
    ) {
        self.restoring = false;
        self.session_path = path;
        match result {
            Ok(restored) => {
                self.selected_sync_mode = repository::SyncMode::from_option_key(&restored.saved.pull_mode);
                self.projects = restored.projects;
                self.open_projects = (0..self.projects.len()).collect();
                self.project = restored.project;
                self.commit = 0;
                self.recent_repositories = restored.saved.recent;
                self.restore_warning = restored.warning;
                self.start_tab_open = self.projects.is_empty()
                    || restored.saved.start_tab_open
                    || restored.saved.start_tab_active;
                self.start_tab_active = self.projects.is_empty() || restored.saved.start_tab_active;
                self.viewer_files = restored.viewer_files;
                self.history_files = Ok(Vec::new());
                self.panel_references = Ok(Vec::new());
                self.repository_selection.clear();
                self.file_selection.clear();
                self.histories.clear();
                if let Some(project) = self.projects.get_mut(self.project) {
                    project.files = self
                        .viewer_files
                        .as_ref()
                        .map(|files| {
                            files
                                .iter()
                                .map(|file| ChangedFile {
                                    status: if file.staged {
                                        format!("S·{}", file.status)
                                    } else {
                                        file.status.clone()
                                    },
                                    path: file.label.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    self.histories
                        .insert(self.project, HistoryState::from_prepared(restored.history));
                }
            }
            Err(error) => self.restore_warning = Some(error),
        }
        self.update_tab_notice();
    }

    #[cfg(test)]
    fn load_from_path(path: std::path::PathBuf) -> Self {
        let result = RestoredWorkspace::read(&path);
        let mut state = Self::default();
        state.finish_restore(Some(path), result);
        state
    }

    fn entry_request(entry: &RepositoryEntry) -> Request {
        Request {
            operation: Operation::Open,
            source: String::new(),
            name: entry.name.clone(),
            path: entry.path.to_string_lossy().into_owned(),
        }
    }

    fn session_snapshot(&self) -> Session {
        Session {
            pull_mode: self.selected_sync_mode.key().into(),
            open: self
                .open_projects
                .iter()
                .filter_map(|&index| {
                    let project = &self.projects[index];
                    Some(RepositoryEntry {
                        name: project.name.clone(),
                        path: project.path.clone()?,
                    })
                })
                .collect(),
            active: if self.open_projects.contains(&self.project) {
                self.projects[self.project].path.clone()
            } else {
                None
            },
            start_tab_open: self.start_tab_open,
            start_tab_active: self.start_tab_active,
            recent: self.recent_repositories.clone(),
            ..Session::default()
        }
    }

    fn save_session(&mut self) {
        if self.restoring {
            return;
        }
        if let Some(path) = &self.session_path {
            self.save_revision = self.save_revision.wrapping_add(1);
            let result = self
                .session_writer
                .enqueue(path.clone(), self.session_snapshot());
            self.save_pending = Some((self.save_revision, result));
        }
    }

    fn finish_save(
        &mut self,
        revision: u64,
        result: Result<Result<(), String>, std::sync::mpsc::RecvError>,
    ) {
        if revision == self.save_revision
            && let Ok(result) = result
        {
            self.save_error = result.err();
        }
    }

    fn remember_current_repository(&mut self) {
        if let Some(project) = self.projects.get(self.project)
            && let Some(path) = &project.path
        {
            session::remember_normalized(
                &mut self.recent_repositories,
                RepositoryEntry {
                    name: project.name.clone(),
                    path: path.clone(),
                },
            );
        }
    }

    fn session_warning(&self) -> Option<String> {
        let warnings: Vec<_> = self
            .restore_warning
            .iter()
            .chain(self.save_error.iter())
            .cloned()
            .collect();
        (!warnings.is_empty()).then(|| warnings.join(" / "))
    }

    fn global_progress_visible(&self) -> bool {
        self.repository_operation.is_some()
            || self.deleting_recent.is_some()
            || self.sync_busy
            || self.commit_busy
    }

    fn delayed_progress_pending(&self, cx: &gpui::App) -> bool {
        self.delayed_worktree_edit_pending(cx) || self.search_progress_pending(cx)
    }

    fn delayed_worktree_edit_pending(&self, cx: &gpui::App) -> bool {
        self.file_stage_busy || self.viewer.as_ref().is_some_and(|viewer| {
            viewer.read(cx).worktree_write_pending(cx)
        })
    }

    fn search_progress_pending(&self, cx: &gpui::App) -> bool {
        self.screen() == WorkspaceScreen::Project
                && self.repository_tab == RepositoryTab::Search
                && self.search_owner == Some(self.project)
                && self.search_panel.as_ref().is_some_and(|panel| {
                    let panel = panel.read(cx);
                    panel.search_pending()
                        && self.projects[self.project]
                            .path
                            .as_deref()
                            .is_some_and(|root| panel.is_for(root))
                })
    }

    fn update_delayed_progress(&mut self, cx: &mut Context<Self>) {
        self.track_delayed_progress(self.delayed_progress_pending(cx), Instant::now(), || {
            let timer = cx.background_executor().timer(LOADING_DELAY);
            cx.spawn(async move |this, cx| {
                timer.await;
                let _ = this.update(cx, |_, cx| cx.notify());
            })
        });
    }

    fn track_delayed_progress(
        &mut self,
        pending: bool,
        now: Instant,
        schedule: impl FnOnce() -> gpui::Task<()>,
    ) {
        if !pending {
            self.delayed_progress = None;
        } else if self.delayed_progress.is_none() {
            self.delayed_progress = Some((now, schedule()));
        }
    }

    fn delayed_progress_visible(&self, now: Instant) -> bool {
        self.delayed_progress
            .as_ref()
            .is_some_and(|(started, _)| now.saturating_duration_since(*started) >= LOADING_DELAY)
    }

    fn repository_busy(&self) -> bool {
        self.file_stage_busy || self.repository_busy_except_file_edit()
    }

    fn repository_busy_except_file_edit(&self) -> bool {
        self.git_action_prompt.is_some() || self.checkout_branch_prompt.is_some()
            || self.merge_abort_confirmation.is_some()
            || self.reference_delete.is_some()
            || self.checkout_confirmation.is_some()
            || self.commit_busy
            || self.commit_dialog.is_some()
            || self.branch_dialog.is_some()
            || self.sync_busy
            || self.discard_prompt.is_some()
            || self.line_discard.is_some()
            || self.file_history_overlay.is_some()
            || self.encoding_dialog.is_some()
            || self.settings_panel.is_some()
            || self.repository_operation.is_some()
            || self.deleting_recent.is_some()
            || self.recent_removal.is_some()
    }

    fn forget_recent(&mut self, entry: &RepositoryEntry) {
        self.recent_repositories
            .retain(|item| item.path != entry.path);
        self.save_session();
    }

    fn open_recent_removal(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.restoring
            || self.repository_busy()
            || self.repository_dialog.is_some()
            || self.alert.is_some()
        {
            return;
        }
        let Some(entry) = self.recent_repositories.get(index).cloned() else {
            return;
        };
        self.cancel_list_drags();
        self.history_resize = None;
        self.removal_hold.reset();
        self.recent_removal = Some(entry);
        self.removal_previous_focus = window.focused(cx);
        let focus = cx.focus_handle();
        focus.focus(window);
        self.removal_observers = vec![
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.removal_hold.reset();
                    cx.notify();
                }
            }),
            cx.on_blur(&focus, window, |this, _, cx| {
                this.removal_hold.reset();
                cx.notify();
            }),
        ];
        self.removal_focus = Some(focus);
        cx.notify();
    }

    fn finish_recent_deletion(&mut self, result: anyhow::Result<()>) -> Result<(), String> {
        let Some(entry) = self.deleting_recent.take() else {
            return Ok(());
        };
        result.map_err(|error| format!("{error:#}\n일부 파일은 이미 삭제되었을 수 있습니다."))?;
        let deleted_root = if entry.path.file_name().is_some_and(|name| name == ".git") {
            entry.path.parent().unwrap_or(&entry.path).to_path_buf()
        } else { entry.path.clone() };
        self.invalidate_requests();
        let affected: Vec<_> = self
            .open_projects
            .iter()
            .copied()
            .filter(|&id| {
                self.projects[id]
                    .path
                    .as_ref()
                    .is_some_and(|path| path.starts_with(&deleted_root))
            })
            .collect();
        for id in affected {
            self.close_project(id);
        }
        self.recent_repositories
            .retain(|item| !item.path.starts_with(&deleted_root));
        if self.open_projects.is_empty() {
            self.start_tab_open = true;
            self.start_tab_active = true;
            self.viewer = None;
            self.viewer_subscription = None;
        } else if !self.start_tab_active {
            self.prepare_history();
        }
        self.update_tab_notice();
        self.save_session();
        Ok(())
    }

    fn choose_recent_removal(
        &mut self,
        choice: crate::recent_repository_dialog::RemovalChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::recent_repository_dialog::RemovalChoice;
        let Some(entry) = self.recent_removal.take() else {
            return;
        };
        self.removal_hold.reset();
        self.removal_observers.clear();
        self.removal_focus = None;
        if let Some(previous) = self.removal_previous_focus.take() {
            previous.focus(window);
        } else {
            window.blur();
        }
        match choice {
            RemovalChoice::Cancel => {}
            RemovalChoice::EntryOnly => self.forget_recent(&entry),
            RemovalChoice::Files => {
                self.invalidate_requests();
                self.deleting_recent = Some(entry.clone());
                self.progress_previous_focus = window.focused(cx);
                let focus = cx.focus_handle();
                focus.focus(window);
                self.progress_focus = Some(focus);
                let task = crate::backend::spawn(cx.background_executor(), move || {
                    repository::delete_repository_files(&entry.path)
                });
                cx.spawn_in(window, async move |this, cx| {
                    let result = task.await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        this.progress_focus = None;
                        if let Some(previous) = this.progress_previous_focus.take() {
                            previous.focus(window);
                        } else {
                            window.blur();
                        }
                        if let Err(error) = this.finish_recent_deletion(result) {
                            this.show_alert("저장소 삭제 실패", &error, window, cx);
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    fn begin_repository_operation(&mut self, operation: Operation) -> bool {
        if self.restoring || self.repository_busy() || self.alert.is_some() {
            return false;
        }
        self.repository_operation = Some(operation);
        self.cancel_list_drags();
        self.history_resize = None;
        true
    }

    fn start_repository_operation(
        &mut self,
        request: Request,
        failure_title: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.begin_repository_operation(request.operation) {
            return;
        }
        self.progress_previous_focus = window.focused(cx);
        let focus = cx.focus_handle();
        focus.focus(window);
        self.progress_focus = Some(focus);
        cx.notify();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            repository::execute(request)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.progress_focus = None;
                if let Some(previous) = this.progress_previous_focus.take() {
                    previous.focus(window);
                } else {
                    window.blur();
                }
                if let Err(error) = this.finish_repository_operation(result) {
                    this.show_alert(failure_title, &error, window, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn finish_repository_operation(
        &mut self,
        result: anyhow::Result<Project>,
    ) -> Result<(), String> {
        self.repository_operation = None;
        let project = result.map_err(|error| format!("{error:#}"))?;
        self.activate_repository(project);
        Ok(())
    }

    fn open_recent_repository(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.restoring
            || self.repository_busy()
            || self.repository_dialog.is_some()
            || self.alert.is_some()
        {
            return;
        }
        let Some(entry) = self.recent_repositories.get(index) else {
            return;
        };
        let request = Self::entry_request(entry);
        self.start_repository_operation(request, "저장소 열기 실패", window, cx);
    }
    fn screen(&self) -> WorkspaceScreen {
        if self.restoring {
            WorkspaceScreen::Loading
        } else if self.start_tab_active {
            WorkspaceScreen::Start
        } else if self.open_projects.is_empty() {
            WorkspaceScreen::Empty
        } else {
            WorkspaceScreen::Project
        }
    }

    fn window_frame(&self, _window: &Window) -> Div {
        div()
            .relative()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BASE))
            .text_color(rgb(TEXT))
            .font_family("Segoe UI")
            .text_size(px(13.))
            .map(|el| {
                #[cfg(target_os = "windows")]
                let el = el.child(self.titlebar(_window));
                el
            })
    }

    #[cfg(target_os = "windows")]
    fn titlebar(&self, window: &Window) -> Div {
        use gpui::WindowControlArea;
        use std::sync::{Arc, LazyLock};

        static ICON: LazyLock<Arc<gpui::Image>> = LazyLock::new(|| {
            Arc::new(gpui::Image::from_bytes(
                gpui::ImageFormat::Png,
                include_bytes!("../design/favicon.png").to_vec(),
            ))
        });

        row()
            .h(px(CUSTOM_TITLEBAR_HEIGHT))
            .flex_shrink_0()
            .bg(rgb(BASE))
            .child(
                row()
                    .h_full()
                    .flex_1()
                    .pl_2()
                    .pr_4()
                    .gap_2()
                    .window_control_area(WindowControlArea::Drag)
                    .child(gpui::img(Arc::clone(&ICON)).size(px(18.)).flex_shrink_0())
                    .child(label("Feather Git").text_size(px(11.))),
            )
            .children(
                [
                    ("minimize", "\u{E921}", WindowControlArea::Min),
                    (
                        "maximize",
                        if window.is_maximized() {
                            "\u{E923}"
                        } else {
                            "\u{E922}"
                        },
                        WindowControlArea::Max,
                    ),
                    ("close", "\u{E8BB}", WindowControlArea::Close),
                ]
                .into_iter()
                .map(|(id, icon, area)| {
                    row()
                        .id(id)
                        .w(px(46.))
                        .h_full()
                        .justify_center()
                        .window_control_area(area)
                        .text_color(rgb(TEXT))
                        .font_family("Segoe MDL2 Assets")
                        .text_size(px(10.))
                        .line_height(relative(1.))
                        .hover(move |style| {
                            style
                                .bg(rgb(if area == WindowControlArea::Close {
                                    0xc42b1c
                                } else {
                                    RAISED
                                }))
                                .text_color(rgb(TEXT))
                        })
                        .child(icon)
                }),
            )
    }
    fn update_tab_notice(&mut self) {
        self.notice = if self.restoring {
            "작업 공간을 복원하는 중입니다…".into()
        } else if self.start_tab_active {
            "새 탭 · 저장소를 선택하세요".into()
        } else if self.open_projects.is_empty() {
            "열린 프로젝트가 없습니다 · + 버튼으로 새 탭을 여세요".into()
        } else {
            match self.projects[self.project].path.as_ref() {
                Some(path) => display_path(path),
                None => "저장소 경로가 없습니다".into(),
            }
        };
    }
    fn cancel_list_drags(&self) {
        self.panel_resize.set(None);
        for handle in self.panel_scrolls.values() {
            handle.cancel_drag();
        }
        for history in self.histories.values() {
            history.scroll_frame.cancel_drag();
        }
    }

    fn open_start_tab(&mut self) {
        if self.restoring || self.repository_busy() {
            return;
        }
        self.invalidate_requests();
        self.cancel_list_drags();
        self.history_resize = None;
        self.start_tab_open = true;
        self.start_tab_active = true;
        self.update_tab_notice();
        self.save_session();
    }
    fn select_project(&mut self, project: usize) {
        if self.restoring || self.repository_busy() {
            return;
        }
        self.cancel_list_drags();
        self.history_resize = None;
        if !self.open_projects.contains(&project) {
            return;
        }
        self.invalidate_requests();
        if self.project != project {
            self.panel_references = Ok(Vec::new());
            self.history_files = Ok(Vec::new());
            self.commit = 0;
        }
        self.project = project;
        self.start_tab_active = false;
        self.prepare_history();
        self.update_tab_notice();
        self.remember_current_repository();
        self.save_session();
    }
    fn close_start_tab(&mut self) {
        if self.restoring || self.repository_busy() {
            return;
        }
        self.cancel_list_drags();
        self.history_resize = None;
        if !self.start_tab_open {
            return;
        }
        self.invalidate_requests();
        self.start_tab_open = false;
        self.start_tab_active = false;
        if !self.open_projects.is_empty() {
            self.prepare_history();
        }
        self.update_tab_notice();
        self.save_session();
    }
    fn close_project(&mut self, project: usize) {
        if self.restoring || self.repository_busy() {
            return;
        }
        self.cancel_list_drags();
        self.history_resize = None;
        let Some(position) = self.open_projects.iter().position(|id| *id == project) else {
            return;
        };
        self.invalidate_requests();
        self.open_projects.remove(position);
        self.projects[project].history_cursor = None;
        if self
            .history_append_inflight
            .as_ref()
            .is_some_and(|request| request.project == project)
        {
            self.history_append_reads.cancel();
            self.history_append_pending = None;
            self.history_append_inflight = None;
        }
        self.histories.remove(&project);
        self.panel_scrolls
            .retain(|(_, owner, _), _| *owner != project);
        if self.open_projects.is_empty() {
            self.start_tab_active = self.start_tab_open;
        } else if self.project == project {
            self.project = self.open_projects[position.min(self.open_projects.len() - 1)];
            if self.start_tab_open && position == self.open_projects.len() {
                self.start_tab_active = true;
            }
        }
        self.prepare_history();
        self.update_tab_notice();
        self.save_session();
    }
    fn tabs(&self, cx: &mut Context<Self>) -> Div {
        row()
            .h(px(42.))
            .border_t_1()
            .flex_shrink_0()
            .bg(rgb(BASE))
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                row()
                    .id("project-tabs")
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .children(
                        self.open_projects
                            .iter()
                            .copied()
                            .map(|index| (index, self.projects[index].name.clone(), false))
                            .chain(self.start_tab_open.then_some((
                                usize::MAX,
                                "새 탭".to_owned(),
                                true,
                            )))
                            .map(|(index, title, start_tab)| {
                                let active = if start_tab {
                                    self.start_tab_active
                                } else {
                                    !self.start_tab_active && self.project == index
                                };
                                row()
                                    .id(("project", index))
                                    .group("project-tab")
                                    .border_r_1()
                                    .border_color(rgb(LINE))
                                    .h_full()
                                    .flex_shrink_0()
                                    .pl_5()
                                    .pr_3()
                                    .gap_4()
                                    .relative()
                                    .bg(rgb(if active { PANEL } else { BASE }))
                                    .text_color(rgb(if active { TEXT } else { MUTED }))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(RAISED)))
                                    .when(active, |el| {
                                        el.child(
                                            div()
                                                .absolute()
                                                .top_0()
                                                .left_0()
                                                .right_0()
                                                .h(px(2.))
                                                .bg(rgb(ACCENT)),
                                        )
                                    })
                                    .child(title)
                                    .child(
                                        row()
                                            .id(("close-project", index))
                                            .when(!active, |el| el.invisible())
                                            .group_hover("project-tab", |style| style.visible())
                                            .size(px(20.))
                                            .justify_center()
                                            .rounded_md()
                                            .text_color(rgb(MUTED))
                                            .text_size(px(18.))
                                            .hover(|style| {
                                                style.bg(rgb(LINE)).text_color(rgb(if active {
                                                    ACCENT
                                                } else {
                                                    TEXT
                                                }))
                                            })
                                            .child("×")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                if start_tab {
                                                    this.close_start_tab();
                                                } else {
                                                    this.close_project(index);
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if start_tab {
                                            this.open_start_tab();
                                        } else {
                                            this.select_project(index);
                                        }
                                        cx.notify();
                                    }))
                            }),
                    ),
            )
            .child(
                row()
                    .id("add-project")
                    .w(px(40.))
                    .h_full()
                    .flex_shrink_0()
                    .justify_center()
                    .border_l_1()
                    .border_color(rgb(LINE))
                    .text_size(px(22.))
                    .text_color(rgb(MUTED))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(RAISED)).text_color(rgb(ACCENT)))
                    .child(
                        div()
                            .relative()
                            .size(px(12.))
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(5.5))
                                    .w_full()
                                    .h(px(1.))
                                    .bg(rgb(MUTED)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left(px(5.5))
                                    .h_full()
                                    .w(px(1.))
                                    .bg(rgb(MUTED)),
                            ),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_start_tab();
                        cx.notify();
                    })),
            )
    }
    fn invalidate_file_request(&mut self) {
        self.retained_file_list = None;
        self.panel_reads.cancel();
        self.panel_revision = self.panel_revision.wrapping_add(1);
        self.panel_pending = None;
        self.panel_waiting = false;
        self.panel_loading.reset();
    }

    fn invalidate_reference_request(&mut self) {
        self.reference_reads.cancel();
        self.commit_loading.reset();
        self.reference_revision = self.reference_revision.wrapping_add(1);
        self.reference_pending = None;
    }

    fn invalidate_requests(&mut self) {
        self.viewer_refresh_pending = false;
        self.retained_repository_names = None;
        self.repository_reads.cancel();
        self.repository_revision = self.repository_revision.wrapping_add(1);
        self.repository_pending = None;
        self.repository_waiting = false;
        self.repository_loading.reset();
        self.invalidate_reference_request();
        self.invalidate_file_request();
    }

    fn refresh_repository_list(&mut self) {
        let previous = self.display_repository_names().to_vec();
        self.retained_repository_names = None;
        self.repository_reads.cancel();
        self.repository_revision = self.repository_revision.wrapping_add(1);
        self.repository_pending = None;
        self.repository_waiting = false;
        self.repository_loading.reset();
        self.repository_selection.clear();
        self.panel_references = Ok(Vec::new());
        let stashes = match self.repository_tab {
            RepositoryTab::Tags => false,
            RepositoryTab::Stashes => true,
            _ => return,
        };
        if self.start_tab_active || !self.open_projects.contains(&self.project) {
            return;
        }
        let Some(path) = self
            .projects
            .get(self.project)
            .and_then(|project| project.path.clone())
        else {
            return;
        };
        self.repository_pending = Some(RepositoryListRequest {
            revision: self.repository_revision,
            project: self.project,
            path,
            stashes,
        });
        self.retained_repository_names = Some(previous);
        self.repository_waiting = true;
        self.repository_loading
            .begin(self.repository_revision, std::time::Instant::now());
    }

    fn select_repository_tab(&mut self, tab: RepositoryTab) -> bool {
        if self.repository_busy()
            || self.repository_tab == tab
            || self.start_tab_active
            || !self.open_projects.contains(&self.project)
        {
            return false;
        }
        for ((kind, project, _), scroll) in &self.panel_scrolls {
            if *kind == "branches" && *project == self.project {
                scroll.cancel_drag();
            }
        }
        let previous = self.display_repository_names().to_vec();
        self.repository_tab = tab;
        self.reference_reads.cancel();
        self.commit_loading.reset();
        self.reference_revision = self.reference_revision.wrapping_add(1);
        self.reference_pending = None;
        self.refresh_repository_list();
        if self.repository_waiting { self.retained_repository_names = Some(previous); }
        true
    }

    fn finish_repository_list(&mut self, result: RepositoryListResult) -> bool {
        let request = &result.request;
        let mode = match self.repository_tab {
            RepositoryTab::Tags => Some(false),
            RepositoryTab::Stashes => Some(true),
            _ => None,
        };
        if request.revision != self.repository_revision
            || request.project != self.project
            || mode != Some(request.stashes)
            || self.start_tab_active
            || !self.open_projects.contains(&request.project)
            || self
                .projects
                .get(self.project)
                .and_then(|project| project.path.as_ref())
                != Some(&request.path)
        {
            return false;
        }
        self.repository_waiting = false;
        self.repository_loading.reset();
        self.retained_repository_names = None;
        self.panel_references = result.names;
        true
    }

    fn refresh_panel_data(&mut self) {
        self.cancel_list_drags();
        self.invalidate_reference_request();
        self.refresh_file_data();
        self.refresh_repository_list();
    }

    fn select_file_tab(&mut self, historical: bool) -> bool {
        if self.repository_busy()
            || self.file_history == historical
            || self.start_tab_active
            || !self.open_projects.contains(&self.project)
        {
            return false;
        }
        let (files, detail) = self.display_file_list();
        let previous = (files.to_vec(), detail);
        self.file_history = historical;
        self.refresh_file_data();
        self.retained_file_list = Some(previous);
        true
    }

    fn refresh_file_data(&mut self) {
        self.request_file_data(false);
    }

    fn request_file_data(&mut self, preserve_worktree: bool) {
        let previous = (!preserve_worktree).then(|| {
            let (files, detail) = self.display_file_list();
            (files.to_vec(), detail)
        });
        for ((kind, project, _), scroll) in &self.panel_scrolls {
            if *kind == "files" && *project == self.project {
                scroll.cancel_drag();
            }
        }
        self.invalidate_file_request();
        if !preserve_worktree {
            self.file_selection.clear();
            self.history_files = Ok(Vec::new());
            self.viewer_files = Ok(Vec::new());
        }
        let Some(project) = self.projects.get_mut(self.project) else {
            return;
        };
        if !preserve_worktree && !self.file_history {
            project.files.clear();
        }
        let graph = (!preserve_worktree
            && self
                .histories
                .get(&self.project)
                .is_none_or(|history| history.len() != project.commits.len()))
        .then(|| project.commits.clone());
        self.panel_pending = Some(PanelRequest {
            revision: self.panel_revision,
            project: self.project,
            path: project.path.clone(),
            historical: self.file_history,
            commit: project
                .commits
                .get(self.commit)
                .map(|commit| commit.id.clone()),
            graph,
            selected_files: Vec::new(),
        });
        self.retained_file_list = previous;
        self.viewer_refresh_pending = preserve_worktree;
        self.panel_waiting = true;
        if preserve_worktree && !self.changed_files().is_empty() {
            self.panel_loading.begin_retaining(self.panel_revision);
        } else {
            self.panel_loading.begin(self.panel_revision, Instant::now());
        }
    }

    fn finish_panel(&mut self, result: PanelResult) -> bool {
        let request = &result.request;
        if request.revision != self.panel_revision
            || request.project != self.project
            || self.start_tab_active
            || !self.open_projects.contains(&request.project)
            || self.file_history != request.historical
            || self.projects[self.project].path != request.path
            || self.projects[self.project]
                .commits
                .get(self.commit)
                .map(|c| &c.id)
                != request.commit.as_ref()
        {
            return false;
        }
        self.panel_waiting = false;
        self.retained_file_list = None;
        self.panel_loading.reset();
        self.projects[self.project].work_state = result.work_state.unwrap_or_else(|error| {
            repository::RepositoryWorkState { error: Some(error), ..Default::default() }
        });
        self.viewer_files = result.files;
        if self.file_history {
            self.history_files = result.displayed_files;
        } else {
            self.projects[self.project].files = result.displayed_files.unwrap_or_default();
        }
        if let Some(history) = result.history {
            self.install_history(self.project, history);
        }
        if let Ok(files) = &self.viewer_files {
            let rows = request.selected_files.iter().filter_map(|(path, staged)| {
                files
                    .iter()
                    .position(|file| &file.path == path && file.staged == *staged)
            });
            self.file_selection.restore(rows, files.len());
        } else {
            self.file_selection.clear();
        }
        true
    }

    fn install_history(&mut self, project: usize, prepared: PreparedHistory) {
        let mut history = HistoryState::from_prepared(prepared);
        if let Some(previous) = self.histories.get(&project) {
            history.scroll = previous.scroll.clone();
            history.horizontal_scroll = previous.horizontal_scroll.clone();
            history.scroll_frame = previous.scroll_frame.clone();
        }
        self.histories.insert(project, history);
    }

    #[cfg(test)]
    fn history_loading_visible(&self) -> bool {
        self.panel_waiting
            && self.file_history
            && !self.start_tab_active
            && self.open_projects.contains(&self.project)
            && self.panel_loading.is_visible(self.panel_revision)
    }

    #[cfg(test)]
    fn reveal_history_loading(
        &mut self,
        revision: u64,
        backend_finished: bool,
        now: std::time::Instant,
    ) -> bool {
        if backend_finished
            || !self.panel_waiting
            || !self.file_history
            || self.start_tab_active
            || !self.open_projects.contains(&self.project)
        {
            return false;
        }
        self.panel_loading.reveal(revision, now)
    }

    fn queue_history_page(&mut self, project_id: usize, visible_end: usize) -> bool {
        if self.project != project_id
            || self.start_tab_active
            || self.repository_busy()
            || self.alert.is_some()
            || self.repository_dialog.is_some()
            || self.history_append_inflight.is_some()
            || self.commit_grid_loading().is_some()
        {
            return false;
        }
        let Some(project) = self.projects.get(project_id) else {
            return false;
        };
        let Some(history) = self.histories.get(&project_id) else {
            return false;
        };
        if !project.more_commits
            || history.append_error.is_some()
            || visible_end.saturating_add(50) < project.commits.len()
            || history.len() != project.commits.len()
            || !self.open_projects.contains(&project_id)
        {
            return false;
        }
        let Some(cursor) = project.history_cursor.clone() else {
            return false;
        };
        let request = crate::workspace_backend::HistoryPageRequest {
            project: project_id,
            offset: project.commits.len(),
            cursor,
            builder: history.graph_builder(),
        };
        self.history_append_inflight = Some(request.clone());
        self.history_append_pending = Some(request);
        true
    }
    fn finish_history_page(&mut self, result: crate::workspace_backend::HistoryPageResult) -> bool {
        let request = &result.request;
        if !self
            .history_append_inflight
            .as_ref()
            .is_some_and(|current| current.matches(request))
        {
            return false;
        }
        self.history_append_inflight = None;
        let Some(project) = self.projects.get_mut(request.project) else {
            return false;
        };
        if !self.open_projects.contains(&request.project)
            || project.commits.len() != request.offset
            || !project
                .history_cursor
                .as_ref()
                .is_some_and(|cursor| cursor.same_session(&request.cursor))
        {
            return false;
        }
        let Some(history) = self.histories.get_mut(&request.project) else {
            return false;
        };
        if history.len() != request.offset {
            return false;
        }
        match result.result {
            Ok((page, prepared)) => {
                project.commits.extend(page.commits);
                project.more_commits = page.more;
                if !page.more {
                    project.history_cursor = None;
                }
                history.append(prepared);
            }
            Err(error) => history.append_error = Some(error),
        }
        true
    }
    fn dispatch_pending(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(request) = self.history_append_pending.take() {
            let task = self
                .history_append_reads
                .spawn(cx.background_executor(), move || request.execute());
            cx.spawn_in(window, async move |this, cx| {
                let Some(result) = task.await else {
                    return;
                };
                let _ = this.update_in(cx, |this, _, cx| {
                    this.finish_history_page(result);
                    cx.notify();
                });
            })
            .detach();
        }
        if let Some(request) = self.repository_pending.take() {
            use std::sync::atomic::Ordering;
            let completed = self.repository_loading.completion();
            let task = self
                .repository_reads
                .spawn(cx.background_executor(), move || {
                    let result = request.execute();
                    completed.store(true, Ordering::Release);
                    result
                });
            cx.spawn_in(window, async move |this, cx| {
                let Some(result) = task.await else {
                    return;
                };
                let _ = this.update_in(cx, |this, _, cx| {
                    if this.finish_repository_list(result) {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if let Some(request) = self.panel_pending.take() {
            use std::sync::atomic::Ordering;
            let completed = self.panel_loading.completion();
            let task = self.panel_reads.spawn(cx.background_executor(), move || {
                let result = request.execute();
                completed.store(true, Ordering::Release);
                result
            });
            cx.spawn_in(window, async move |this, cx| {
                let Some(result) = task.await else {
                    return;
                };
                let _ = this.update_in(cx, |this, _, cx| {
                    if this.finish_panel(result) {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if let Some(request) = self.reference_pending.take() {
            let expected = request.clone();
            let completed = self.commit_loading.completion();
            let task = self
                .reference_reads
                .spawn(cx.background_executor(), move || {
                    let result = request.execute();
                    completed.store(true, std::sync::atomic::Ordering::Release);
                    result
                });
            cx.spawn_in(window, async move |this, cx| {
                let Some(result) = task.await else {
                    return;
                };
                let _ = this.update_in(cx, |this, _, cx| {
                    if this.finish_reference(expected, result) {
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if let Some((revision, result)) = self.save_pending.take() {
            let task = crate::backend::spawn(cx.background_executor(), move || result.recv());
            cx.spawn_in(window, async move |this, cx| {
                let result = task.await;
                let _ = this.update_in(cx, |this, _, cx| {
                    this.finish_save(revision, result);
                    cx.notify();
                });
            })
            .detach();
        }
    }

    fn panel_tabs(&self, files: bool, cx: &mut Context<Self>) -> Div {
        let titles: &[&str] = if files {
            &["작업 트리", "히스토리"]
        } else {
            &["로컬", "원격", "태그", "스태시", "찾기"]
        };
        row()
            .px_3()
            .py_2()
            .gap_1()
            .flex_wrap()
            .text_size(px(12.))
            .children(titles.iter().enumerate().map(|(index, title)| {
                let tab = [
                    RepositoryTab::Local,
                    RepositoryTab::Remote,
                    RepositoryTab::Tags,
                    RepositoryTab::Stashes,
                    RepositoryTab::Search,
                ][index];
                let active = if files {
                    self.file_history == (index == 1)
                } else {
                    self.repository_tab == tab
                };
                row()
                    .id((
                        if files {
                            "file-scope"
                        } else {
                            "repository-scope"
                        },
                        index,
                    ))
                    .h(px(30.))
                    .px_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(if active { ACCENT } else { LINE }))
                    .bg(rgb(if active { SELECTED } else { RAISED }))
                    .text_color(rgb(if active { ACCENT } else { MUTED }))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(LINE)).text_color(rgb(TEXT)))
                    .child(*title)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if files {
                            if this.select_file_tab(index == 1) {
                                cx.notify();
                            }
                        } else if this.select_repository_tab(tab) {
                            cx.notify();
                        }
                    }))
            }))
    }
    fn prepare_history(&mut self) {
        if self.open_projects.is_empty() {
            self.commit = 0;
            return;
        }
        let project = &self.projects[self.project];
        self.histories
            .entry(self.project)
            .or_insert_with(|| HistoryState::from_prepared(PreparedHistory::empty()));
        self.commit = self.commit.min(project.commits.len().saturating_sub(1));
        self.refresh_panel_data();
    }
    fn activate_repository(&mut self, project: Project) {
        self.cancel_list_drags();
        self.history_resize = None;
        let existing = self
            .projects
            .iter()
            .position(|p| p.path.is_some() && p.path == project.path);
        let index = if let Some(index) = existing {
            self.projects[index] = project;
            index
        } else {
            self.projects.push(project);
            self.projects.len() - 1
        };
        if !self.open_projects.contains(&index) {
            self.open_projects.push(index);
        }
        self.histories.remove(&index);
        self.panel_scrolls
            .retain(|(_, owner, _), _| *owner != index);
        self.project = index;
        self.commit = 0;
        self.start_tab_open = false;
        self.start_tab_active = false;
        self.prepare_history();
        self.update_tab_notice();
        self.remember_current_repository();
        self.save_session();
    }
    fn footer_actions_blocked(&self) -> bool {
        self.file_stage_busy || self.footer_actions_blocked_except_file_edit()
    }

    fn footer_actions_blocked_except_file_edit(&self) -> bool {
        self.checkout_branch_prompt.is_some()
            || self.merge_abort_confirmation.is_some()
            || self.reference_delete.is_some()
            || self.checkout_confirmation.is_some()
            || self.restoring
            || self.repository_busy_except_file_edit()
            || self.alert.is_some()
            || self.repository_dialog.is_some()
            || self.footer_action_busy
    }
    fn chrome_actions_enabled(&self, cx: &gpui::App) -> bool {
        !self.footer_actions_blocked_except_file_edit()
            && !self.viewer.as_ref().is_some_and(|viewer| {
                viewer.read(cx).conflict_write_pending(cx)
            })
    }

    fn footer_directory(&self) -> Option<std::path::PathBuf> {
        if self.restoring || self.start_tab_active || !self.open_projects.contains(&self.project) {
            return None;
        }
        self.projects.get(self.project)?.path.clone()
    }
    fn directory_action_context(&self) -> Option<(usize, std::path::PathBuf)> {
        if self.footer_actions_blocked() {
            return None;
        }
        Some((self.project, self.footer_directory()?))
    }
    fn directory_request_current(&self, project: usize, root: &std::path::Path) -> bool {
        project == self.project
            && self.footer_directory().as_deref() == Some(root)
            && !self.footer_actions_blocked()
    }
    fn open_directory_action(
        &mut self,
        action: workspace_backend::DirectoryAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((project, root)) = self.directory_action_context() else {
            return;
        };
        if !window.is_window_active()
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        self.footer_action_busy = true;
        let owner = root.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            workspace_backend::execute_directory_action(&root, action)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.footer_action_busy = false;
                if this.directory_request_current(project, &owner) {
                    match result {
                        Ok(Some(path)) => cx.open_with_system(&path),
                        Ok(None) => {}
                        Err(error) => this.show_alert("폴더 열기 실패", &error, window, cx),
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.footer_actions_blocked()
            || self.viewer.as_ref().is_some_and(|v| v.read(cx).is_busy(cx))
        {
            return;
        }
        crate::context_menu::dismiss(window, cx);
        self.cancel_list_drags();
        self.settings_previous_focus = window.focused(cx);
        let root = self.footer_directory();
        let panel = cx.new(|cx| crate::settings::SettingsPanel::new(root, cx));
        cx.observe(&panel, |_, _, cx| cx.notify()).detach();
        self.settings_subscription = Some(cx.subscribe_in(
            &panel,
            window,
            |this, panel, _: &crate::settings::SettingsEvent, window, cx| {
                if this
                    .settings_panel
                    .as_ref()
                    .is_some_and(|current| current.entity_id() == panel.entity_id())
                {
                    this.close_settings(window, cx);
                }
            },
        ));
        let focus = cx.focus_handle();
        focus.focus(window);
        self.settings_focus = Some(focus);
        self.settings_panel = Some(panel);
        cx.notify();
    }
    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.settings_panel.clone() else {
            return;
        };
        if panel.read(cx).child_popup_open(cx) {
            panel.update(cx, |panel, cx| {
                panel.close_child_popup(window, cx);
            });
            return;
        }
        if !panel.read(cx).can_close(cx) {
            return;
        }
        panel.read(cx).cancel();
        self.settings_panel = None;
        self.settings_subscription = None;
        self.settings_focus = None;
        if let Some(focus) = self.settings_previous_focus.take() {
            focus.focus(window);
        } else {
            window.blur();
        }
        cx.notify();
    }
    fn settings_overlay(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<Div>> {
        let panel = self.settings_panel.clone()?;
        let focus = self.settings_focus.as_ref()?;
        let footer = row()
            .gap_2()
            .child(panel.update(cx, |panel, cx| panel.save_button(cx)))
            .child(
                crate::dialog::button(
                    "settings-close",
                    "취소",
                    false,
                    panel.read(cx).can_close(cx),
                )
                .on_click(cx.listener(|this, _, window, cx| this.close_settings(window, cx))),
            );
        let mut dialog = crate::dialog::Dialog::new("settings-overlay", "환경설정")
            .width(px(760.))
            .track_focus(focus)
            .body(panel.clone())
            .footer(footer)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    if this
                        .settings_panel
                        .as_ref()
                        .is_some_and(|panel| !panel.read(cx).is_composing(cx))
                    {
                        this.close_settings(window, cx);
                    }
                }
            }));
        if panel.read(cx).child_popup_open(cx) {
            dialog = dialog.overlay(panel.update(cx, |panel, cx| panel.child_popup(window, cx)));
        }
        Some(dialog.build())
    }
    fn open_encoding_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.footer_actions_blocked()
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        crate::context_menu::dismiss(window, cx);
        self.cancel_list_drags();
        self.encoding_previous_focus = window.focused(cx);
        let current = crate::text_encoding::TextEncoding::current(cx);
        let dialog = cx.new(|cx| crate::encoding_dialog::EncodingDialog::new(current, cx));
        self.encoding_subscription = Some(cx.subscribe_in(
            &dialog,
            window,
            |this, dialog, event: &crate::encoding_dialog::EncodingDialogEvent, window, cx| {
                if !this
                    .encoding_dialog
                    .as_ref()
                    .is_some_and(|current| current.entity_id() == dialog.entity_id())
                {
                    return;
                }
                if let crate::encoding_dialog::EncodingDialogEvent::Select(encoding) = event
                    && crate::text_encoding::TextEncoding::current(cx) != *encoding
                {
                    cx.set_global(*encoding);
                }
                this.close_encoding_dialog(window, cx);
            },
        ));
        dialog.read(cx).focus(window);
        self.encoding_dialog = Some(dialog);
        cx.notify();
    }
    fn close_encoding_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.encoding_dialog = None;
        self.encoding_subscription = None;
        if let Some(focus) = self.encoding_previous_focus.take() {
            focus.focus(window);
        } else {
            window.blur();
        }
        cx.notify();
    }
    fn status_bar(&self, cx: &mut Context<Self>) -> Div {
        let directory = self.footer_directory();
        let path = directory.as_ref().map(|path| display_path(path));
        let message = path
            .as_ref()
            .filter(|path| !self.notice.is_empty() && **path != self.notice)
            .map(|_| self.notice.clone());
        let encoding_enabled = self.chrome_actions_enabled(cx);
        let folder_enabled = directory.is_some() && encoding_enabled;
        row()
            .h(px(32.))
            .flex_shrink_0()
            .px_4()
            .gap_3()
            .border_t_1()
            .border_color(rgb(LINE))
            .bg(rgb(PANEL))
            .text_size(px(11.))
            .child(
                label(path.unwrap_or_else(|| self.notice.clone()))
                    .min_w_0()
                    .max_w(px(400.))
                    .truncate(),
            )
            .when_some(message, |el, message| {
                el.child(label(message).min_w_0().truncate())
            })
            .when_some(self.session_warning(), |el, warning| {
                el.child(label(warning).text_color(rgb(ACCENT)).min_w_0().truncate())
            })
            .child(div().flex_1())
            .child(
                row()
                    .gap_2()
                    .flex_shrink_0()
                    .child(
                        status_button("footer-git-bash", "Git Bash", folder_enabled).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.open_directory_action(
                                    workspace_backend::DirectoryAction::GitBash,
                                    window,
                                    cx,
                                )
                            }),
                        ),
                    )
                    .when(cfg!(windows), |bar| {
                        bar.child(
                            status_button("footer-powershell", "PowerShell", folder_enabled)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_directory_action(
                                        workspace_backend::DirectoryAction::PowerShell,
                                        window,
                                        cx,
                                    )
                                })),
                        )
                    })
                    .child(
                        status_button("footer-explorer", "탐색기", folder_enabled).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.open_directory_action(
                                    workspace_backend::DirectoryAction::Explorer,
                                    window,
                                    cx,
                                )
                            }),
                        ),
                    )
                    .child(
                        status_button(
                            "diff-encoding-picker",
                            crate::text_encoding::TextEncoding::current(cx).name(),
                            encoding_enabled,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| {
                                this.open_encoding_dialog(window, cx)
                            }),
                        ),
                    )
                    .child(
                        status_button("footer-settings", "환경설정", encoding_enabled).on_click(
                            cx.listener(|this, _, window, cx| this.open_settings(window, cx)),
                        ),
                    ),
            )
    }

    fn show_alert(
        &mut self,
        title: &str,
        message: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.previous_focus = window.focused(cx);
        self.alert = Some(Alert::new(title.to_owned(), message.to_owned()));
        let focus = cx.focus_handle();
        focus.focus(window);
        self.alert_focus = Some(focus);
        cx.notify();
    }
    fn open_repository_dialog(
        &mut self,
        operation: Operation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.restoring
            || self.repository_dialog.is_some()
            || self.alert.is_some()
            || self.repository_busy()
        {
            return;
        }
        self.dialog_previous_focus = window.focused(cx);
        let dialog = cx.new(|cx| RepositoryDialog::new(operation, cx));
        self.dialog_subscription = Some(cx.subscribe_in(
            &dialog,
            window,
            |this, dialog, event: &DialogEvent, window, cx| {
                if !this
                    .repository_dialog
                    .as_ref()
                    .is_some_and(|current| current.entity_id() == dialog.entity_id())
                {
                    return;
                }
                match event {
                    DialogEvent::Browse => this.browse_repository_path(window, cx),
                    DialogEvent::Confirm(request) => {
                        this.submit_repository(request.clone(), window, cx)
                    }
                    DialogEvent::Cancel => {
                        if !dialog.read(cx).is_busy() {
                            this.close_repository_dialog(window, cx);
                        }
                    }
                }
            },
        ));
        dialog.read(cx).focus_first(window, cx);
        self.repository_dialog = Some(dialog);
        cx.notify();
    }
    fn close_repository_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.repository_dialog = None;
        self.dialog_subscription = None;
        if let Some(focus) = self.dialog_previous_focus.take() {
            focus.focus(window);
        } else {
            window.blur();
        }
        cx.notify();
    }
    fn browse_repository_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.repository_dialog.clone() else {
            return;
        };
        if dialog.read(cx).is_busy() {
            return;
        }
        dialog.update(cx, |dialog, cx| {
            dialog.set_activity(DialogActivity::Browsing, window, cx)
        });
        let selected = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(if dialog.read(cx).operation == Operation::Open { "Git 디렉터리 선택" } else { "작업 폴더 선택" }.into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = selected.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if !this
                    .repository_dialog
                    .as_ref()
                    .is_some_and(|current| current.entity_id() == dialog.entity_id())
                {
                    return;
                }
                dialog.update(cx, |dialog, cx| {
                    dialog.set_activity(DialogActivity::Idle, window, cx)
                });
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            let path = repository::selected_repository_path(dialog.read(cx).operation, path);
                            let input = dialog.read(cx).path.clone();
                            input.update(cx, |input, cx| {
                                input.set_text(&path.to_string_lossy(), cx)
                            });
                            input.read(cx).focus_handle.focus(window);
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => this.show_alert(
                        "경로 선택 오류",
                        "폴더 선택 창을 열지 못했습니다. 경로를 직접 입력하세요.",
                        window,
                        cx,
                    ),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn submit_repository(&mut self, request: Request, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.repository_dialog.as_ref() else {
            return;
        };
        if dialog.read(cx).is_busy()
            || self.repository_busy()
            || self.restoring
            || self.alert.is_some()
        {
            return;
        }
        let failure_title = request.operation.title();
        self.close_repository_dialog(window, cx);
        self.start_repository_operation(request, failure_title, window, cx);
    }
    fn dismiss_alert(&mut self) {
        self.alert = None;
        self.alert_focus = None;
    }
    fn start_page(&mut self, cx: &mut Context<Self>) -> Div {
        let focus = self
            .recent_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let mouse_focus = focus.clone();

        div()
            .flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .gap_5()
            .p_5()
            .child(
                div()
                    .id("start-actions")
                    .flex()
                    .flex_col()
                    .w(px(300.))
                    .max_w(relative(0.4))
                    .min_w_0()
                    .min_h_0()
                    .flex_shrink_0()
                    .overflow_y_scroll()
                    .gap_3()
                    .children(["Create", "Open", "Clone"].into_iter().enumerate().map(
                        |(index, title)| {
                            row()
                                .id(("start-action", index))
                                .h(px(36.))
                                .flex_shrink_0()
                                .justify_center()
                                .gap_2()
                                .border_1()
                                .border_color(rgb(LINE))
                                .rounded_md()
                                .bg(rgb(if self.recent_selection == Some(index) {
                                    SELECTED
                                } else {
                                    PANEL
                                }))
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(RAISED)).text_color(rgb(ACCENT)))
                                .child(start_action_icon(index))
                                .child(title)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let operation =
                                        [Operation::Create, Operation::Open, Operation::Clone]
                                            [index];
                                    this.open_repository_dialog(operation, window, cx);
                                }))
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .gap_3()
                    .pl_5()
                    .border_l_1()
                    .border_color(rgb(LINE))
                    .child(div().text_size(px(18.)).child("최근 저장소"))
                    .child(
                        div()
                            .id("recent-repositories")
                            .track_focus(&focus)
                            .track_scroll(&self.recent_scroll)
                            .on_mouse_down(gpui::MouseButton::Left, move |_, window, _| {
                                mouse_focus.focus(window)
                            })
                            .on_key_down(cx.listener(
                                move |this, event: &gpui::KeyDownEvent, window, cx| {
                                    if !focus.is_focused(window) || this.footer_actions_blocked() {
                                        return;
                                    }
                                    if let Some(down) =
                                        crate::virtual_list::arrow_direction(&event.keystroke)
                                    {
                                        this.recent_selection =
                                            crate::virtual_list::adjacent_selection(
                                                this.recent_selection,
                                                this.recent_repositories.len(),
                                                down,
                                            );
                                        if let Some(index) = this.recent_selection {
                                            this.recent_scroll.scroll_to_item(index);
                                        }
                                        cx.notify();
                                        cx.stop_propagation();
                                    } else if event.keystroke.key == "enter" {
                                        if let Some(index) = this.recent_selection {
                                            this.open_recent_repository(index, window, cx);
                                        }
                                        cx.stop_propagation();
                                    }
                                },
                            ))
                            .flex()
                            .flex_col()
                            .flex_1()
                            .w_full()
                            .min_w_0()
                            .min_h_0()
                            .overflow_y_scroll()
                            .gap_2()
                            .when(self.recent_repositories.is_empty(), |el| {
                                el.child(panel_message("아직 열었던 저장소가 없습니다."))
                            })
                            .children(self.recent_repositories.iter().enumerate().map(
                                |(index, entry)| {
                                    div()
                                        .id(("recent-repository", index))
                                        .flex()
                                        .items_center()
                                        .w_full()
                                        .flex_shrink_0()
                                        .min_w_0()
                                        .p_3()
                                        .gap_3()
                                        .rounded_md()
                                        .bg(rgb(PANEL))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(rgb(RAISED)))
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .flex_1()
                                                .min_w_0()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .w_full()
                                                        .min_w_0()
                                                        .flex_shrink_0()
                                                        .min_h(px(20.))
                                                        .line_height(px(20.))
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .truncate()
                                                        .child(entry.name.clone()),
                                                )
                                                .child(
                                                    label(display_path(&entry.path))
                                                        .w_full()
                                                        .min_w_0()
                                                        .flex_shrink_0()
                                                        .min_h(px(18.))
                                                        .line_height(px(18.))
                                                        .text_size(px(11.))
                                                        .truncate(),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id(("delete-recent-repository", index))
                                                .size(px(30.))
                                                .flex_shrink_0()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded_md()
                                                .cursor_pointer()
                                                .hover(|style| style.bg(rgb(SELECTED)))
                                                .font_family("Segoe MDL2 Assets")
                                                .child("\u{E74D}")
                                                .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        cx.stop_propagation();
                                                        this.open_recent_removal(index, window, cx);
                                                    },
                                                )),
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.open_recent_repository(index, window, cx);
                                        }))
                                },
                            )),
                    ),
            )
    }
    fn sync_available(&self, cx: &gpui::App) -> bool {
        self.footer_directory().is_some()
            && !self.footer_actions_blocked()
            && !self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
    }
    fn choose_sync_mode(&mut self, mode: repository::SyncMode) -> bool {
        if !repository::SyncMode::OPTIONS.contains(&mode) || self.footer_actions_blocked() {
            return false;
        }
        if self.selected_sync_mode != mode {
            self.selected_sync_mode = mode;
            self.save_session();
        }
        true
    }
    fn start_sync(
        &mut self,
        mode: repository::SyncMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(mode, repository::SyncMode::Merge | repository::SyncMode::FastForwardOnly) {
            self.open_git_confirmation(PendingGitAction::Sync(mode), window, cx);
        } else { self.execute_sync(mode, window, cx); }
    }
    fn execute_sync(&mut self, mode: repository::SyncMode, window: &mut Window, cx: &mut Context<Self>) {
        if !repository::SyncMode::OPTIONS.contains(&mode) || !self.sync_available(cx) || !window.is_window_active() {
            return;
        }
        let Some(path) = self.footer_directory() else {
            return;
        };
        self.cancel_list_drags();
        self.invalidate_requests();
        self.history_append_reads.cancel();
        self.history_append_pending = None;
        self.history_append_inflight = None;
        self.sync_revision = self.sync_revision.wrapping_add(1);
        self.sync_busy = true;
        self.progress_previous_focus = window.focused(cx);
        let focus = cx.focus_handle();
        focus.focus(window);
        self.progress_focus = Some(focus);
        self.sync_operation = Some(match mode {
            repository::SyncMode::Fetch => "Fetch 실행 중…",
            repository::SyncMode::Merge => "Pull 병합 실행 중…",
            repository::SyncMode::FastForwardOnly => "Pull 실행 중…",
            repository::SyncMode::Rebase => "Pull 리베이스 실행 중…",
        });
        let request = workspace_backend::SyncRequest {
            revision: self.sync_revision,
            project: self.project,
            path,
            name: self.projects[self.project].name.clone(),
            mode,
        };
        let task = crate::backend::spawn(cx.background_executor(), move || request.execute());
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let current = result.request.revision == this.sync_revision;
                let conflict_paths = this.sync_failure_paths(&result);
                let error = this.finish_sync(result);
                if current {
                    this.progress_focus = None;
                    if let Some(previous) = this.progress_previous_focus.take() {
                        previous.focus(window);
                    } else {
                        window.blur();
                    }
                }
                if let Some(error) = error {
                    this.show_alert("원격 작업 실패", &error, window, cx);
                    if let Some(alert) = &mut this.alert { alert.set_paths(conflict_paths); }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn sync_failure_paths(&self, result: &workspace_backend::SyncResult) -> Vec<String> {
        if result.request.revision != self.sync_revision
            || result.request.project != self.project
            || self.footer_directory().as_ref() != Some(&result.request.path)
            || result.request.mode == repository::SyncMode::Fetch
        {
            return Vec::new();
        }
        result.conflict_paths.iter().map(|path| display_path(path)).collect()
    }
    fn finish_sync(&mut self, result: workspace_backend::SyncResult) -> Option<String> {
        let request = &result.request;
        if request.revision != self.sync_revision {
            return None;
        }
        self.sync_busy = false;
        self.sync_operation = None;
        if self.project != request.project
            || self.footer_directory().as_ref() != Some(&request.path)
        {
            return None;
        }
        let mut errors = Vec::new();
        if let Err(error) = result.outcome {
            errors.push(error);
        }
        match result.refreshed {
            Ok((project, history)) => self.apply_refreshed_project(project, history),
            Err(error) => errors.push(error),
        }
        if errors.is_empty() {
            None
        } else {
            Some(errors.join("\n"))
        }
    }
    fn apply_refreshed_project(&mut self, project: Project, history: PreparedHistory) {
        let previous = &self.projects[self.project];
        let selected = previous
            .commits
            .get(self.commit)
            .map(|commit| commit.id.clone());
        let follow_head = selected.as_ref() == previous.head.as_ref();
        self.commit = if follow_head {
            project
                .head
                .as_ref()
                .and_then(|id| project.commits.iter().position(|commit| &commit.id == id))
                .unwrap_or(0)
        } else {
            selected
                .and_then(|id| project.commits.iter().position(|commit| commit.id == id))
                .unwrap_or(0)
        };
        self.projects[self.project] = project;
        self.install_history(self.project, history);
        self.refresh_panel_data();
    }
    fn open_commit_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sync_available(cx) {
            return;
        }
        crate::context_menu::dismiss(window, cx);
        self.cancel_list_drags();
        let Some(path) = self.footer_directory() else {
            return;
        };
        self.commit_previous_focus = window.focused(cx);
        self.commit_owner = Some((self.project, path.clone()));
        let dialog = cx.new(|cx| crate::commit_dialog::CommitDialog::new(path, cx));
        self.commit_subscription = Some(cx.subscribe_in(
            &dialog,
            window,
            |this, dialog, event: &crate::commit_dialog::CommitDialogEvent, window, cx| {
                if !this
                    .commit_dialog
                    .as_ref()
                    .is_some_and(|current| current.entity_id() == dialog.entity_id())
                {
                    return;
                }
                match event {
                    crate::commit_dialog::CommitDialogEvent::Cancel => {
                        this.close_commit_dialog(window, cx)
                    }
                    crate::commit_dialog::CommitDialogEvent::Confirm(request) => {
                        this.start_commit(request.clone(), window, cx)
                    }
                }
            },
        ));
        dialog.read(cx).focus(window, cx);
        self.commit_dialog = Some(dialog);
        cx.notify();
    }
    fn close_commit_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.commit_busy
            || self
                .commit_dialog
                .as_ref()
                .is_some_and(|dialog| !dialog.read(cx).can_close())
        {
            return;
        }
        self.commit_dialog = None;
        self.commit_subscription = None;
        self.commit_owner = None;
        if let Some(previous) = self.commit_previous_focus.take() {
            previous.focus(window);
        } else {
            window.blur();
        }
        cx.notify();
    }
    fn start_commit(
        &mut self,
        commit: repository::CommitRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.commit_busy || self.sync_busy || self.alert.is_some() || !window.is_window_active()
        {
            return;
        }
        let Some(dialog) = self.commit_dialog.clone() else {
            return;
        };
        let Some((project, path)) = self.commit_owner.clone() else {
            return;
        };
        if project != self.project || self.footer_directory().as_ref() != Some(&path) {
            return;
        }
        self.invalidate_requests();
        self.cancel_list_drags();
        self.history_append_reads.cancel();
        self.history_append_pending = None;
        self.history_append_inflight = None;
        self.commit_busy = true;
        self.commit_revision = self.commit_revision.wrapping_add(1);
        dialog.update(cx, |dialog, cx| dialog.set_busy(true, cx));
        self.progress_previous_focus = window.focused(cx);
        let focus = cx.focus_handle();
        focus.focus(window);
        self.progress_focus = Some(focus);
        let request = workspace_backend::CommitWorkRequest {
            revision: self.commit_revision,
            project,
            path,
            name: self.projects[project].name.clone(),
            commit,
        };
        let task = crate::backend::spawn(cx.background_executor(), move || request.execute());
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let Some(completion) = this.finish_commit_work(result) else {
                    return;
                };
                this.progress_focus = None;
                let previous = this.progress_previous_focus.take();
                if let Some(dialog) = this.commit_dialog.clone() {
                    dialog.update(cx, |dialog, cx| dialog.set_busy(false, cx));
                }
                if completion.commit_id.is_some() {
                    this.close_commit_dialog(window, cx);
                    if let Some(error) = completion.error {
                        this.show_alert("커밋 후 처리 실패", &error, window, cx);
                    }
                } else {
                    if let Some(previous) = previous {
                        previous.focus(window);
                    }
                    let error = completion
                        .error
                        .unwrap_or_else(|| "커밋하지 못했습니다.".into());
                    if let Some(dialog) = this.commit_dialog.clone() {
                        dialog.update(cx, |dialog, cx| dialog.fail(error, cx));
                    } else {
                        this.show_alert("커밋 실패", &error, window, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn finish_commit_work(
        &mut self,
        result: workspace_backend::CommitWorkResult,
    ) -> Option<CommitCompletion> {
        let request = &result.request;
        if request.revision != self.commit_revision {
            return None;
        }
        self.commit_busy = false;
        if request.project != self.project
            || self.footer_directory().as_ref() != Some(&request.path)
        {
            return Some(CommitCompletion {
                commit_id: None,
                error: Some("커밋 대상 저장소가 변경되었습니다.".into()),
            });
        }
        let (commit_id, mut errors) = match result.outcome {
            Ok(outcome) => (
                Some(outcome.id),
                outcome.error.into_iter().collect::<Vec<_>>(),
            ),
            Err(error) => (None, vec![error]),
        };
        match result.refreshed {
            Ok((project, history)) => self.apply_refreshed_project(project, history),
            Err(error) => errors.push(error),
        }
        Some(CommitCompletion {
            commit_id,
            error: (!errors.is_empty()).then(|| errors.join("\n")),
        })
    }
    fn tag_dialog_context(&self, id: &str) -> bool {
        self.tag_dialog_commit.as_deref() == Some(id)
            && self.branch_owner.as_ref().is_some_and(|(project, path)| {
                *project == self.project && self.footer_directory().as_ref() == Some(path)
            })
    }

    fn open_tag_dialog(
        &mut self,
        project: usize,
        index: usize,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.sync_available(cx)
            || self.checkout_menu_target((project, None, index)).as_ref() != Some(&id)
        {
            return;
        }
        self.open_branch_dialog(window, cx);
        if let Some(dialog) = self.branch_dialog.clone() {
            self.tag_dialog_commit = Some(id.clone());
            dialog.update(cx, |dialog, cx| dialog.configure_tag(id, cx));
            dialog.read(cx).focus(window, cx);
        }
    }

    fn stash_dialog_context(&self) -> bool {
        self.stash_dialog
            && self.branch_owner.as_ref().is_some_and(|(project, path)| {
                *project == self.project && self.footer_directory().as_ref() == Some(path)
            })
    }

    fn open_stash_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sync_available(cx) {
            return;
        }
        self.open_branch_dialog(window, cx);
        if let Some(dialog) = self.branch_dialog.clone() {
            self.stash_dialog = true;
            dialog.update(cx, |dialog, cx| dialog.configure_stash(cx));
            dialog.read(cx).focus(window, cx);
        }
    }

    fn open_branch_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sync_available(cx) {
            return;
        }
        let Some(path) = self.footer_directory() else {
            return;
        };
        crate::context_menu::dismiss(window, cx);
        self.cancel_list_drags();
        self.branch_previous_focus = window.focused(cx);
        self.branch_owner = Some((self.project, path));
        self.branch_remote = None;
        self.tag_dialog_commit = None;
        self.stash_dialog = false;
        let dialog = cx.new(crate::branch_dialog::BranchDialog::new);
        self.branch_subscription = Some(cx.subscribe_in(
            &dialog,
            window,
            |this, dialog, event: &crate::branch_dialog::BranchDialogEvent, window, cx| {
                if !this
                    .branch_dialog
                    .as_ref()
                    .is_some_and(|current| current.entity_id() == dialog.entity_id())
                {
                    return;
                }
                match event {
                    crate::branch_dialog::BranchDialogEvent::Cancel => {
                        this.close_branch_dialog(window, cx)
                    }
                    crate::branch_dialog::BranchDialogEvent::ConfirmStash { name, keep_index } => {
                        this.start_toolbar_action(
                            workspace_backend::ToolbarAction::CreateStash {
                                name: name.clone(),
                                keep_index: *keep_index,
                            },
                            window,
                            cx,
                        );
                    }
                    crate::branch_dialog::BranchDialogEvent::ConfirmTag { id, name, push } => {
                        this.start_toolbar_action(
                            workspace_backend::ToolbarAction::CreateTag {
                                id: id.clone(),
                                name: name.clone(),
                                push: *push,
                            },
                            window,
                            cx,
                        );
                    }
                    crate::branch_dialog::BranchDialogEvent::Confirm { name, checkout } => this
                        .start_toolbar_action(
                            if let Some(remote) = &this.branch_remote {
                                workspace_backend::ToolbarAction::CreateTrackingBranch {
                                    name: name.clone(),
                                    remote: remote.clone(),
                                }
                            } else {
                                workspace_backend::ToolbarAction::CreateBranch {
                                    name: name.clone(),
                                    checkout: *checkout,
                                }
                            },
                            window,
                            cx,
                        ),
                }
            },
        ));
        dialog.read(cx).focus(window, cx);
        self.branch_dialog = Some(dialog);
        cx.notify();
    }
    fn close_branch_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sync_busy {
            return;
        }
        self.branch_dialog = None;
        self.branch_subscription = None;
        self.branch_owner = None;
        self.branch_remote = None;
        self.tag_dialog_commit = None;
        self.stash_dialog = false;
        if let Some(previous) = self.branch_previous_focus.take() {
            previous.focus(window);
        } else {
            window.blur();
        }
        cx.notify();
    }
    fn existing_remote_local_branch(&self, reference: &str) -> Option<String> {
        let upstream = reference.strip_prefix("refs/remotes/")?;
        let project = self.projects.get(self.project)?;
        let name = repository::remote_branch_label(upstream, &project.remote_names);
        project
            .branches
            .iter()
            .any(|branch| branch == name)
            .then(|| format!("refs/heads/{name}"))
    }

    fn history_action_request(&self, row: (usize, Option<RepositoryTab>, usize)) -> Option<repository::CommitHistoryRequest> {
        if row.1.is_some() { return None; }
        let target = self.checkout_menu_target(row)?;
        let project = self.projects.get(row.0)?;
        if project.work_state.operation.is_some() || project.work_state.has_conflicts || project.work_state.error.is_some() { return None; }
        Some(repository::CommitHistoryRequest { target, expected_head: project.head.clone()?,
            expected_reference: project.branches.contains(&project.branch).then(|| format!("refs/heads/{}", project.branch)) })
    }

    fn git_prompt(&self, action: PendingGitAction) -> Option<GitActionPrompt> {
        if self.footer_actions_blocked() { return None; }
        let project = self.projects.get(self.project)?;
        let id = match &action {
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::RevertCommit { request, .. }
                | workspace_backend::ToolbarAction::ResetCommit { request, .. }) => Some(&request.target),
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::MergeCommit { id }) => Some(id),
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::RebaseCommit { request }) => Some(&request.target),
            _ => None,
        };
        let target = match id { Some(id) => Some((project.commits.iter().position(|commit| &commit.id == id)?, id.clone())), None => None };
        Some(GitActionPrompt { project: self.project, path: self.footer_directory()?, head: project.head.clone(), branch: project.branch.clone(), target, action })
    }

    fn take_git_confirmation(&mut self, options: Option<crate::git_action_dialog::ActionOptions>) -> Option<PendingGitAction> {
        use crate::git_action_dialog::ActionOptions;
        let mut prompt = self.git_action_prompt.take()?;
        let options = options?;
        let project = self.projects.get(self.project)?;
        if self.repository_busy() || self.alert.is_some() || self.repository_dialog.is_some()
            || prompt.project != self.project || self.footer_directory().as_ref() != Some(&prompt.path)
            || project.head != prompt.head || project.branch != prompt.branch
            || prompt.target.as_ref().is_some_and(|(index, id)| self.commit != *index || project.commits.get(*index).is_none_or(|commit| &commit.id != id)) { return None; }
        match (&mut prompt.action, options) {
            (PendingGitAction::Toolbar(workspace_backend::ToolbarAction::RevertCommit { no_commit, .. }), ActionOptions::Revert(value)) => *no_commit = value,
            (PendingGitAction::Toolbar(workspace_backend::ToolbarAction::ResetCommit { mode, .. }), ActionOptions::Reset(value)) => *mode = value,
            (PendingGitAction::Toolbar(workspace_backend::ToolbarAction::MergeCommit { .. } | workspace_backend::ToolbarAction::RebaseCommit { .. }) | PendingGitAction::Sync(_), ActionOptions::Confirm) => {},
            (PendingGitAction::Toolbar(action @ (workspace_backend::ToolbarAction::Push | workspace_backend::ToolbarAction::PushWithLease)), ActionOptions::Push(value)) => {
                *action = if value { workspace_backend::ToolbarAction::PushWithLease } else { workspace_backend::ToolbarAction::Push };
            }
            _ => return None,
        }
        Some(prompt.action)
    }

    fn open_git_confirmation(&mut self, action: PendingGitAction, window: &mut Window, cx: &mut Context<Self>) {
        use crate::git_action_dialog::{ActionOptions, GitActionDialog, GitActionEvent};
        if !window.is_window_active() || !self.sync_available(cx) { return; }
        let Some(prompt) = self.git_prompt(action) else { return; };
        let (title, options, detail) = match &prompt.action {
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::RevertCommit { .. }) => ("Revert", ActionOptions::Revert(false), "선택한 커밋의 변경을 되돌립니다. 병합 커밋은 첫 번째 부모를 기준으로 처리합니다.".to_owned()),
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::ResetCommit { .. }) => ("Reset", ActionOptions::Reset(repository::ResetMode::Mixed), "선택한 커밋으로 현재 HEAD를 이동합니다.".to_owned()),
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::MergeCommit { .. }) => ("병합 확인", ActionOptions::Confirm, "선택한 커밋을 현재 브랜치에 병합합니다.".to_owned()),
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::RebaseCommit { request }) => ("리베이스 확인", ActionOptions::Confirm, request.reverse_branch.as_ref().map_or_else(|| "현재 브랜치를 선택한 커밋 위에 리베이스합니다.".into(), |branch| format!("{branch} 브랜치를 현재 HEAD 위에 리베이스합니다."))),
            PendingGitAction::Toolbar(workspace_backend::ToolbarAction::Push | workspace_backend::ToolbarAction::PushWithLease) => ("Push 확인", ActionOptions::Push(false), "현재 브랜치를 원격 저장소로 Push합니다.".to_owned()),
            PendingGitAction::Sync(_) => ("Pull 병합 확인", ActionOptions::Confirm, "원격 변경을 가져와 현재 브랜치에 병합합니다.".to_owned()),
            _ => return,
        };
        let mut description = format!("저장소: {}\n브랜치: {}\n{}", crate::path_display::display_path(&prompt.path), prompt.branch, detail);
        if let Some((_, id)) = &prompt.target { description.push_str(&format!("\n대상: {id}")); }
        crate::context_menu::dismiss(window, cx); self.cancel_list_drags();
        self.git_action_previous_focus = window.focused(cx);
        let dialog = cx.new(|cx| GitActionDialog::new(title.into(), description, options, cx));
        self.git_action_prompt = Some(prompt);
        self.git_action_subscription = Some(cx.subscribe_in(&dialog, window, |this, source, event: &GitActionEvent, window, cx| {
            if !this.git_action_dialog.as_ref().is_some_and(|dialog| dialog.entity_id() == source.entity_id()) { return; }
            let options = match event { GitActionEvent::Confirm(value) if window.is_window_active() => Some(*value), _ => None };
            let action = this.take_git_confirmation(options);
            this.git_action_dialog = None; this.git_action_subscription = None;
            if let Some(focus) = this.git_action_previous_focus.take() { focus.focus(window); } else { window.blur(); }
            if let Some(action) = action { match action {
                PendingGitAction::Toolbar(action) => this.execute_toolbar_action(action, window, cx),
                PendingGitAction::Sync(mode) => this.execute_sync(mode, window, cx),
            } } else if options.is_some() {
                this.show_alert("Git 작업 대상 변경", "확인한 저장소·브랜치 또는 커밋이 변경되었습니다. 다시 선택한 뒤 실행하세요.", window, cx);
            }
            cx.notify();
        }));
        dialog.read(cx).focus(window); self.git_action_dialog = Some(dialog); cx.notify();
    }

    fn start_toolbar_action(
        &mut self,
        action: workspace_backend::ToolbarAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.checkout_branch_prompt.is_some() || self.checkout_confirmation.is_some() || self.merge_abort_confirmation.is_some() {
            return;
        }
        if let workspace_backend::ToolbarAction::Checkout { reference } = &action
            && reference.starts_with("refs/remotes/")
        {
            if let Some(reference) = self.existing_remote_local_branch(reference) {
                self.start_toolbar_action(
                    workspace_backend::ToolbarAction::Checkout { reference },
                    window,
                    cx,
                );
                return;
            }
            let short = reference.strip_prefix("refs/remotes/").unwrap_or(reference);
            let name =
                repository::remote_branch_label(short, &self.projects[self.project].remote_names)
                    .to_owned();
            self.open_branch_dialog(window, cx);
            if let Some(dialog) = self.branch_dialog.clone() {
                self.branch_remote = Some(reference.clone());
                dialog.update(cx, |dialog, cx| {
                    dialog.configure_remote(reference.clone(), name, cx)
                });
                dialog.read(cx).focus(window, cx);
            }
            return;
        }

        if let workspace_backend::ToolbarAction::Checkout { reference } = &action
            && !reference.starts_with("refs/heads/")
        {
            if !self.sync_available(cx) || self.alert.is_some() || !window.is_window_active() {
                return;
            }
            let Some(path) = self.footer_directory() else {
                return;
            };
            self.checkout_confirmation = Some((self.project, path, reference.clone()));
            self.checkout_previous_focus = window.focused(cx);
            let focus = cx.focus_handle();
            focus.focus(window);
            self.checkout_focus = Some(focus);
            self.checkout_buttons = Some(std::array::from_fn(|_| cx.focus_handle()));
            cx.notify();
            return;
        }
        if matches!(action, workspace_backend::ToolbarAction::RevertCommit { .. } | workspace_backend::ToolbarAction::ResetCommit { .. }
            | workspace_backend::ToolbarAction::MergeCommit { .. } | workspace_backend::ToolbarAction::RebaseCommit { .. } | workspace_backend::ToolbarAction::Push | workspace_backend::ToolbarAction::PushWithLease) {
            self.open_git_confirmation(PendingGitAction::Toolbar(action), window, cx);
        } else { self.execute_toolbar_action(action, window, cx); }
    }

    fn history_checkout_candidates(&self, row: (usize, Option<RepositoryTab>, usize)) -> Option<Vec<String>> {
        if row.1.is_some() || self.alert.is_some() || self.repository_dialog.is_some() || self.restoring { return None; }
        self.checkout_menu_target(row)?;
        let project = self.projects.get(row.0)?;
        let references = &project.commits.get(row.2)?.references;
        let mut local: Vec<_> = references.iter().filter(|entry| entry.full_name.starts_with("refs/heads/"))
            .map(|entry| entry.full_name.clone()).collect();
        local.sort(); local.dedup();
        if !local.is_empty() { return Some(local); }
        let mut remote: Vec<_> = references.iter().filter(|entry| entry.full_name.starts_with("refs/remotes/")
            && !project.remote_names.iter().any(|remote| entry.full_name == format!("refs/remotes/{remote}/HEAD"))
            && !entry.full_name.strip_prefix("refs/remotes/").and_then(|name| name.split_once('/')).is_some_and(|(_, branch)| branch == "HEAD"))
            .map(|entry| entry.full_name.clone()).collect();
        remote.sort(); remote.dedup(); Some(remote)
    }

    fn execute_history_checkout(&mut self, reference: String, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let action = if reference.starts_with("refs/heads/") { workspace_backend::ToolbarAction::CheckoutCommitBranch { reference, id } }
            else { workspace_backend::ToolbarAction::Checkout { reference } };
        self.start_toolbar_action(action, window, cx);
    }

    fn start_history_checkout(&mut self, row: (usize, Option<RepositoryTab>, usize), id: String, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sync_available(cx) || !window.is_window_active() || self.checkout_menu_target(row).as_ref() != Some(&id) { return; }
        let Some(candidates) = self.history_checkout_candidates(row) else { return; };
        match candidates.as_slice() {
            [] => self.start_toolbar_action(workspace_backend::ToolbarAction::Checkout { reference: id }, window, cx),
            _ => {
                let project = &self.projects[row.0];
                let Some(path) = project.path.clone() else { return; };
                let selected = candidates.iter().position(|reference| reference == &format!("refs/heads/{}", project.branch)).unwrap_or(0);
                self.checkout_branch_prompt = Some(BranchCheckoutPrompt { project: row.0, path, index: row.2, commit: id,
                    candidates, selected, scroll: Default::default() });
                self.checkout_branch_previous_focus = window.focused(cx);
                self.checkout_branch_focus = Some(cx.focus_handle());
                let controls = std::array::from_fn(|_| cx.focus_handle()); controls[0].focus(window);
                self.checkout_branch_controls = Some(controls);
                cx.notify();
            }
        }
    }

    fn take_branch_checkout(&mut self, confirm: bool) -> Option<(String, String)> {
        let prompt = self.checkout_branch_prompt.take()?;
        if !confirm || self.repository_busy() || self.alert.is_some()
            || prompt.project != self.project || self.footer_directory().as_ref() != Some(&prompt.path)
            || self.checkout_menu_target((prompt.project, None, prompt.index)).as_ref() != Some(&prompt.commit)
            || self.history_checkout_candidates((prompt.project, None, prompt.index)).as_ref() != Some(&prompt.candidates) { return None; }
        let reference = if prompt.selected == prompt.candidates.len() { prompt.commit.clone() } else { prompt.candidates.get(prompt.selected)?.clone() };
        Some((reference, prompt.commit))
    }

    fn choose_branch_checkout(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.take_branch_checkout(confirm);
        self.checkout_branch_focus = None; self.checkout_branch_controls = None;
        if let Some(focus) = self.checkout_branch_previous_focus.take() { focus.focus(window); }
        if let Some((reference, id)) = selected { self.execute_history_checkout(reference, id, window, cx); }
        cx.notify();
    }

    fn select_checkout_branch(&mut self, index: usize) {
        if let Some(prompt) = &mut self.checkout_branch_prompt && index <= prompt.candidates.len() {
            prompt.selected = index;
            prompt.scroll.vertical.scroll_to_item(index, gpui::ScrollStrategy::Center);
        }
    }
    fn branch_checkout_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let prompt = self.checkout_branch_prompt.as_ref()?;
        let focus = self.checkout_branch_focus.as_ref()?;
        let controls = self.checkout_branch_controls.as_ref()?;
        let count = prompt.candidates.len() + 1;
        let rows = gpui::uniform_list("checkout-branch-options", count,
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                let Some(prompt) = &this.checkout_branch_prompt else { return Vec::new(); };
                range.filter_map(|index| {
                    if index == prompt.candidates.len() { Some((index, "Detached HEAD")) }
                    else { prompt.candidates.get(index).map(|name| (index, name.as_str())) }
                }).map(|(index, name)| {
                    let selected = index == prompt.selected;
                    div().h(px(32.)).pb_1().child(
                        div().id(("checkout-branch-option", index))
                            .h(px(28.)).px_3().flex().items_center().gap_2()
                            .rounded_sm().cursor_pointer()
                            .when(selected, |el| el.bg(rgb(SELECTED)))
                            .hover(|style| style.bg(rgb(RAISED)))
                            .child(div().w(px(16.)).flex_shrink_0().child(if selected { "✓" } else { "" }))
                            .child(label(name.trim_start_matches("refs/heads/").trim_start_matches("refs/remotes/").to_owned()).truncate())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.select_checkout_branch(index);
                                if let Some(controls) = &this.checkout_branch_controls { controls[0].focus(window); }
                                cx.notify();
                            }))
                    )
                }).collect::<Vec<_>>()
            }))
            .track_scroll(prompt.scroll.vertical.clone())
            .w_full().flex_1().min_h_0();
        Some(crate::dialog::Dialog::new("checkout-branch-dialog", "체크아웃할 브랜치 선택")
            .width(px(400.))
            .track_focus(focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => this.choose_branch_checkout(false, window, cx),
                    "enter" => {
                        let confirm = !this.checkout_branch_controls.as_ref().is_some_and(|controls| controls[2].is_focused(window));
                        this.choose_branch_checkout(confirm, window, cx);
                    },
                    "up" | "down" => {
                        if let Some(prompt) = &this.checkout_branch_prompt {
                            let next = if event.keystroke.key == "down" { (prompt.selected + 1).min(prompt.candidates.len()) }
                                else { prompt.selected.saturating_sub(1) };
                            this.select_checkout_branch(next); cx.notify();
                        }
                    },
                    "tab" => {
                        if let Some(controls) = &this.checkout_branch_controls {
                            let current = controls.iter().position(|focus| focus.is_focused(window)).unwrap_or(0);
                            let next = if event.keystroke.modifiers.shift { (current + 2) % 3 } else { (current + 1) % 3 };
                            controls[next].focus(window);
                        }
                    },
                    _ => {},
                }
                cx.stop_propagation();
            }))
            .body(div().flex().flex_col().gap_3().child("브랜치 또는 Detached HEAD를 선택하세요.")
                .child(div().flex().flex_col().flex_shrink_0().h(px((count.clamp(1, 8) as f32) * 32.)).w_full().track_focus(&controls[0]).child(rows)))
            .footer(row().gap_2()
                .child(crate::dialog::button("checkout-branch-confirm", "확인", true, prompt.selected < count).track_focus(&controls[1])
                    .on_click(cx.listener(|this, _, window, cx| this.choose_branch_checkout(true, window, cx))))
                .child(crate::dialog::button("checkout-branch-cancel", "취소", false, true).track_focus(&controls[2])
                    .on_click(cx.listener(|this, _, window, cx| this.choose_branch_checkout(false, window, cx)))))
            .build())
    }

    fn take_confirmed_checkout(&mut self, confirm: bool) -> Option<String> {
        let (project, path, reference) = self.checkout_confirmation.take()?;
        (confirm
            && project == self.project
            && self.footer_directory().as_ref() == Some(&path)
            && !self.repository_busy()
            && self.alert.is_none())
        .then_some(reference)
    }

    fn choose_checkout(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        let reference = self.take_confirmed_checkout(confirm);
        self.checkout_focus = None;
        self.checkout_buttons = None;
        if let Some(focus) = self.checkout_previous_focus.take() {
            focus.focus(window);
        }
        if let Some(reference) = reference {
            self.execute_toolbar_action(
                workspace_backend::ToolbarAction::Checkout { reference },
                window,
                cx,
            );
        }
        cx.notify();
    }

    fn checkout_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let (_, _, reference) = self.checkout_confirmation.as_ref()?;
        let focus = self.checkout_focus.as_ref()?;
        let buttons = self.checkout_buttons.as_ref()?;
        Some(crate::dialog::Dialog::new("checkout-confirmation", "Detached HEAD로 전환할까요?")
            .track_focus(focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "enter" => { let confirm = !this.checkout_buttons.as_ref().is_some_and(|buttons| buttons[1].is_focused(window)); this.choose_checkout(confirm, window, cx); },
                    "escape" => this.choose_checkout(false, window, cx),
                    "tab" => {
                        if let Some(buttons) = &this.checkout_buttons {
                            let next = match buttons.iter().position(|focus| focus.is_focused(window)) {
                                Some(index) => 1 - index,
                                None => usize::from(event.keystroke.modifiers.shift),
                            };
                            buttons[next].focus(window);
                        }
                    },
                    _ => {}
                }
                cx.stop_propagation();
            }))
            .body(div().flex().flex_col().gap_3().child(reference.clone())
                .child("브랜치에 연결되지 않은 상태로 전환합니다. 이 상태에서 만든 커밋을 유지하려면 새 브랜치를 만들어야 합니다. 계속할까요?"))
            .footer(row().gap_2()
                .child(crate::dialog::button("checkout-confirm", "확인", true, true).track_focus(&buttons[0])
                    .on_click(cx.listener(|this, _, window, cx| this.choose_checkout(true, window, cx))))
                .child(crate::dialog::button("checkout-cancel", "취소", false, true).track_focus(&buttons[1])
                    .on_click(cx.listener(|this, _, window, cx| this.choose_checkout(false, window, cx)))))
            .build())
    }

    fn take_confirmed_merge_abort(&mut self, confirm: bool) -> Option<String> {
        let (project, path, reference) = self.merge_abort_confirmation.take()?;
        (confirm
            && project == self.project
            && self.footer_directory().as_ref() == Some(&path)
            && self.operation_action_available(self.abort_operation, false)
            && self.projects[self.project].head.as_ref() == Some(&reference)
            && self.alert.is_none())
        .then_some(reference)
    }

    fn choose_merge_abort(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        let operation = self.abort_operation;
        let reference = self.take_confirmed_merge_abort(confirm);
        self.abort_operation = AbortOperation::Merge;
        self.merge_abort_focus = None;
        self.merge_abort_buttons = None;
        if let Some(focus) = self.merge_abort_previous_focus.take() {
            focus.focus(window);
        }
        if let Some(reference) = reference {
            self.execute_toolbar_action(
                match operation { AbortOperation::Revert => workspace_backend::ToolbarAction::AbortRevert { expected_head: reference }, AbortOperation::Rebase => workspace_backend::ToolbarAction::AbortRebase { expected_head: reference }, AbortOperation::Merge => workspace_backend::ToolbarAction::AbortMerge { expected_head: reference } },
                window,
                cx,
            );
        }
        cx.notify();
    }

    fn merge_abort_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let _pending = self.merge_abort_confirmation.as_ref()?;
        let focus = self.merge_abort_focus.as_ref()?;
        let buttons = self.merge_abort_buttons.as_ref()?;
        let (operation, title) = match self.abort_operation {
            AbortOperation::Merge => ("병합", "병합을 취소할까요?"),
            AbortOperation::Rebase => ("리베이스", "리베이스를 취소할까요?"),
            AbortOperation::Revert => ("Revert", "Revert를 취소할까요?"),
        };
        Some(crate::dialog::Dialog::new("merge_abort-confirmation", title)
            .track_focus(focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "enter" => { let confirm = this.merge_abort_buttons.as_ref().is_some_and(|buttons| buttons[0].is_focused(window)); this.choose_merge_abort(confirm, window, cx); },
                    "escape" => this.choose_merge_abort(false, window, cx),
                    "tab" => {
                        if let Some(buttons) = &this.merge_abort_buttons {
                            let next = match buttons.iter().position(|focus| focus.is_focused(window)) {
                                Some(index) => 1 - index,
                                None => usize::from(event.keystroke.modifiers.shift),
                            };
                            buttons[next].focus(window);
                        }
                    },
                    _ => {}
                }
                cx.stop_propagation();
            }))
            .body(div().flex().flex_col().gap_3()
                .child(format!("{operation} 전 상태로 돌아갑니다. 작업 결과와 충돌 해결 내용이 제거될 수 있습니다. 계속할까요?")))
            .footer(row().gap_2()
                .child(crate::dialog::button("merge_abort-confirm", "확인", true, true).track_focus(&buttons[0])
                    .on_click(cx.listener(|this, _, window, cx| this.choose_merge_abort(true, window, cx))))
                .child(crate::dialog::button("merge_abort-cancel", "취소", false, true).track_focus(&buttons[1])
                    .on_click(cx.listener(|this, _, window, cx| this.choose_merge_abort(false, window, cx)))))
            .build())
    }

    fn execute_toolbar_action(
        &mut self,
        action: workspace_backend::ToolbarAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.sync_busy || self.commit_busy || self.alert.is_some() || !window.is_window_active()
        {
            return;
        }
        match &action {
            workspace_backend::ToolbarAction::ContinueRevert { .. }
            | workspace_backend::ToolbarAction::AbortRevert { .. }
            | workspace_backend::ToolbarAction::Refresh { .. }
            | workspace_backend::ToolbarAction::RevertCommit { .. }
            | workspace_backend::ToolbarAction::ResetCommit { .. }
            | workspace_backend::ToolbarAction::CheckoutCommitBranch { .. }
            | workspace_backend::ToolbarAction::RebaseCommit { .. }
            | workspace_backend::ToolbarAction::ContinueRebase { .. }
            | workspace_backend::ToolbarAction::AbortRebase { .. }
            | workspace_backend::ToolbarAction::AbortMerge { .. }
            | workspace_backend::ToolbarAction::MergeCommit { .. }
            | workspace_backend::ToolbarAction::Push
            | workspace_backend::ToolbarAction::PushWithLease
            | workspace_backend::ToolbarAction::ApplyStash { .. }
            | workspace_backend::ToolbarAction::DeleteReference { .. }
            | workspace_backend::ToolbarAction::Checkout { .. } => {
                if !self.sync_available(cx) {
                    return;
                }
            }
            workspace_backend::ToolbarAction::CreateTag { id, .. } => {
                if self.branch_dialog.is_none() || !self.tag_dialog_context(id) {
                    return;
                }
            }
            workspace_backend::ToolbarAction::CreateStash { .. } => {
                if self.branch_dialog.is_none() || !self.stash_dialog_context() {
                    return;
                }
            }
            workspace_backend::ToolbarAction::CreateBranch { .. }
            | workspace_backend::ToolbarAction::CreateTrackingBranch { .. } => {
                if self.branch_dialog.is_none()
                    || !self.branch_owner.as_ref().is_some_and(|(project, path)| {
                        *project == self.project && self.footer_directory().as_ref() == Some(path)
                    })
                {
                    return;
                }
            }
        }
        let Some(path) = self.footer_directory() else {
            return;
        };
        self.invalidate_requests();
        self.cancel_list_drags();
        self.history_append_reads.cancel();
        self.history_append_pending = None;
        self.history_append_inflight = None;
        self.sync_revision = self.sync_revision.wrapping_add(1);
        self.sync_busy = true;
        if let Some(dialog) = self.branch_dialog.clone() {
            dialog.update(cx, |dialog, cx| dialog.set_busy(true, cx));
        }
        self.progress_previous_focus = window.focused(cx);
        let focus = cx.focus_handle();
        focus.focus(window);
        self.progress_focus = Some(focus);
        self.sync_operation = Some(match &action {
            workspace_backend::ToolbarAction::ContinueRevert { .. } => "Revert 계속 중…",
            workspace_backend::ToolbarAction::AbortRevert { .. } => "Revert 취소 중…",
            workspace_backend::ToolbarAction::Refresh { .. } => "로컬 저장소 새로고침 중…",
            workspace_backend::ToolbarAction::RevertCommit { .. } => "Revert 실행 중…",
            workspace_backend::ToolbarAction::ResetCommit { .. } => "Reset 실행 중…",
            workspace_backend::ToolbarAction::RebaseCommit { .. } => "리베이스 실행 중…",
            workspace_backend::ToolbarAction::ContinueRebase { .. } => "리베이스 계속 중…",
            workspace_backend::ToolbarAction::AbortRebase { .. } => "리베이스 취소 중…",
            workspace_backend::ToolbarAction::AbortMerge { .. } => "병합 취소 중…",
            workspace_backend::ToolbarAction::MergeCommit { .. } => "병합 실행 중…",
            workspace_backend::ToolbarAction::Push | workspace_backend::ToolbarAction::PushWithLease => "Push 실행 중…",
            workspace_backend::ToolbarAction::ApplyStash { .. } => "스태시 적용 중…",
            workspace_backend::ToolbarAction::CreateStash { .. } => "스태시 생성 중…",
            workspace_backend::ToolbarAction::CreateTag { .. } => "태그 생성 중…",
            workspace_backend::ToolbarAction::DeleteReference { .. } => "참조 삭제 중…",
            workspace_backend::ToolbarAction::Checkout { .. } | workspace_backend::ToolbarAction::CheckoutCommitBranch { .. } => "체크아웃 실행 중…",
            workspace_backend::ToolbarAction::CreateBranch { .. }
            | workspace_backend::ToolbarAction::CreateTrackingBranch { .. } => "브랜치 생성 중…",
        });
        let request = workspace_backend::ToolbarRequest {
            revision: self.sync_revision,
            project: self.project,
            path,
            name: self.projects[self.project].name.clone(),
            action,
        };
        let task = crate::backend::spawn(cx.background_executor(), move || request.execute());
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let refreshing = matches!(&result.request.action, workspace_backend::ToolbarAction::Refresh { .. });
            let reverting = matches!(&result.request.action, workspace_backend::ToolbarAction::RevertCommit { .. } | workspace_backend::ToolbarAction::ContinueRevert { .. } | workspace_backend::ToolbarAction::AbortRevert { .. });
            let resetting = matches!(&result.request.action, workspace_backend::ToolbarAction::ResetCommit { .. });
            let rebasing = matches!(&result.request.action, workspace_backend::ToolbarAction::RebaseCommit { .. } | workspace_backend::ToolbarAction::ContinueRebase { .. } | workspace_backend::ToolbarAction::AbortRebase { .. });
            let aborting = matches!(&result.request.action, workspace_backend::ToolbarAction::AbortMerge { .. });
            let merging = matches!(&result.request.action, workspace_backend::ToolbarAction::MergeCommit { .. });
            let checkout = matches!(
                &result.request.action,
                workspace_backend::ToolbarAction::Checkout { .. } | workspace_backend::ToolbarAction::CheckoutCommitBranch { .. }
            );
            let deleting = matches!(
                &result.request.action,
                workspace_backend::ToolbarAction::DeleteReference { .. }
            );
            let applying = matches!(
                &result.request.action,
                workspace_backend::ToolbarAction::ApplyStash { .. }
            );
            let stashing = matches!(
                &result.request.action,
                workspace_backend::ToolbarAction::CreateStash { .. }
            );
            let tagging = matches!(
                &result.request.action,
                workspace_backend::ToolbarAction::CreateTag { .. }
            );
            let branch = matches!(
                &result.request.action,
                workspace_backend::ToolbarAction::CreateBranch { .. }
                    | workspace_backend::ToolbarAction::CreateTrackingBranch { .. }
                    | workspace_backend::ToolbarAction::CreateTag { .. }
                    | workspace_backend::ToolbarAction::CreateStash { .. }
            );
            let _ = this.update_in(cx, |this, window, cx| {
                let conflict_paths = this.toolbar_failure_paths(&result);
                let Some((completed, error)) = this.finish_toolbar_action(result) else {
                    return;
                };
                this.progress_focus = None;
                if refreshing && completed && this.repository_tab == RepositoryTab::Search && this.search_owner == Some(this.project)
                    && let Some(panel) = this.search_panel.clone() {
                    panel.update(cx, |panel, cx| { panel.set_enabled(true, true, cx); panel.refresh(window, cx); });
                }
                let previous = this.progress_previous_focus.take();
                if let Some(dialog) = this.branch_dialog.clone() {
                    dialog.update(cx, |dialog, cx| dialog.set_busy(false, cx));
                }
                if branch && completed {
                    this.close_branch_dialog(window, cx);
                } else if let Some(previous) = previous {
                    previous.focus(window);
                } else {
                    window.blur();
                }
                if let Some(error) = error {
                    if branch
                        && !completed
                        && let Some(dialog) = this.branch_dialog.clone()
                    {
                        dialog.update(cx, |dialog, cx| dialog.fail(error, cx));
                    } else {
                        this.show_alert(
                            if refreshing { "로컬 새로고침 실패" } else if reverting { "Revert 실패" } else if resetting { "Reset 실패" } else if applying {
                                "스태시 적용 실패"
                            } else if stashing {
                                "Stash 작업 안내"
                            } else if tagging {
                                "태그 작업 안내"
                            } else if branch {
                                "브랜치 작업 안내"
                            } else if deleting {
                                "삭제 실패"
                            } else if rebasing {
                                "리베이스 작업 실패"
                            } else if aborting {
                                "병합 취소 실패"
                            } else if merging {
                                "병합 실패"
                            } else if checkout {
                                "체크아웃 실패"
                            } else {
                                "Push 실패"
                            },
                            &error,
                            window,
                            cx,
                        );
                        if (applying || merging || rebasing || reverting) && let Some(alert) = &mut this.alert {
                            alert.set_paths(conflict_paths);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn toolbar_failure_paths(&self, result: &workspace_backend::ToolbarResult) -> Vec<String> {
        if result.request.revision != self.sync_revision
            || result.request.project != self.project
            || self.footer_directory().as_ref() != Some(&result.request.path)
            || !matches!(
                result.request.action,
                workspace_backend::ToolbarAction::ContinueRevert { .. } | workspace_backend::ToolbarAction::AbortRevert { .. } | workspace_backend::ToolbarAction::RevertCommit { .. } | workspace_backend::ToolbarAction::ApplyStash { .. } | workspace_backend::ToolbarAction::MergeCommit { .. } | workspace_backend::ToolbarAction::RebaseCommit { .. } | workspace_backend::ToolbarAction::ContinueRebase { .. }
            )
        {
            return Vec::new();
        }
        result
            .conflict_paths
            .iter()
            .map(|path| display_path(path))
            .collect()
    }

    fn finish_toolbar_action(
        &mut self,
        result: workspace_backend::ToolbarResult,
    ) -> Option<(bool, Option<String>)> {
        if result.request.revision != self.sync_revision {
            return None;
        }
        self.sync_busy = false;
        self.sync_operation = None;
        if self.project != result.request.project
            || self.footer_directory().as_ref() != Some(&result.request.path)
        {
            return Some((false, Some("작업 대상 저장소가 변경되었습니다.".into())));
        }
        let (completed, mut errors) = match result.outcome {
            Ok(error) => (true, error.into_iter().collect::<Vec<_>>()),
            Err(error) => (false, vec![error]),
        };
        let refresh_selection = match &result.request.action {
            workspace_backend::ToolbarAction::Refresh { selected_files } => Some(selected_files.clone()), _ => None,
        };
        match result.refreshed {
            Ok((project, history)) => {
                self.apply_refreshed_project(project, history);
                if let Some(selected) = refresh_selection {
                    if let Some(request) = &mut self.panel_pending { request.selected_files = selected; }
                    self.viewer_refresh_pending = true;
                }
            },
            Err(error) => errors.push(error),
        }
        Some((completed, (!errors.is_empty()).then(|| errors.join("\n"))))
    }
    fn toolbar(&self, cx: &mut Context<Self>) -> Div {
        let execute_owner = cx.weak_entity();
        let select_owner = cx.weak_entity();
        let enabled = self.footer_directory().is_some() && self.chrome_actions_enabled(cx);
        let selected = repository::SyncMode::OPTIONS
            .iter()
            .position(|mode| *mode == self.selected_sync_mode)
            .unwrap_or(1);
        row()
            .h(px(60.))
            .flex_shrink_0()
            .px_3()
            .py(px(3.))
            .gap_2()
            .bg(rgb(PANEL))
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                crate::split_button::toolbar_action(
                    "commit-button",
                    "Commit",
                    crate::split_button::commit_icon(),
                    enabled,
                )
                .rounded_md()
                .overflow_hidden()
                .when(!enabled, |button| button.opacity(0.45))
                .on_click(cx.listener(|this, _, window, cx| this.open_commit_dialog(window, cx))),
            )
            .child(
                crate::split_button::SplitButton::new(
                    "pull-split-button",
                    if self.selected_sync_mode == repository::SyncMode::Fetch {
                        "Fetch"
                    } else {
                        "Pull"
                    },
                    crate::split_button::pull_icon(),
                    repository::SyncMode::OPTIONS
                        .into_iter()
                        .map(|mode| mode.label().into())
                        .collect(),
                    selected,
                )
                .enabled(self.sync_available(cx))
                .appearance_enabled(enabled)
                .busy(self.sync_busy)
                .on_execute(move |window, cx| {
                    let _ = execute_owner.update(cx, |this, cx| {
                        this.start_sync(this.selected_sync_mode, window, cx)
                    });
                })
                .on_select(move |index, _, cx| {
                    let Some(mode) = repository::SyncMode::OPTIONS.get(index).copied() else {
                        return;
                    };
                    let _ = select_owner.update(cx, |this, cx| {
                        if this.sync_available(cx) && this.choose_sync_mode(mode) {
                            cx.notify();
                        }
                    });
                }),
            )
            .child(
                crate::split_button::toolbar_action(
                    "push-button",
                    "Push",
                    crate::split_button::push_icon(),
                    enabled,
                )
                .rounded_md()
                .overflow_hidden()
                .when(!enabled, |button| button.opacity(0.45))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.start_toolbar_action(workspace_backend::ToolbarAction::Push, window, cx)
                })),
            )
            .child(
                crate::split_button::toolbar_action(
                    "branch-button",
                    "Branch",
                    crate::split_button::branch_icon(),
                    enabled,
                )
                .rounded_md()
                .overflow_hidden()
                .when(!enabled, |button| button.opacity(0.45))
                .on_click(cx.listener(|this, _, window, cx| this.open_branch_dialog(window, cx))),
            )
            .child(
                crate::split_button::toolbar_action(
                    "stash-button",
                    "Stash",
                    crate::split_button::stash_icon(),
                    enabled,
                )
                .rounded_md()
                .overflow_hidden()
                .when(!enabled, |button| button.opacity(0.45))
                .on_click(cx.listener(|this, _, window, cx| this.open_stash_dialog(window, cx))),
            )
    }
    fn display_repository_names(&self) -> &[String] {
        self.retained_repository_names.as_deref().unwrap_or_else(|| self.repository_names())
    }

    fn display_file_list(&self) -> (&[ChangedFile], bool) {
        self.retained_file_list.as_ref().map_or_else(
            || (self.changed_files(), self.file_list_has_detail()),
            |(files, detail)| (files.as_slice(), *detail),
        )
    }

    fn repository_names(&self) -> &[String] {
        let project = &self.projects[self.project];
        match self.repository_tab {
            RepositoryTab::Local => &project.branches,
            RepositoryTab::Remote => &project.remotes,
            _ => self.panel_references.as_deref().unwrap_or(&[]),
        }
    }

    fn changed_files(&self) -> &[ChangedFile] {
        if self.file_history {
            self.history_files.as_deref().unwrap_or(&[])
        } else {
            &self.projects[self.project].files
        }
    }

    fn select_repository_row(
        &mut self,
        project_id: usize,
        tab: RepositoryTab,
        index: usize,
    ) -> bool {
        if self.repository_waiting || self.repository_busy()
            || self.project != project_id
            || self.repository_tab != tab
            || self.start_tab_active
            || !self.open_projects.contains(&project_id)
        {
            return false;
        }
        let Some(name) = self.repository_names().get(index).cloned() else {
            return false;
        };
        if !self
            .repository_selection
            .select(index, self.repository_names().len())
        {
            return false;
        }
        self.reference_reads.cancel();
        self.commit_loading.reset();
        self.reference_revision = self.reference_revision.wrapping_add(1);
        self.reference_pending = None;
        let Some(full_name) = tab.reference_name(&name) else {
            return true;
        };
        self.reference_reads.cancel();
        self.commit_loading.reset();
        self.reference_revision = self.reference_revision.wrapping_add(1);
        self.reference_pending = None;
        let target = self.projects[project_id].commits.iter().position(|commit| {
            commit
                .references
                .iter()
                .any(|reference| reference.full_name == full_name)
        });
        if let Some(target) = target {
            self.focus_reference_commit(project_id, target, index);
        } else if let Some(path) = self.projects[project_id].path.clone() {
            self.reference_pending = Some(ReferenceRequest {
                revision: self.reference_revision,
                project: project_id,
                path,
                name: self.projects[project_id].name.clone(),
                full_name,
                row: index,
                tab: tab as usize,
            });
            self.commit_loading
                .begin(self.reference_revision, std::time::Instant::now());
        } else {
            self.notice = format!("{name}: 저장소 경로가 없습니다.");
        }
        true
    }

    fn focus_reference_commit(&mut self, project: usize, target: usize, row: usize) {
        let references = self.panel_references.clone();
        let count = self.repository_names().len();
        if self.select_commit(project, target, false) {
            self.panel_references = references;
            self.repository_selection.select(row, count);
            self.histories[&project]
                .scroll
                .scroll_to_item(target, gpui::ScrollStrategy::Center);
            self.update_tab_notice();
        }
    }

    fn finish_reference(
        &mut self,
        request: ReferenceRequest,
        result: Result<(Project, PreparedHistory), String>,
    ) -> bool {
        if request.revision != self.reference_revision
            || request.project != self.project
            || self.start_tab_active
            || !self.open_projects.contains(&request.project)
            || self.repository_tab as usize != request.tab
            || self.projects[self.project].path.as_ref() != Some(&request.path)
            || self
                .repository_names()
                .get(request.row)
                .and_then(|name| self.repository_tab.reference_name(name))
                .as_ref()
                != Some(&request.full_name)
        {
            return false;
        }
        self.commit_loading.reset();
        match result {
            Ok((project, history)) => {
                let Some(target) = project.commits.iter().position(|commit| {
                    commit
                        .references
                        .iter()
                        .any(|reference| reference.full_name == request.full_name)
                }) else {
                    self.notice = "대상 커밋을 찾을 수 없습니다.".into();
                    return true;
                };
                self.projects[request.project].commits = project.commits;
                self.projects[request.project].more_commits = project.more_commits;
                self.projects[request.project].history_cursor = project.history_cursor;
                self.install_history(request.project, history);
                self.focus_reference_commit(request.project, target, request.row);
            }
            Err(error) => self.notice = format!("{}: {error}", request.full_name),
        }
        true
    }

    fn repository_checkout_target(
        &self,
        project: usize,
        tab: RepositoryTab,
        index: usize,
        click_count: usize,
    ) -> Option<String> {
        if click_count != 2
            || self.repository_busy()
            || self.project != project
            || self.repository_tab != tab
            || self.start_tab_active
            || !self.open_projects.contains(&project)
        {
            return None;
        }
        self.repository_names()
            .get(index)
            .and_then(|name| tab.reference_name(name))
    }
    fn repository_row_mouse_down(
        &mut self,
        project: usize,
        tab: RepositoryTab,
        index: usize,
        click_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if click_count == 2 {
            if let Some(reference) =
                self.repository_checkout_target(project, tab, index, click_count)
            {
                self.start_toolbar_action(
                    workspace_backend::ToolbarAction::Checkout { reference },
                    window,
                    cx,
                );
            }
        } else if self.select_repository_row(project, tab, index) {
            cx.notify();
        }
    }
    fn checkout_menu_target(&self, row: (usize, Option<RepositoryTab>, usize)) -> Option<String> {
        let (project, tab, index) = row;
        if self.repository_busy()
            || self.project != project
            || self.start_tab_active
            || !self.open_projects.contains(&project)
        {
            return None;
        }
        if let Some(tab) = tab {
            if self
                .repository_selection
                .selected(self.repository_names().len())
                != Some(index)
            {
                return None;
            }
            self.repository_checkout_target(project, tab, index, 2)
        } else {
            if self.commit != index || self.commit_grid_loading().is_some() {
                return None;
            }
            self.projects
                .get(project)?
                .commits
                .get(index)
                .map(|commit| commit.id.clone())
        }
    }

    fn repository_delete_target(
        &self,
        project: usize,
        tab: RepositoryTab,
        index: usize,
    ) -> Option<String> {
        if self.repository_busy()
            || self.alert.is_some()
            || self.repository_dialog.is_some()
            || self.project != project
            || self.repository_tab != tab
            || self.start_tab_active
            || !self.open_projects.contains(&project)
            || self
                .repository_selection
                .selected(self.repository_names().len())
                != Some(index)
        {
            return None;
        }
        let name = self.repository_names().get(index)?;
        if tab == RepositoryTab::Stashes {
            let ordinal: usize = name.split_once(": ")?.0.parse().ok()?;
            if ordinal != index {
                return None;
            }
            Some(format!("stash@{{{ordinal}}}"))
        } else {
            tab.reference_name(name)
        }
    }

    fn stash_apply_target(&self, project: usize, index: usize) -> Option<(String, String)> {
        let reference = self.repository_delete_target(project, RepositoryTab::Stashes, index)?;
        Some((reference, self.repository_names().get(index)?.clone()))
    }

    fn prepare_stash_apply(
        &mut self,
        project: usize,
        index: usize,
        target: (String, String),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.footer_directory() else {
            return;
        };
        self.stash_apply_revision = self.stash_apply_revision.wrapping_add(1);
        let revision = self.stash_apply_revision;
        let owner = path.clone();
        let reference = target.0.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            repository::prepare_reference_deletion(&owner, &reference)
                .map_err(|error| format!("{error:#}"))
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if revision != this.stash_apply_revision
                    || !window.is_window_active()
                    || this.footer_directory().as_ref() != Some(&path)
                    || this.stash_apply_target(project, index).as_ref() != Some(&target)
                {
                    return;
                }
                match result {
                    Ok(request) => {
                        let display = repository::stash_display_label(
                            index,
                            request.description.as_deref().unwrap_or_default(),
                        );
                        if display != target.1 {
                            this.show_alert(
                                "스태시 적용 실패",
                                "스태시 목록이 변경되었습니다. 목록을 갱신한 뒤 다시 적용하세요.",
                                window,
                                cx,
                            );
                            return;
                        }
                        this.start_toolbar_action(
                            workspace_backend::ToolbarAction::ApplyStash { request },
                            window,
                            cx,
                        );
                    }
                    Err(error) => this.show_alert("스태시 적용 실패", &error, window, cx),
                }
            });
        })
        .detach();
    }

    fn history_rebase_branches(&self, row: (usize, Option<RepositoryTab>, usize)) -> Vec<String> {
        let Some(_) = self.history_rebase_request(row, None) else { return Vec::new(); };
        let project = &self.projects[row.0];
        let current = format!("refs/heads/{}", project.branch);
        let mut branches: Vec<_> = project.commits[row.2].references.iter()
            .filter(|reference| reference.full_name.starts_with("refs/heads/") && reference.full_name != current)
            .map(|reference| reference.full_name.clone()).collect();
        branches.sort(); branches.dedup(); branches
    }

    fn history_rebase_request(&self, row: (usize, Option<RepositoryTab>, usize), reverse: Option<&str>) -> Option<repository::RebaseRequest> {
        if row.1.is_some() { return None; }
        let target = self.checkout_menu_target(row)?;
        let project = self.projects.get(row.0)?;
        if project.work_state.error.is_some() || project.work_state.operation.is_some()
            || !project.branches.contains(&project.branch) { return None; }
        let reference = format!("refs/heads/{}", project.branch);
        if let Some(reverse) = reverse
            && (reverse == reference || !reverse.starts_with("refs/heads/")
                || !project.commits[row.2].references.iter().any(|entry| entry.full_name == reverse)) { return None; }
        Some(repository::RebaseRequest { target, expected_head: project.head.clone()?, expected_reference: reference,
            reverse_branch: reverse.map(str::to_owned) })
    }

    fn history_rebase_menu(&self, row: (usize, Option<RepositoryTab>, usize), cx: &mut Context<Self>) -> crate::context_menu::MenuItem {
        let path = self.projects[row.0].path.clone();
        let item = |label: String, request: Option<repository::RebaseRequest>| {
            let owner = cx.weak_entity(); let path = path.clone();
            crate::context_menu::MenuItem::new(label, "", request.is_some(), move |window, cx| {
                let Some(request) = &request else { return; };
                let _ = owner.update(cx, |this, cx| {
                    if window.is_window_active()
                        && this.projects.get(row.0).is_some_and(|project| project.path == path)
                        && this.history_rebase_request(row, request.reverse_branch.as_deref()).as_ref() == Some(request) {
                        this.start_toolbar_action(workspace_backend::ToolbarAction::RebaseCommit { request: request.clone() }, window, cx);
                    }
                });
            })
        };
        let forward = item("정방향 (현재를 대상 브랜치로)".into(), self.history_rebase_request(row, None));
        let branches = self.history_rebase_branches(row);
        let reverse = if branches.len() > 1 {
            crate::context_menu::MenuItem::submenu("역방향 (대상을 현재 브랜치로)", branches.iter().map(|branch| {
                item(branch.trim_start_matches("refs/heads/").to_owned(), self.history_rebase_request(row, Some(branch)))
            }).collect())
        } else {
            item("역방향 (대상을 현재 브랜치로)".into(), branches.first().and_then(|branch| self.history_rebase_request(row, Some(branch))))
        };
        crate::context_menu::MenuItem::submenu("Rebase", vec![forward, reverse])
    }
    fn open_checkout_menu(
        &mut self,
        row: (usize, Option<RepositoryTab>, usize),
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (project, tab, index) = row;
        if self.repository_busy() || self.project != project || self.start_tab_active {
            return;
        }
        if let Some(tab) = tab {
            if self.repository_tab != tab || index >= self.repository_names().len() {
                return;
            }
            self.select_repository_row(project, tab, index);
        } else {
            self.select_history_row(project, index);
        }
        let path = self.projects[project].path.clone();
        let mut items = Vec::new();
        if let Some(reference) = self.checkout_menu_target(row) {
            let owner = cx.weak_entity();
            let path = path.clone();
            items.push(crate::context_menu::MenuItem::new(
                "체크아웃",
                "",
                true,
                move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if window.is_window_active()
                            && this
                                .projects
                                .get(project)
                                .is_some_and(|project| project.path == path)
                            && this.checkout_menu_target(row).as_ref() == Some(&reference)
                        {
                            if tab.is_none() { this.start_history_checkout(row, reference.clone(), window, cx); }
                            else { this.start_toolbar_action(workspace_backend::ToolbarAction::Checkout { reference: reference.clone() }, window, cx); }
                        }
                    });
                },
            ));
        }
        if tab.is_none()
            && let Some(id) = self.checkout_menu_target(row)
        {
            let owner = cx.weak_entity();
            let path = path.clone();
            items.push(crate::context_menu::MenuItem::new(
                "태그 추가",
                "",
                true,
                move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if window.is_window_active()
                            && this
                                .projects
                                .get(project)
                                .is_some_and(|project| project.path == path)
                            && this.checkout_menu_target(row).as_ref() == Some(&id)
                        {
                            this.open_tag_dialog(project, index, id.clone(), window, cx);
                        }
                    });
                },
            ));
        }
        if tab.is_none() && let Some(id) = self.checkout_menu_target(row) {
            let owner = cx.weak_entity();
            let path = path.clone();
            items.push(crate::context_menu::MenuItem::new("Merge", "", true, move |window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if window.is_window_active()
                        && this.projects.get(project).is_some_and(|project| project.path == path)
                        && this.checkout_menu_target(row).as_ref() == Some(&id)
                    {
                        this.start_toolbar_action(workspace_backend::ToolbarAction::MergeCommit { id: id.clone() }, window, cx);
                    }
                });
            }));
        }
        if tab.is_none() {
            let request = self.history_action_request(row);
            for (label, reset) in [("Revert", false), ("Reset", true)] {
                let owner = cx.weak_entity(); let expected_path = path.clone(); let request = request.clone();
                items.push(crate::context_menu::MenuItem::new(label, "", request.is_some(), move |window, cx| {
                    let Some(request) = &request else { return; };
                    let _ = owner.update(cx, |this, cx| {
                        if window.is_window_active() && this.footer_directory() == expected_path && this.history_action_request(row).as_ref() == Some(request) {
                            let action = if reset { workspace_backend::ToolbarAction::ResetCommit { request: request.clone(), mode: repository::ResetMode::Mixed } }
                                else { workspace_backend::ToolbarAction::RevertCommit { request: request.clone(), no_commit: false } };
                            this.start_toolbar_action(action, window, cx);
                        }
                    });
                }));
            }
        }
        if tab.is_none() { items.push(self.history_rebase_menu(row, cx)); }
        if tab == Some(RepositoryTab::Stashes)
            && let Some(target) = self.stash_apply_target(project, index)
        {
            let owner = cx.weak_entity();
            let path = path.clone();
            items.push(crate::context_menu::MenuItem::new(
                "스태시 적용",
                "",
                true,
                move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if window.is_window_active()
                            && this
                                .projects
                                .get(project)
                                .is_some_and(|project| project.path == path)
                            && this.stash_apply_target(project, index).as_ref() == Some(&target)
                        {
                            this.prepare_stash_apply(project, index, target.clone(), window, cx);
                        }
                    });
                },
            ));
        }
        if let Some(tab) = tab
            && let Some(reference) = self.repository_delete_target(project, tab, index)
        {
            let owner = cx.weak_entity();
            let enabled = !(tab == RepositoryTab::Local
                && self.repository_names()[index] == self.projects[project].branch);
            items.push(crate::context_menu::MenuItem::new(
                "삭제",
                "",
                enabled,
                move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if window.is_window_active()
                            && this
                                .projects
                                .get(project)
                                .is_some_and(|project| project.path == path)
                            && this.repository_delete_target(project, tab, index).as_ref()
                                == Some(&reference)
                        {
                            this.prepare_reference_delete(
                                project,
                                tab,
                                index,
                                reference.clone(),
                                window,
                                cx,
                            );
                        }
                    });
                },
            ));
        }
        if !items.is_empty() {
            crate::context_menu::open(position, items, window, cx);
        }
        cx.notify();
        cx.stop_propagation();
    }

    fn prepare_reference_delete(
        &mut self,
        project: usize,
        tab: RepositoryTab,
        index: usize,
        reference: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.footer_directory() else {
            return;
        };
        self.reference_delete_revision = self.reference_delete_revision.wrapping_add(1);
        let revision = self.reference_delete_revision;
        let owner = path.clone();
        let target = reference.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            repository::prepare_reference_deletion(&owner, &target)
                .map_err(|error| format!("{error:#}"))
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if revision != this.reference_delete_revision
                    || !window.is_window_active()
                    || this.footer_directory().as_ref() != Some(&path)
                    || this.repository_delete_target(project, tab, index).as_ref()
                        != Some(&reference)
                {
                    return;
                }
                match result {
                    Ok(request) => {
                        this.reference_delete_previous_focus = window.focused(cx);
                        let handles = std::array::from_fn(|_| cx.focus_handle());
                        handles[0].focus(window);
                        this.reference_delete_focus = Some(handles);
                        this.reference_delete_remote_tag = false;
                        this.reference_delete = Some(ReferenceDeletePrompt {
                            project,
                            path,
                            tab,
                            index,
                            request,
                        });
                        cx.notify();
                    }
                    Err(error) => this.show_alert("삭제 대상 확인 실패", &error, window, cx),
                }
            });
        })
        .detach();
    }

    fn take_reference_delete(&mut self, confirm: bool) -> Option<repository::ReferenceDeletion> {
        let prompt = self.reference_delete.take()?;
        (confirm
            && self.footer_directory().as_ref() == Some(&prompt.path)
            && self
                .repository_delete_target(prompt.project, prompt.tab, prompt.index)
                .as_ref()
                == Some(&prompt.request.reference))
        .then_some(prompt.request)
    }

    fn take_reference_delete_action(
        &mut self,
        confirm: bool,
    ) -> Option<workspace_backend::ToolbarAction> {
        let remote_tag = std::mem::take(&mut self.reference_delete_remote_tag);
        self.take_reference_delete(confirm).map(|request| {
            workspace_backend::ToolbarAction::DeleteReference {
                remote_tag: remote_tag && request.reference.starts_with("refs/tags/"),
                request,
            }
        })
    }

    fn choose_reference_delete(
        &mut self,
        confirm: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let action = self.take_reference_delete_action(confirm);
        self.reference_delete_focus = None;
        if let Some(previous) = self.reference_delete_previous_focus.take() {
            previous.focus(window);
        }
        if let Some(action) = action {
            self.start_toolbar_action(action, window, cx);
        }
        cx.notify();
    }

    fn reference_delete_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let prompt = self.reference_delete.as_ref()?;
        let handles = self.reference_delete_focus.as_ref()?;
        let message = match prompt.tab {
            RepositoryTab::Local => {
                "로컬 브랜치를 삭제합니다. 현재 체크아웃되었거나 병합되지 않은 브랜치는 삭제할 수 없습니다. 계속할까요?"
            }
            RepositoryTab::Remote => {
                "원격 서버의 브랜치를 삭제하고 이 PC의 원격 추적 정보도 정리합니다. 다른 사용자에게도 영향을 줍니다. 계속할까요?"
            }
            RepositoryTab::Tags => {
                "이 저장소의 로컬 태그를 삭제합니다. 원격 서버의 태그는 유지됩니다. 계속할까요?"
            }
            RepositoryTab::Search => return None,
            RepositoryTab::Stashes => {
                "선택한 스태시를 삭제합니다. 저장된 변경을 복구하지 못할 수 있습니다. 계속할까요?"
            }
        };
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(prompt.request.reference.clone())
            .child(prompt.request.expected.clone())
            .when_some(prompt.request.description.clone(), |body, description| {
                body.child(description)
            })
            .when(prompt.tab == RepositoryTab::Tags, |body| body.child(
                crate::form_controls::checkbox("delete-remote-tag", self.reference_delete_remote_tag, "원격 태그 삭제", true)
                    .track_focus(&handles[3])
                    .on_click(cx.listener(|this, event, window, cx| {
                        if crate::form_controls::checkbox_click(event) {
                            if let Some(handles) = &this.reference_delete_focus { handles[3].focus(window); }
                            this.reference_delete_remote_tag = !this.reference_delete_remote_tag; cx.notify();
                        }
                    }))
            ))
            .child(if prompt.tab == RepositoryTab::Tags && self.reference_delete_remote_tag { "원격 서버의 태그와 로컬 태그를 모두 삭제합니다. 다른 사용자에게도 영향을 줍니다. 계속할까요?" } else { message });
        if let Some((remote, reference)) = &prompt.request.remote {
            body = body.child(format!("{remote}: {reference}"));
        }
        Some(
            crate::dialog::Dialog::new(
                "reference-delete-confirmation",
                "선택한 항목을 삭제할까요?",
            )
            .track_focus(&handles[0])
            .body(body)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if let Some(handles) = &this.reference_delete_focus {
                    match event.keystroke.key.as_str() {
                        "escape" => this.choose_reference_delete(false, window, cx),
                        "enter" | "space"
                            if handles[3].is_focused(window)
                                && this
                                    .reference_delete
                                    .as_ref()
                                    .is_some_and(|prompt| prompt.tab == RepositoryTab::Tags) =>
                        {
                            this.reference_delete_remote_tag = !this.reference_delete_remote_tag;
                            cx.notify();
                        }
                        "enter" => {
                            let confirm = !handles[2].is_focused(window);
                            this.choose_reference_delete(confirm, window, cx);
                        }
                        "tab" => {
                            let order: &[usize] = if this
                                .reference_delete
                                .as_ref()
                                .is_some_and(|prompt| prompt.tab == RepositoryTab::Tags)
                            {
                                &[3, 1, 2]
                            } else {
                                &[1, 2]
                            };
                            let current = order
                                .iter()
                                .position(|index| handles[*index].is_focused(window));
                            let next = match current {
                                Some(index) if event.keystroke.modifiers.shift => {
                                    (index + order.len() - 1) % order.len()
                                }
                                Some(index) => (index + 1) % order.len(),
                                None if event.keystroke.modifiers.shift => order.len() - 1,
                                None => 0,
                            };
                            handles[order[next]].focus(window);
                        }
                        _ => {}
                    }
                }
                cx.stop_propagation();
            }))
            .footer(
                row()
                    .gap_2()
                    .child(
                        crate::dialog::button("reference-delete-confirm", "확인", true, true)
                            .track_focus(&handles[1])
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_reference_delete(true, window, cx)
                            })),
                    )
                    .child(
                        crate::dialog::button("reference-delete-cancel", "취소", false, true)
                            .track_focus(&handles[2])
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_reference_delete(false, window, cx)
                            })),
                    ),
            )
            .build(),
        )
    }

    fn search_result(&mut self, event: crate::search_panel::SearchEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !window.is_window_active() || self.footer_actions_blocked() || self.repository_tab != RepositoryTab::Search { return; }
        let Some(root) = self.footer_directory() else { return; };
        if !self.search_panel.as_ref().is_some_and(|panel| panel.read(cx).is_for(&root)) { return; }
        match event {
            crate::search_panel::SearchEvent::File { path, revision } => {
                if !self.search_panel.as_ref().is_some_and(|panel| panel.read(cx).is_current(revision)) { return; }
                let Some(root) = self.footer_directory() else { return; };
                self.show_file_history(root, FileChange { path: path.as_bytes().to_vec(), label: path, status: String::new(), staged: false,
                    before: crate::diff_model::Content::Empty, after: crate::diff_model::Content::Empty }, window, cx);
            }
            crate::search_panel::SearchEvent::Commit { id, revision: search_revision } => {
                if !self.search_panel.as_ref().is_some_and(|panel| panel.read(cx).selected_commit_is(search_revision, &id)) { return; }
                if let Some(index) = self.projects[self.project].commits.iter().position(|commit| commit.id == id) {
                    self.select_commit(self.project, index, true);
                    if let Some(history) = self.histories.get(&self.project) { history.scroll.scroll_to_item(index, gpui::ScrollStrategy::Center); }
                    cx.notify(); return;
                }
                let Some(path) = self.footer_directory() else { return; }; let owner = self.project;
                let name = self.projects[owner].name.clone(); self.invalidate_reference_request(); let revision = self.reference_revision;
                let source = self.search_panel.as_ref().unwrap().entity_id(); let root = path.clone(); let expected = id.clone();
                self.commit_loading.begin(revision, std::time::Instant::now()); let completed = self.commit_loading.completion();
                let task = self.reference_reads.spawn(cx.background_executor(), move || {
                    let result = repository::commit_snapshot(&root, &name, &id).map(|project| { let history = PreparedHistory::new(&project.commits); (project, history) }).map_err(|error| format!("{error:#}"));
                    completed.store(true, std::sync::atomic::Ordering::Release); result
                });
                cx.spawn_in(window, async move |this, cx| {
                    let Some(result) = task.await else { return; };
                    let _ = this.update_in(cx, |this, window, cx| {
                        if this.reference_revision != revision || this.project != owner || this.footer_directory().as_ref() != Some(&path) { return; }
                        this.commit_loading.reset();
                        if this.repository_tab != RepositoryTab::Search || !this.search_panel.as_ref().is_some_and(|panel| panel.entity_id() == source && panel.read(cx).selected_commit_is(search_revision, &expected)) { cx.notify(); return; }
                        match result { Ok((project, history)) => {
                            this.apply_refreshed_project(project, history);
                            if let Some(index) = this.projects[owner].commits.iter().position(|commit| commit.id == expected) {
                                this.select_commit(owner, index, true);
                                if let Some(history) = this.histories.get(&owner) { history.scroll.scroll_to_item(index, gpui::ScrollStrategy::Center); }
                            }
                        }, Err(error) => this.show_alert("검색 커밋 이동 실패", &error, window, cx) }
                        cx.notify();
                    });
                }).detach(); cx.notify();
            }
        }
    }

    fn search_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let root = self.projects[self.project].path.clone().unwrap_or_default();
        if self.search_owner != Some(self.project) || self.search_panel.as_ref().is_none_or(|panel| !panel.read(cx).is_for(&root)) {
            if let Some(panel) = self.search_panel.take() { panel.update(cx, |panel, _| panel.deactivate()); }
            let root = self.projects[self.project].path.clone().unwrap_or_default(); let owner = self.project;
            let panel = cx.new(|cx| crate::search_panel::SearchPanel::new(root, cx));
            self.search_subscription = Some(cx.subscribe_in(&panel, window, move |this, source, event: &crate::search_panel::SearchEvent, window, cx| {
                if this.project == owner && this.search_panel.as_ref().is_some_and(|panel| panel.entity_id() == source.entity_id()) { this.search_result(event.clone(), window, cx); }
            }));
            self.search_observer = Some(cx.observe(&panel, |this, _, cx| {
                this.update_delayed_progress(cx);
                cx.notify();
            }));
            self.search_owner = Some(owner); self.search_panel = Some(panel);
        }
        let panel = self.search_panel.as_ref().unwrap().clone();
        let enabled = !self.footer_actions_blocked() && !self.delayed_worktree_edit_pending(cx);
        let button_enabled = self.chrome_actions_enabled(cx);
        panel.update(cx, |panel, cx| panel.set_enabled(enabled, button_enabled, cx));
        div().flex().flex_col().flex_1().min_h_0().child(heading("저장소", "GIT")).child(self.panel_tabs(false, cx)).child(panel)
    }

    fn branches(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        if self.repository_tab == RepositoryTab::Search { return self.search_tab(window, cx); }
        let list_scroll = self
            .panel_scrolls
            .entry(("branches", self.project, self.repository_tab as usize))
            .or_default()
            .clone();
        let names = self.display_repository_names();
        let project_id = self.project;
        let tab = self.repository_tab;
        let list = multi_virtual_list(
            ("branches", self.project * 5 + self.repository_tab as usize),
            names.len(),
            self.repository_selection
                .selected(names.len())
                .into_iter()
                .collect(),
            PANEL,
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                let project = &this.projects[this.project];
                this.display_repository_names()
                    .get(range.clone())
                    .unwrap_or(&[])
                    .iter()
                    .enumerate()
                    .map(|(offset, name)| {
                        let index = range.start + offset;
                        let active =
                            this.repository_tab == RepositoryTab::Local && *name == project.branch;
                        row()
                            .id(("reference-menu-row", index))
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(
                                    move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        this.open_checkout_menu(
                                            (project_id, Some(tab), index),
                                            event.position,
                                            window,
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    },
                                ),
                            )
                            .h_full()
                            .px_3()
                            .gap(px(4.))
                            .rounded_md()
                            .text_color(rgb(if active { ACCENT } else { TEXT }))
                            .when(active, |el| el.child(badge("현재", ACCENT).flex_shrink_0()))
                            .child(
                                reference_icon_view(
                                    match this.repository_tab {
                                        RepositoryTab::Local => ReferenceKind::Local,
                                        RepositoryTab::Remote => ReferenceKind::Remote,
                                        RepositoryTab::Tags => ReferenceKind::Tag,
                                        RepositoryTab::Stashes | RepositoryTab::Search => ReferenceKind::Other,
                                    },
                                    if active { ACCENT } else { TEXT },
                                )
                                .unwrap_or_else(|| div().child("\u{2442}")),
                            )
                            .child(div().min_w_0().truncate().child(name.clone()))
                            .when(this.repository_tab == RepositoryTab::Local, |el| {
                                el.when_some(project.branch_upstreams.get(name), |el, upstream| {
                                    el.child(label("\u{2192}").flex_shrink_0()).child(
                                        reference_badge(
                                            ReferenceKind::Remote,
                                            repository::remote_branch_label(
                                                upstream,
                                                &project.remote_names,
                                            )
                                            .to_owned(),
                                            0xffa7b2,
                                        ),
                                    )
                                })
                            })
                    })
                    .collect::<Vec<_>>()
            }),
            {
                let owner = cx.weak_entity();
                move |index,
                      event: &gpui::MouseDownEvent,
                      window: &mut Window,
                      cx: &mut gpui::App| {
                    let _ = owner.update(cx, |this, cx| {
                        this.repository_row_mouse_down(
                            project_id,
                            tab,
                            index,
                            event.click_count,
                            window,
                            cx,
                        )
                    });
                }
            },
        )
        .scroll_handle(list_scroll)
        .loading(self.repository_loading.clone());
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(heading("저장소", "GIT"))
            .child(self.panel_tabs(false, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when(self.panel_references.is_err(), |el| {
                        el.child(
                            panel_message(self.panel_references.as_ref().unwrap_err().clone()),
                        )
                    })
                    .when(
                        !self.repository_waiting
                            && names.is_empty()
                            && self.panel_references.is_ok(),
                        |el| el.child(panel_message("표시할 항목이 없습니다.")),
                    )
                    .when(!names.is_empty() || self.repository_waiting, |el| {
                        el.child(list)
                    }),
            )
    }
    fn file_list_has_detail(&self) -> bool {
        self.file_history
            && !self.panel_waiting
            && self.history_files.is_ok()
            && !self.start_tab_active
            && self.open_projects.contains(&self.project)
            && self
                .projects
                .get(self.project)
                .is_some_and(|project| project.commits.get(self.commit).is_some())
    }

    fn file_list_len(&self) -> usize {
        self.changed_files().len() + usize::from(self.file_list_has_detail())
    }

    fn file_list_selected_row(&self) -> Option<usize> {
        let (files, detail) = self.display_file_list();
        self.file_selection
            .selected(files.len())
            .map(|index| index + usize::from(detail))
            .or_else(|| detail.then_some(0))
    }

    #[cfg(test)]
    fn select_file_list_row(
        &mut self,
        project: usize,
        revision: u64,
        historical: bool,
        row: usize,
    ) -> bool {
        self.select_file_list_row_with_modifiers(
            project,
            revision,
            historical,
            row,
            gpui::Modifiers::default(),
        )
    }

    fn select_file_list_row_with_modifiers(
        &mut self,
        project: usize,
        revision: u64,
        historical: bool,
        row: usize,
        modifiers: gpui::Modifiers,
    ) -> bool {
        if self.repository_busy()
            || project != self.project
            || revision != self.panel_revision
            || historical != self.file_history
            || self.panel_waiting
            || self.start_tab_active
            || !self.open_projects.contains(&project)
            || row >= self.file_list_len()
        {
            return false;
        }
        let detail = self.file_list_has_detail();
        if detail && row == 0 {
            self.select_commit_detail(project)
        } else if historical {
            self.file_selection
                .select(row - usize::from(detail), self.changed_files().len())
        } else {
            self.file_selection.click(
                row,
                self.changed_files().len(),
                modifiers.shift,
                modifiers.control,
            )
        }
    }

    fn commit_detail_selected(&self) -> bool {
        self.file_history
            && self
                .file_selection
                .selected(self.changed_files().len())
                .is_none()
    }

    fn select_commit_detail(&mut self, project_id: usize) -> bool {
        if self.repository_busy()
            || self.project != project_id
            || !self.file_list_has_detail()
            || self.start_tab_active
            || !self.open_projects.contains(&project_id)
        {
            return false;
        }
        self.file_selection.clear();
        true
    }

    fn files(&mut self, cx: &mut Context<Self>) -> Div {
        let list_scroll = self
            .panel_scrolls
            .entry(("files", self.project, usize::from(self.file_history)))
            .or_default()
            .clone();
        let (files, detail) = self.display_file_list();
        let project_id = self.project;
        let file_history = self.file_history;
        let panel_revision = self.panel_revision;
        let list_count = files.len() + usize::from(detail);
        let on_select = cx.processor(
            move |this, (index, modifiers): (usize, gpui::Modifiers), _, cx| {
                if this
                    .viewer
                    .as_ref()
                    .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
                {
                    return;
                }
                if this.select_file_list_row_with_modifiers(
                    project_id,
                    panel_revision,
                    file_history,
                    index,
                    modifiers,
                ) {
                    cx.notify();
                }
            },
        );
        let on_context =
            std::rc::Rc::new(cx.processor(move |this, (row, position), window, cx| {
                this.open_files_menu(
                    FileMenuRequest {
                        project: project_id,
                        revision: panel_revision,
                        historical: file_history,
                        row,
                    },
                    position,
                    window,
                    cx,
                );
            }));
        let selected = if file_history {
            self.file_list_selected_row().into_iter().collect()
        } else {
            self.file_selection
                .indices(files.len())
                .into_iter()
                .collect()
        };
        let list = crate::virtual_list::multi_virtual_list(
            ("files", self.project * 2 + usize::from(self.file_history)),
            list_count,
            selected,
            PANEL,
            cx.processor(move |this, range: std::ops::Range<usize>, _, _| {
                let (displayed_files, detail) = this.display_file_list();
                range
                    .filter_map(|index| {
                        if detail && index == 0 {
                            return Some(
                                row()
                                    .id(("file-content", index))
                                    .h_full()
                                    .px_2()
                                    .gap_3()
                                    .child(div().w(px(32.)).flex_shrink_0().child(commit_icon(
                                        if this.commit_detail_selected() {
                                            SELECTED
                                        } else {
                                            PANEL
                                        },
                                    )))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(12.))
                                            .text_color(rgb(TEXT))
                                            .child("커밋상세"),
                                    ),
                            );
                        }
                        let file = displayed_files.get(index - usize::from(detail))?;
                        let on_context = on_context.clone();
                        Some(
                            row()
                                .id(("file-content", index))
                                .on_mouse_down(
                                    gpui::MouseButton::Right,
                                    move |event, window, cx| {
                                        on_context((index, event.position), window, cx);
                                        cx.stop_propagation();
                                    },
                                )
                                .h_full()
                                .px_2()
                                .gap_3()
                                .child(
                                    div()
                                        .w(px(32.))
                                        .font_family("Cascadia Mono")
                                        .flex_shrink_0()
                                        .text_color(rgb(file_status_color(&file.status)))
                                        .child(file.status.clone()),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .child(file.path.clone()),
                                ),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            move |index, event, window, cx| on_select((index, event.modifiers), window, cx),
        )
        .keyboard_selected(self.file_list_selected_row())
        .scroll_handle(list_scroll)
        .loading(self.panel_loading.clone())
        .keep_content_while_loading(self.viewer_refresh_pending && !files.is_empty());
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(heading("변경 파일", files.len().to_string()))
            .child(self.panel_tabs(true, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when(self.file_history && self.history_files.is_err(), |el| {
                        el.child(
                            panel_message(self.history_files.as_ref().unwrap_err().clone()),
                        )
                    })
                    .when(
                        !self.panel_waiting
                            && list_count == 0
                            && (!self.file_history || self.history_files.is_ok()),
                        |el| el.child(panel_message("변경 파일이 없습니다")),
                    )
                    .when(list_count > 0 || self.panel_waiting, |el| el.child(list)),
            )
    }
    fn begin_panel_resize(&mut self, row: usize, position: [f32; 2]) -> bool {
        if row > 2
            || position.iter().any(|value| !value.is_finite())
            || self.panel_height.get() <= 0.
            || self.panel_width.get() <= 0.
            || self.restoring
            || self.repository_busy()
            || self.alert.is_some()
            || self.repository_dialog.is_some()
            || self.start_tab_active
            || !self.open_projects.contains(&self.project)
        {
            return false;
        }
        self.cancel_list_drags();
        self.history_resize = None;
        self.panel_resize.set(Some(PanelResize {
            row,
            project: self.project,
            start_position: position,
            start_split: self
                .panel_layout
                .split([self.panel_width.get(), self.panel_height.get()]),
        }));
        true
    }
    fn update_panel_resize(&mut self, position: [f32; 2], pressed: bool) -> bool {
        let Some(drag) = self.panel_resize.get() else {
            return false;
        };
        if !pressed
            || self.restoring
            || self.repository_busy()
            || self.alert.is_some()
            || self.repository_dialog.is_some()
            || self.start_tab_active
            || drag.project != self.project
            || !self.open_projects.contains(&drag.project)
        {
            self.panel_resize.set(None);
            return true;
        }
        self.panel_layout.resize(
            drag.row,
            drag.start_split,
            [
                position[0] - drag.start_position[0],
                position[1] - drag.start_position[1],
            ],
            [self.panel_width.get(), self.panel_height.get()],
        )
    }
    fn finish_panel_resize(&mut self, position: [f32; 2]) -> bool {
        if self.panel_resize.get().is_none() {
            return false;
        }
        self.update_panel_resize(position, true);
        self.panel_resize.set(None);
        true
    }
    fn panel_cursor(axis: usize) -> gpui::CursorStyle {
        match axis {
            0 => gpui::CursorStyle::ResizeLeftRight,
            1 => gpui::CursorStyle::ResizeUpDown,
            _ => gpui::CursorStyle::Crosshair,
        }
    }
    fn panel_splitter(&self, axis: usize, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let [x, y] = self
            .panel_layout
            .split([self.panel_width.get(), self.panel_height.get()]);
        let handle = crate::panel_layout::DIVIDER_WIDTH;
        div()
            .id(("panel-splitter", axis))
            .absolute()
            .occlude()
            .cursor(Self::panel_cursor(axis))
            .when(axis == 0, |el| {
                el.left(px(x - handle / 2.)).top_0().w(px(handle)).h_full()
            })
            .when(axis == 1, |el| {
                el.left_0().top(px(y - handle / 2.)).w_full().h(px(handle))
            })
            .when(axis == 2, |el| {
                el.left(px(x - handle))
                    .top(px(y - handle))
                    .size(px(handle * 2.))
            })
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    if this.begin_panel_resize(
                        axis,
                        [event.position.x.into(), event.position.y.into()],
                    ) {
                        cx.stop_propagation();
                        cx.notify();
                    }
                }),
            )
    }
    fn register_panel_resize(&self, window: &mut Window, cx: &Context<Self>) {
        if let Some(drag) = self.panel_resize.get() {
            window.set_window_cursor_style(Self::panel_cursor(drag.row));
        }
        let weak = cx.weak_entity();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, _, cx| {
            if phase != gpui::DispatchPhase::Capture {
                return;
            }
            let _ = weak.update(cx, |this, cx| {
                if this.panel_resize.get().is_some() {
                    if this.update_panel_resize(
                        [event.position.x.into(), event.position.y.into()],
                        event.dragging(),
                    ) {
                        cx.notify();
                    }
                    cx.stop_propagation();
                }
            });
        });
        let weak = cx.weak_entity();
        window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, _, cx| {
            if phase == gpui::DispatchPhase::Capture && event.button == gpui::MouseButton::Left {
                let _ = weak.update(cx, |this, cx| {
                    if this.finish_panel_resize([event.position.x.into(), event.position.y.into()])
                    {
                        cx.stop_propagation();
                        cx.notify();
                    }
                });
            }
        });
        let weak = cx.weak_entity();
        window.on_mouse_event(move |_: &gpui::MouseExitEvent, phase, _, cx| {
            if phase == gpui::DispatchPhase::Capture {
                let _ = weak.update(cx, |this, cx| {
                    if this.panel_resize.take().is_some() {
                        cx.notify();
                    }
                });
            }
        });
    }

    fn begin_history_resize(
        &mut self,
        column: usize,
        x: f32,
        measured_widths: Option<[f32; 5]>,
    ) -> bool {
        if self.repository_busy()
            || column >= 4
            || !x.is_finite()
            || self.start_tab_active
            || !self.open_projects.contains(&self.project)
        {
            return false;
        }
        if !self.histories.contains_key(&self.project) {
            return false;
        }
        let widths = measured_widths.unwrap_or_else(|| self.history_columns.widths());
        if widths.iter().any(|width| !width.is_finite() || *width < 0.) {
            return false;
        }
        self.panel_resize.set(None);
        self.history_resize = Some(HistoryResize {
            project: self.project,
            column,
            start_x: x,
            start_widths: widths,
        });
        true
    }

    fn update_history_resize(&mut self, x: f32, pressed: bool) -> bool {
        let Some(resize) = self.history_resize else {
            return false;
        };
        if !pressed
            || self.start_tab_active
            || resize.project != self.project
            || !self.open_projects.contains(&resize.project)
        {
            self.history_resize = None;
            return true;
        }
        if !self.histories.contains_key(&resize.project) {
            self.history_resize = None;
            return true;
        }
        self.history_columns
            .resize_boundary(resize.column, resize.start_widths, x - resize.start_x)
    }

    fn finish_history_resize(&mut self, x: f32) -> bool {
        if self.history_resize.is_none() {
            return false;
        }
        self.update_history_resize(x, true);
        self.history_resize = None;
        true
    }

    fn history_row(&self, data: PreparedHistoryRow, columns: [f32; 5], lanes: usize) -> Div {
        commit_grid_row(data, columns, lanes, true)
    }

    fn select_history_row(&mut self, project_id: usize, index: usize) -> bool {
        if self.project == project_id && self.commit == index {
            return false;
        }
        self.select_commit(project_id, index, true)
    }

    fn select_commit(&mut self, project_id: usize, index: usize, reveal_detail: bool) -> bool {
        if self.repository_busy()
            || self.project != project_id
            || self.start_tab_active
            || !self.open_projects.contains(&project_id)
            || index >= self.projects[project_id].commits.len()
        {
            return false;
        }
        let (files, detail) = self.display_file_list();
        let previous = (files.to_vec(), detail);
        if reveal_detail {
            self.file_history = true;
        }
        self.commit = index;
        self.invalidate_reference_request();
        self.repository_selection.clear();
        self.refresh_file_data();
        self.retained_file_list = Some(previous);
        true
    }
    fn commit_grid_loading(&self) -> Option<ListLoading> {
        if self.start_tab_active || !self.open_projects.contains(&self.project) {
            return None;
        }
        if self.commit_loading.is_pending() {
            return Some(self.commit_loading.clone());
        }
        let project = self.projects.get(self.project)?;
        (self.panel_waiting
            && self
                .histories
                .get(&self.project)
                .is_none_or(|history| history.len() != project.commits.len()))
        .then(|| self.panel_loading.clone())
    }

    fn history_checkout_target(
        &self,
        project: usize,
        index: usize,
        click_count: usize,
    ) -> Option<String> {
        if click_count != 2
            || self.repository_busy()
            || self.start_tab_active
            || self.project != project
            || !self.open_projects.contains(&project)
            || self.commit_grid_loading().is_some()
        {
            return None;
        }
        self.projects
            .get(project)?
            .commits
            .get(index)
            .map(|commit| commit.id.clone())
    }

    fn history_row_mouse_down(
        &mut self,
        project: usize,
        index: usize,
        click_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if click_count == 2 {
            if let Some(reference) = self.history_checkout_target(project, index, click_count) {
                self.select_history_row(project, index);
                cx.defer_in(window, move |this, window, cx| {
                    if this.history_checkout_target(project, index, 2).as_ref() == Some(&reference)
                    {
                        this.start_history_checkout((project, None, index), reference, window, cx);
                    }
                });
                cx.notify();
            }
        } else if click_count == 1 && self.select_history_row(project, index) {
            cx.notify();
        }
    }

    fn history_operation_status(&self) -> Option<(String, bool)> {
        if self.start_tab_active || !self.open_projects.contains(&self.project) {
            return None;
        }
        let project = self.projects.get(self.project)?;
        if self.commit_busy {
            return Some(("커밋 실행 중…".into(), false));
        }
        if self.sync_busy {
            return Some((self.sync_operation.unwrap_or("Git 작업 실행 중…").into(), false));
        }
        let state = &project.work_state;
        if let Some(error) = &state.error {
            return Some((format!("Git 작업 상태 확인 실패: {error}"), true));
        }
        use git2::RepositoryState;
        let operation = match state.operation {
            Some(RepositoryState::Merge) => "병합",
            Some(RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge) => "리베이스",
            Some(RepositoryState::CherryPick | RepositoryState::CherryPickSequence) => "체리픽",
            Some(RepositoryState::Revert | RepositoryState::RevertSequence) => "되돌리기",
            Some(RepositoryState::Bisect) => "이분 탐색",
            Some(RepositoryState::ApplyMailbox) => "패치 적용",
            Some(RepositoryState::ApplyMailboxOrRebase) => "패치 적용 또는 리베이스",
            Some(RepositoryState::Clean) | None => {
                return state.has_conflicts.then(|| ("충돌 해결 필요".into(), true));
            }
        };
        let detail = if state.has_conflicts { "충돌 해결 필요" } else { "완료 대기" };
        Some((format!("{operation} 진행 중 · {detail}"), state.has_conflicts))
    }

    fn merge_action_available(&self, commit: bool) -> bool { self.integration_action_available(false, commit) }
    fn integration_action_available(&self, rebase: bool, require_resolved: bool) -> bool {
        if self.footer_actions_blocked() || self.panel_waiting { return false; }
        self.projects.get(self.project).is_some_and(|project| {
            project.path.is_some() && project.head.is_some() && project.work_state.error.is_none()
                && if rebase { project.work_state.operation.is_some_and(repository::is_rebase_state) }
                    else { project.work_state.operation == Some(git2::RepositoryState::Merge) }
                && (!require_resolved || !project.work_state.has_conflicts)
        })
    }
    fn revert_action_available(&self, require_resolved: bool) -> bool {
        if self.footer_actions_blocked() || self.panel_waiting { return false; }
        self.projects.get(self.project).is_some_and(|project| project.path.is_some() && project.head.is_some()
            && project.work_state.error.is_none() && project.work_state.operation.is_some_and(repository::is_revert_state)
            && (!require_resolved || !project.work_state.has_conflicts))
    }
    fn operation_action_available(&self, operation: AbortOperation, require_resolved: bool) -> bool {
        match operation {
            AbortOperation::Revert => self.revert_action_available(require_resolved),
            AbortOperation::Rebase => self.integration_action_available(true, require_resolved),
            AbortOperation::Merge => self.integration_action_available(false, require_resolved),
        }
    }
    fn open_merge_abort(&mut self, window: &mut Window, cx: &mut Context<Self>) { self.open_operation_abort(false, window, cx); }
    fn open_operation_abort(&mut self, rebase: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open_abort_confirmation(if rebase { AbortOperation::Rebase } else { AbortOperation::Merge }, window, cx);
    }
    fn open_abort_confirmation(&mut self, operation: AbortOperation, window: &mut Window, cx: &mut Context<Self>) {
        if !self.operation_action_available(operation, false) || !window.is_window_active() { return; }
        let project = &self.projects[self.project];
        self.abort_operation = operation;
        self.merge_abort_confirmation = Some((self.project, project.path.clone().unwrap(), project.head.clone().unwrap()));
        self.merge_abort_previous_focus = window.focused(cx);
        self.merge_abort_focus = Some(cx.focus_handle());
        let buttons = std::array::from_fn(|_| cx.focus_handle()); buttons[1].focus(window);
        self.merge_abort_buttons = Some(buttons); cx.notify();
    }
    fn history_operation_toolbar(&self, cx: &mut Context<Self>) -> Option<Div> {
        let (message, attention) = self.history_operation_status()?;
        let merge = self.projects[self.project].work_state.operation == Some(git2::RepositoryState::Merge);
        let rebase = self.projects[self.project].work_state.operation.is_some_and(repository::is_rebase_state);
        let revert = self.projects[self.project].work_state.operation.is_some_and(repository::is_revert_state);
        let can_continue = self.integration_action_available(true, true);
        let can_abort_rebase = self.integration_action_available(true, false);
        let can_commit = self.merge_action_available(true);
        let can_abort = self.merge_action_available(false);
        Some(row().w_full().min_w_0().flex_shrink_0().gap_2().px_3().py_2()
            .bg(rgb(RAISED)).border_b_1().border_color(rgb(LINE))
            .child(div().text_color(rgb(if attention { 0xf2c879 } else { ACCENT }))
                .child(if attention { "!" } else { "•" }))
            .child(label(message).flex_1().min_w_0().truncate())
            .when(merge, |toolbar| toolbar
                .child(status_button("merge-commit", "병합 커밋", can_commit).h(px(24.)).text_size(px(12.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.merge_action_available(true) && window.is_window_active() {
                            this.open_commit_dialog(window, cx);
                        }
                    })))
                .child(status_button("merge-abort", "병합 취소", can_abort).h(px(24.)).text_size(px(12.))
                    .on_click(cx.listener(|this, _, window, cx| this.open_merge_abort(window, cx)))))
            .when(revert, |toolbar| toolbar
                .child(status_button("revert-continue", "Revert 계속", self.revert_action_available(true)).h(px(24.)).text_size(px(12.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.revert_action_available(true) && window.is_window_active() {
                            let expected_head = this.projects[this.project].head.clone().unwrap();
                            this.start_toolbar_action(workspace_backend::ToolbarAction::ContinueRevert { expected_head }, window, cx);
                        }
                    })))
                .child(status_button("revert-abort", "Revert 취소", self.revert_action_available(false)).h(px(24.)).text_size(px(12.))
                    .on_click(cx.listener(|this, _, window, cx| this.open_abort_confirmation(AbortOperation::Revert, window, cx)))))
            .when(rebase, |toolbar| toolbar
                .child(status_button("rebase-continue", "리베이스 계속", can_continue).h(px(24.)).text_size(px(12.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.integration_action_available(true, true) && window.is_window_active() {
                            let expected_head = this.projects[this.project].head.clone().unwrap();
                            this.start_toolbar_action(workspace_backend::ToolbarAction::ContinueRebase { expected_head }, window, cx);
                        }
                    })))
                .child(status_button("rebase-abort", "리베이스 취소", can_abort_rebase).h(px(24.)).text_size(px(12.))
                    .on_click(cx.listener(|this, _, window, cx| this.open_operation_abort(true, window, cx))))))
    }

    fn history(&self, cx: &mut Context<Self>) -> Div {
        let project_id = self.project;
        let project = &self.projects[project_id];
        let count = project.commits.len();
        let state = &self.histories[&project_id];
        let empty = state.is_empty();
        let loading = self.commit_grid_loading();
        let grid_pending = loading.is_some();
        let lanes = state.lanes;
        let columns = self.history_columns.widths();
        let total_width = self.history_columns.minimum_total_width();
        let scroll_frame = state.scroll_frame.clone();
        let measured_widths = Rc::new(Cell::new(columns));
        let resize_widths = measured_widths.clone();
        let on_resize = cx.processor(move |this, (column, x), _, cx| {
            if this.project == project_id
                && this.begin_history_resize(column, x, Some(resize_widths.get()))
            {
                cx.stop_propagation();
                cx.notify();
            }
        });
        let heading_row = commit_grid_header(
            columns,
            true,
            measured_widths,
            move |column, x, window, cx| {
                on_resize((column, x), window, cx);
            },
        );
        let list = multi_virtual_list(
            ("history", project_id),
            count,
            [self.commit].into_iter().collect(),
            BASE,
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                let visible_end = range.end;
                cx.defer_in(window, move |this, _, cx| {
                    if this.queue_history_page(project_id, visible_end) {
                        cx.notify();
                    }
                });
                let Some(state) = this.histories.get_mut(&project_id) else {
                    return Vec::new();
                };
                let rows = state.prepare_rows(&this.projects[project_id], range);
                rows.into_iter()
                    .map(|data| {
                        let index = data.index;
                        this.history_row(data, columns, lanes)
                            .id(("commit-menu-row", index))
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(
                                    move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        this.open_checkout_menu(
                                            (project_id, None, index),
                                            event.position,
                                            window,
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    },
                                ),
                            )
                    })
                    .collect::<Vec<_>>()
            }),
            {
                let owner = cx.weak_entity();
                move |index,
                      event: &gpui::MouseDownEvent,
                      window: &mut Window,
                      cx: &mut gpui::App| {
                    let _ = owner.update(cx, |this, cx| {
                        this.history_row_mouse_down(
                            project_id,
                            index,
                            event.click_count,
                            window,
                            cx,
                        )
                    });
                }
            },
        )
        .scroll_handle(scroll_frame)
        .header(heading_row, crate::virtual_list::ROW_HEIGHT)
        .min_content_width(total_width);
        let list = if let Some(loading) = loading {
            list.loading(loading)
        } else {
            list
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(heading(
                "커밋 기록",
                format!(
                    "{}{} commits",
                    count,
                    if project.more_commits { "+" } else { "" }
                ),
            ))
            .when_some(self.history_operation_toolbar(cx), |el, toolbar| el.child(toolbar))
            .when(empty && !grid_pending, |el| {
                el.child(panel_message("아직 커밋이 없습니다"))
            })
            .when(!empty || grid_pending, |el| el.child(list))
            .when_some(state.append_error.clone(), |el, error| {
                el.child(
                    row()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .child(label(error).flex_1().min_w_0().truncate())
                        .child(
                            div()
                                .id("retry-history-page")
                                .cursor_pointer()
                                .child("재시도")
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        if this.project == project_id {
                                            if let Some(history) =
                                                this.histories.get_mut(&project_id)
                                            {
                                                history.append_error = None;
                                            }
                                            this.queue_history_page(project_id, count);
                                            cx.notify();
                                        }
                                    }),
                                ),
                        ),
                )
            })
    }
    fn viewer_target(&self) -> ViewerTarget {
        let project = &self.projects[self.project];
        if !self.file_history
            && let (Some(root), Ok(files)) = (&project.path, &self.viewer_files)
        {
            let selected: Vec<_> = self
                .file_selection
                .indices(files.len())
                .into_iter()
                .map(|index| files[index].clone())
                .collect();
            if selected.len() > 1 {
                return ViewerTarget::Files {
                    root: root.clone(),
                    files: selected,
                };
            }
        }
        if let Some(index) = self.file_selection.selected(self.changed_files().len())
            && let (Some(root), Ok(files)) = (&project.path, &self.viewer_files)
            && let Some(file) = files.get(index)
        {
            return ViewerTarget::File {
                root: root.clone(),
                file: file.clone(),
                historical: self.file_history,
            };
        }
        if self.file_history
            && let Some(commit) = project.commits.get(self.commit)
            && let Some(root) = &project.path
        {
            return ViewerTarget::Detail {
                root: root.clone(),
                id: commit.id.clone(),
            };
        }
        ViewerTarget::Text {
            identity: format!("empty-viewer-{}-{}", self.project, self.file_history),
            title: "선택된 파일 0개".into(),
            lines: self
                .viewer_files
                .as_ref()
                .err()
                .cloned()
                .into_iter()
                .collect(),
        }
    }
    fn viewer_changed(&mut self, event: &ViewerChanged) {
        let Some(owner) = self
            .projects
            .iter()
            .position(|project| project.path.as_ref() == Some(&event.root))
        else {
            return;
        };
        if owner == self.project
            && !self.file_history
            && self.open_projects.contains(&owner)
            && !self.start_tab_active
        {
            let selected = self
                .viewer_files
                .as_ref()
                .map(|files| {
                    self.file_selection
                        .indices(files.len())
                        .into_iter()
                        .map(|index| (files[index].path.clone(), files[index].staged))
                        .collect()
                })
                .unwrap_or_default();
            self.request_file_data(true);
            if let Some(request) = &mut self.panel_pending {
                request.selected_files = selected;
            }
        } else {
            self.projects[owner].files.clear();
        }
    }
    fn selected_stage_requests(&self) -> Option<(std::path::PathBuf, Vec<FileChange>)> {
        if self.screen() != WorkspaceScreen::Project
            || self.file_history
            || self.panel_waiting
            || self.repository_dialog.is_some()
            || self.alert.is_some()
            || self.repository_busy()
        {
            return None;
        }
        let files = self.viewer_files.as_ref().ok()?;
        let selected: Vec<_> = self
            .file_selection
            .indices(files.len())
            .into_iter()
            .map(|index| files[index].clone())
            .collect();
        if selected.is_empty() {
            return None;
        }
        Some((self.projects.get(self.project)?.path.clone()?, selected))
    }

    fn local_refresh_action(&self) -> Option<workspace_backend::ToolbarAction> {
        if self.screen() != WorkspaceScreen::Project || self.footer_actions_blocked() || self.footer_directory().is_none() { return None; }
        let selected_files = self.viewer_files.as_ref().ok().map(|files| self.file_selection.indices(files.len()).into_iter()
            .map(|index| (files[index].path.clone(), files[index].staged)).collect()).unwrap_or_default();
        Some(workspace_backend::ToolbarAction::Refresh { selected_files })
    }

    fn select_all_worktree_files(&mut self) -> bool {
        if self.file_history
            || self.footer_actions_blocked()
            || self.panel_waiting
            || self.start_tab_active
            || !self.open_projects.contains(&self.project)
            || self.viewer_files.is_err()
        {
            return false;
        }
        let count = self.changed_files().len();
        self.file_selection.restore(0..count, count);
        true
    }

    fn files_key_down(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let select_all = select_all_files_shortcut(&event.keystroke);
        let discard = discard_shortcut(&event.keystroke);
        let copy_paths = copy_paths_shortcut(&event.keystroke);
        if (!plain_space(&event.keystroke) && !discard && !copy_paths && !select_all)
            || !window.is_window_active()
            || !self
                .panel_focus
                .as_ref()
                .is_some_and(|handles| handles[1].contains_focused(window, cx))
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        if select_all {
            if self.select_all_worktree_files() {
                cx.notify();
                cx.stop_propagation();
            }
        } else if copy_paths {
            self.copy_file_paths(self.project, self.panel_revision, self.file_history, cx);
            cx.stop_propagation();
        } else if discard {
            self.prepare_file_discard(self.project, self.panel_revision, window, cx);
            cx.stop_propagation();
        } else {
            self.toggle_selected_files(window, cx);
        }
    }

    fn toggle_selected_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !window.is_window_active()
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        let Some((root, files)) = self.selected_stage_requests() else {
            return;
        };
        self.file_stage_busy = true;
        self.update_delayed_progress(cx);
        let owner = root.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            workspace_backend::toggle_selected_files(&root, &files)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.file_stage_busy = false;
                this.update_delayed_progress(cx);
                this.viewer_changed(&ViewerChanged {
                    root: owner.clone(),
                });
                if let Some(request) = &mut this.panel_pending
                    && request.path.as_ref() == Some(&owner)
                {
                    request.selected_files = result.selected_files;
                }
                if !result.errors.is_empty() {
                    this.notice = format!(
                        "Stage 상태 변경 {}개 성공, {}개 실패: {}",
                        result.succeeded,
                        result.errors.len(),
                        result.errors.join(" / ")
                    );
                }
                cx.notify();
            });
        })
        .detach();
        cx.stop_propagation();
        cx.notify();
    }

    fn menu_stage_requests(
        &self,
        project: usize,
        revision: u64,
    ) -> Option<(std::path::PathBuf, Vec<FileChange>)> {
        (self.project == project && self.panel_revision == revision)
            .then(|| self.selected_stage_requests())
            .flatten()
    }

    fn prepare_file_discard(
        &mut self,
        project: usize,
        revision: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .viewer
            .as_ref()
            .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        let Some((root, files)) = self.menu_stage_requests(project, revision) else {
            return;
        };
        self.file_stage_busy = true;
        self.update_delayed_progress(cx);
        let owner = root.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            diff_git::prepare_discard(&root, &files).map_err(|error| format!("{error:#}"))
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.file_stage_busy = false;
                this.update_delayed_progress(cx);
                if this.project != project
                    || this.panel_revision != revision
                    || this
                        .projects
                        .get(project)
                        .and_then(|project| project.path.as_ref())
                        != Some(&owner)
                {
                    cx.notify();
                    return;
                }
                match result {
                    Ok(preview) => {
                        this.discard_previous_focus = window.focused(cx);
                        let focus = cx.focus_handle();
                        focus.focus(window);
                        this.discard_focus = Some(focus);
                        this.discard_prompt = Some(DiscardPrompt {
                            project,
                            root: owner,
                            preview,
                        });
                    }
                    Err(error) => this.show_alert("변경사항 폐기 준비 실패", &error, window, cx),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn take_discard_choice(&mut self, confirm: bool) -> Option<DiscardPrompt> {
        let prompt = self.discard_prompt.take()?;
        (confirm
            && self.project == prompt.project
            && self
                .projects
                .get(prompt.project)
                .and_then(|project| project.path.as_ref())
                == Some(&prompt.root))
        .then_some(prompt)
    }

    fn choose_discard(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.discard_prompt.is_none() {
            return;
        }
        let prompt = self.take_discard_choice(confirm);
        self.discard_focus = None;
        if let Some(focus) = self.discard_previous_focus.take() {
            focus.focus(window);
        } else {
            window.blur();
        }
        let Some(prompt) = prompt else {
            cx.notify();
            return;
        };
        self.file_stage_busy = true;
        self.update_delayed_progress(cx);
        let owner = prompt.root.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            diff_git::discard_selected(&prompt.root, prompt.preview)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.file_stage_busy = false;
                this.update_delayed_progress(cx);
                this.viewer_changed(&ViewerChanged { root: owner });
                this.notice = if result.errors.is_empty() {
                    format!("{}개 파일의 변경사항을 폐기했습니다.", result.succeeded)
                } else {
                    format!(
                        "변경사항 폐기 {}개 성공, {}개 실패: {}",
                        result.succeeded,
                        result.errors.len(),
                        result.errors.join(" / ")
                    )
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn choose_line_discard(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(request) = self.line_discard.take() else {
            return;
        };
        self.discard_focus = None;
        if let Some(focus) = self.discard_previous_focus.take() {
            focus.focus(window);
        } else {
            window.blur();
        }
        if confirm && !request.execute(window, cx) {
            self.notice = "미리보기가 변경되었습니다. 다시 체크한 뒤 폐기하세요.".into();
        }
        cx.notify();
    }

    fn discard_dialog(&self, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let focus = self.discard_focus.as_ref()?;
        let line_discard = self.line_discard.is_some();
        let (title, body) = if let Some(request) = &self.line_discard {
            (
                "체크한 줄을 폐기할까요?",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(request.label().to_owned())
                    .child(format!(
                        "체크한 변경 줄 {}개만 폐기합니다.",
                        request.count()
                    ))
                    .child(label(
                        "스테이지된 내용과 나머지 변경은 유지됩니다. 이 작업은 되돌릴 수 없습니다.",
                    )),
            )
        } else {
            let prompt = self.discard_prompt.as_ref()?;
            ("변경사항을 폐기할까요?", div().flex().flex_col().gap_3()
                .child(format!("선택한 {}개 파일의 스테이지된 변경과 미스테이지 변경을 모두 폐기합니다.", prompt.preview.labels.len()))
                .child(label("마지막 커밋(HEAD)으로 복원하고 신규 파일은 삭제합니다. 이 작업은 되돌릴 수 없습니다."))
                .child(div().id("discard-file-list").max_h(px(240.)).overflow_y_scroll().children(prompt.preview.labels.iter().map(|name| div().child(name.clone())))))
        };
        let button = |id: &'static str, text: &'static str, confirm, cx: &mut Context<Self>| {
            crate::dialog::button(id, text, confirm, true).on_click(cx.listener(
                move |this, _, window, cx| {
                    if line_discard {
                        this.choose_line_discard(confirm, window, cx);
                    } else {
                        this.choose_discard(confirm, window, cx);
                    }
                },
            ))
        };
        Some(
            crate::dialog::Dialog::new("discard-confirmation", title)
                .width(px(520.))
                .track_focus(focus)
                .on_key_down(
                    cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                        if discard_cancel_key(&event.keystroke.key) {
                            if line_discard {
                                this.choose_line_discard(false, window, cx);
                            } else {
                                this.choose_discard(false, window, cx);
                            }
                        }
                        cx.stop_propagation();
                    }),
                )
                .body(body)
                .footer(
                    row()
                        .gap_2()
                        .child(button("discard-confirm", "확인", true, cx))
                        .child(button("discard-cancel", "취소", false, cx)),
                )
                .build(),
        )
    }

    fn file_action_context(&self, project: usize, revision: u64, historical: bool) -> bool {
        self.project == project
            && self.panel_revision == revision
            && self.file_history == historical
            && self.screen() == WorkspaceScreen::Project
            && !self.panel_waiting
            && self.repository_dialog.is_none()
            && self.alert.is_none()
            && !self.repository_busy()
    }

    fn file_menu_row(&self, request: FileMenuRequest) -> Option<&FileChange> {
        if !self.file_action_context(request.project, request.revision, request.historical) {
            return None;
        }
        let index = request
            .row
            .checked_sub(usize::from(self.file_list_has_detail()))?;
        self.viewer_files.as_ref().ok()?.get(index)
    }

    fn selected_absolute_paths(
        &self,
        project: usize,
        revision: u64,
        historical: bool,
    ) -> Option<Vec<std::path::PathBuf>> {
        if !self.file_action_context(project, revision, historical) {
            return None;
        }
        let project = self.projects.get(project)?;
        let root = project.workdir.as_ref().or(project.path.as_ref())?;
        let files = self.viewer_files.as_ref().ok()?;
        let mut paths = Vec::new();
        for index in self.file_selection.indices(files.len()) {
            let path = crate::path_display::absolute_file_path(root, &files[index].path).ok()?;
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        (!paths.is_empty()).then_some(paths)
    }

    fn copy_file_paths(
        &mut self,
        project: usize,
        revision: u64,
        historical: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(paths) = self.selected_absolute_paths(project, revision, historical) else {
            return;
        };
        let text = paths
            .iter()
            .map(|path| display_path(path))
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
        self.notice = format!("{}개 파일의 절대경로를 복사했습니다.", paths.len());
        cx.notify();
    }

    fn open_file_location(
        &mut self,
        request: FileMenuRequest,
        reveal: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !window.is_window_active()
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        let Some(file) = self.file_menu_row(request) else {
            return;
        };
        let raw = file.path.clone();
        let Some(root) = self
            .projects
            .get(request.project)
            .and_then(|project| project.path.clone())
        else {
            return;
        };
        let owner = root.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            workspace_backend::prepare_file_location(&root, &raw, reveal)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.file_menu_row(request).is_none()
                    || this
                        .projects
                        .get(request.project)
                        .and_then(|project| project.path.as_ref())
                        != Some(&owner)
                {
                    return;
                }
                match result {
                    Ok(target) => {
                        // Windows Shell은 확장 경로 접두사 대신 일반 드라이브/UNC 경로를 받습니다.
                        let shell_path = std::path::PathBuf::from(display_path(&target.path));
                        cx.open_with_system(&shell_path);
                    }
                    Err(error) => this.show_alert("파일 열기 실패", &error, window, cx),
                }
            });
        })
        .detach();
    }

    fn file_history_target(
        &self,
        request: FileMenuRequest,
    ) -> Option<(std::path::PathBuf, FileChange)> {
        let file = self.file_menu_row(request)?.clone();
        Some((self.projects.get(request.project)?.path.clone()?, file))
    }

    fn open_file_history(
        &mut self,
        request: FileMenuRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !window.is_window_active()
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        let Some((root, file)) = self.file_history_target(request) else {
            return;
        };
        self.show_file_history(root, file, window, cx);
    }

    fn show_file_history(&mut self, root: std::path::PathBuf, file: FileChange, window: &mut Window, cx: &mut Context<Self>) {
        self.file_history_previous_focus = window.focused(cx);
        let workdir = self.projects.iter().find(|project| project.path.as_ref() == Some(&root))
            .and_then(|project| project.workdir.clone()).unwrap_or_else(|| root.clone());
        let dialog =
            cx.new(|cx| crate::file_history_dialog::FileHistoryDialog::new(root, workdir, file, window, cx));
        self.file_history_subscription = Some(cx.subscribe_in(
            &dialog,
            window,
            |this, _, _: &crate::file_history_dialog::FileHistoryClosed, window, cx| {
                this.close_file_history(window, cx);
            },
        ));
        self.file_history_overlay = Some(dialog);
        cx.notify();
    }

    fn close_file_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dialog) = self.file_history_overlay.take() {
            dialog.update(cx, |dialog, cx| dialog.deactivate(cx));
        }
        self.file_history_subscription = None;
        if let Some(focus) = self.file_history_previous_focus.take() {
            focus.focus(window);
        }
        cx.notify();
    }

    fn prepare_file_menu_selection(&mut self, request: FileMenuRequest) -> Option<bool> {
        self.file_menu_row(request)?;
        let index = request
            .row
            .checked_sub(usize::from(self.file_list_has_detail()))?;
        if self
            .file_selection
            .indices(self.changed_files().len())
            .contains(&index)
        {
            return Some(false);
        }
        self.select_file_list_row_with_modifiers(
            request.project,
            request.revision,
            request.historical,
            request.row,
            gpui::Modifiers::default(),
        )
        .then_some(true)
    }

    fn open_files_menu(
        &mut self,
        request: FileMenuRequest,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.file_menu_row(request).is_none()
            || self
                .viewer
                .as_ref()
                .is_some_and(|viewer| viewer.read(cx).is_busy(cx))
        {
            return;
        }
        let Some(changed) = self.prepare_file_menu_selection(request) else {
            return;
        };
        if changed {
            cx.notify();
        }
        if let Some(handles) = &self.panel_focus {
            handles[1].focus(window);
        }
        let project = request.project;
        let revision = request.revision;
        let can_copy = self
            .selected_absolute_paths(project, revision, request.historical)
            .is_some();
        let can_stage = self.selected_stage_requests().is_some();
        let open_owner = cx.weak_entity();
        let reveal_owner = cx.weak_entity();
        let copy_owner = cx.weak_entity();
        let history_owner = cx.weak_entity();
        let mut items = vec![
            crate::context_menu::MenuItem::new("열기", "", true, move |window, cx| {
                let _ = open_owner.update(cx, |this, cx| {
                    this.open_file_location(request, false, window, cx)
                });
            }),
            crate::context_menu::MenuItem::new(
                "탐색기에서 보기",
                "",
                true,
                move |window, cx| {
                    let _ = reveal_owner.update(cx, |this, cx| {
                        this.open_file_location(request, true, window, cx)
                    });
                },
            ),
            crate::context_menu::MenuItem::new(
                "파일 변경 이력",
                "",
                true,
                move |window, cx| {
                    let _ = history_owner
                        .update(cx, |this, cx| this.open_file_history(request, window, cx));
                },
            ),
            crate::context_menu::MenuItem::new(
                "경로복사",
                "Ctrl + Shift + C",
                can_copy,
                move |window, cx| {
                    let _ = copy_owner.update(cx, |this, cx| {
                        if window.is_window_active() && this.file_menu_row(request).is_some() {
                            this.copy_file_paths(project, revision, request.historical, cx);
                        }
                    });
                },
            )
            .separator_after(),
        ];
        if !request.historical {
            let owner = cx.weak_entity();
            let discard_owner = cx.weak_entity();
            items.push(crate::context_menu::MenuItem::new(
                "선택한 파일 스테이지",
                "Space",
                can_stage,
                move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if this.menu_stage_requests(project, revision).is_some() {
                            this.toggle_selected_files(window, cx);
                        }
                    });
                },
            ));
            items.push(crate::context_menu::MenuItem::new(
                "변경사항 폐기",
                "Ctrl + D",
                can_stage,
                move |window, cx| {
                    let _ = discard_owner.update(cx, |this, cx| {
                        this.prepare_file_discard(project, revision, window, cx)
                    });
                },
            ));
        }
        crate::context_menu::open(position, items, window, cx);
        cx.stop_propagation();
    }

    fn focusable_panel(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let handles = self
            .panel_focus
            .get_or_insert_with(|| std::array::from_fn(|_| cx.focus_handle()));
        let focus = handles[index].clone();
        if self.panel_focus_subscriptions.is_empty() {
            for handle in handles.iter() {
                self.panel_focus_subscriptions
                    .push(cx.on_focus(handle, window, |_, _, cx| cx.notify()));
                self.panel_focus_subscriptions
                    .push(cx.on_blur(handle, window, |_, _, cx| cx.notify()));
            }
        }
        div()
            .id(("workspace-panel", index))
            .track_focus(&focus)
            .on_key_down(cx.listener(move |this, event, window, cx| {
                if index == 1 {
                    this.files_key_down(event, window, cx);
                }
            }))
            // 기본 포커스를 먼저 지정하고 자식 입력 요소의 처리를 계속한다.
            .capture_any_mouse_down(cx.listener(
                move |this, event: &gpui::MouseDownEvent, window, _| {
                    if event.button == gpui::MouseButton::Left
                        && this.repository_dialog.is_none()
                        && this.alert.is_none()
                        && !this.repository_busy()
                    {
                        focus.focus(window);
                    }
                },
            ))
    }

    fn viewer_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        if self.viewer.is_none() {
            let viewer = cx.new(DiffViewer::new);
            self.viewer_subscription =
                Some(cx.subscribe(&viewer, |this, _, event: &ViewerChanged, cx| {
                    this.viewer_changed(event);
                    cx.notify();
                }));
            self.line_discard_subscription = Some(cx.subscribe_in(
                &viewer,
                window,
                |this, _, request: &ViewerDiscardRequest, window, cx| {
                    if this.repository_busy()
                        || this.alert.is_some()
                        || this.repository_dialog.is_some()
                    {
                        return;
                    }
                    this.discard_previous_focus = window.focused(cx);
                    let focus = cx.focus_handle();
                    focus.focus(window);
                    this.discard_focus = Some(focus);
                    this.line_discard = Some(request.clone());
                    cx.notify();
                },
            ));
            self.viewer_observer = Some(cx.observe(&viewer, |this, _, cx| {
                this.update_delayed_progress(cx);
                cx.notify();
            }));
            self.viewer = Some(viewer);
        }
        let target = self.viewer_target();
        let refresh = self.viewer_refresh_pending && !self.panel_waiting;
        if refresh { self.viewer_refresh_pending = false; }
        let viewer = self.viewer.as_ref().unwrap().clone();
        let interactive = self.repository_dialog.is_none()
            && self.alert.is_none()
            && !self.repository_busy()
            && !(self.panel_waiting && self.viewer_refresh_pending);
        viewer.update(cx, |this, cx| {
            this.set_interactive(interactive);
            if refresh { this.refresh_target(target, window, cx); } else { this.set_target(target, window, cx); }
        });
        div()
            .flex_1()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .child(viewer)
    }
}
impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.dispatch_pending(_window, cx);
        let screen = self.screen();
        self.update_delayed_progress(cx);
        let immediate_progress = self.global_progress_visible();
        let progress_pending = immediate_progress || self.delayed_progress.is_some();
        let show_progress = immediate_progress || self.delayed_progress_visible(Instant::now());
        let blocked = progress_pending
            || self.repository_dialog.is_some()
            || self.alert.is_some()
            || self.repository_busy();
        for scroll in self.panel_scrolls.values() {
            scroll.set_suspended(blocked);
        }
        for history in self.histories.values() {
            history.scroll_frame.set_suspended(blocked);
        }
        if blocked || !_window.is_window_active() || self.screen() != WorkspaceScreen::Project {
            self.panel_resize.set(None);
        }
        let popup_menu_allowed = self.file_history_overlay.is_some()
            && self.alert.is_none()
            && self.repository_dialog.is_none();
        if (blocked && !popup_menu_allowed) || !_window.is_window_active() {
            crate::context_menu::dismiss(_window, cx);
        }
        if screen == WorkspaceScreen::Loading {
            return self.window_frame(_window).child(StartupLoadingScreen);
        }
        if (self.start_tab_active || self.open_projects.is_empty())
            && let Some(viewer) = &self.viewer
        {
            viewer.update(cx, |viewer, cx| viewer.deactivate(cx));
        }
        let content = div()
            .id("workspace-content")
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if local_refresh_shortcut(&event.keystroke) && window.is_window_active()
                    && !this.viewer.as_ref().is_some_and(|viewer| viewer.read(cx).is_busy(cx)) {
                    if let Some(action) = this.local_refresh_action() { this.execute_toolbar_action(action, window, cx); }
                    cx.stop_propagation();
                }
            }))
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(self.tabs(cx))
            .when(screen == WorkspaceScreen::Start, |el| {
                el.child(self.start_page(cx))
            })
            .when(screen == WorkspaceScreen::Project, |el| {
                el.child(self.toolbar(cx)).child(
                    div()
                        .relative()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            div()
                                .w(px(self
                                    .panel_layout
                                    .split([self.panel_width.get(), self.panel_height.get()])
                                    [0]))
                                .flex_shrink_0()
                                .flex()
                                .flex_col()
                                .min_h_0()
                                .min_w_0()
                                .overflow_hidden()
                                .bg(rgb(PANEL))
                                .child(
                                    self.focusable_panel(0, _window, cx)
                                        .h(px(self.panel_layout.split([
                                            self.panel_width.get(),
                                            self.panel_height.get(),
                                        ])[1]))
                                        .flex_shrink_0()
                                        .flex()
                                        .flex_col()
                                        .min_h_0()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .child(self.branches(_window, cx)),
                                )
                                .child(
                                    self.focusable_panel(1, _window, cx)
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .min_h_0()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .child(self.files(cx)),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .min_h_0()
                                .min_w_0()
                                .overflow_hidden()
                                .child(
                                    self.focusable_panel(2, _window, cx)
                                        .h(px(self.panel_layout.split([
                                            self.panel_width.get(),
                                            self.panel_height.get(),
                                        ])[1]))
                                        .flex_shrink_0()
                                        .flex()
                                        .flex_col()
                                        .min_h_0()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .child(self.history(cx)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .min_h_0()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .child(self.viewer_panel(_window, cx)),
                                ),
                        )
                        .child({
                            let weak = cx.weak_entity();
                            gpui::canvas(
                                |_, _, _| (),
                                move |bounds, _, window, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        let width: f32 = bounds.size.width.into();
                                        let height: f32 = bounds.size.height.into();
                                        let previous_width = this.panel_width.replace(width);
                                        let previous_height = this.panel_height.replace(height);
                                        if (previous_width - width).abs() > 0.5
                                            || (previous_height - height).abs() > 0.5
                                        {
                                            window.on_next_frame(|window, _| window.refresh());
                                        }
                                        let [x, y] = this.panel_layout.split([width, height]);
                                        window.paint_quad(gpui::fill(
                                            gpui::Bounds::new(
                                                bounds.origin + gpui::point(px(x - 0.5), px(0.)),
                                                gpui::size(px(1.), bounds.size.height),
                                            ),
                                            rgb(LINE),
                                        ));
                                        window.paint_quad(gpui::fill(
                                            gpui::Bounds::new(
                                                bounds.origin + gpui::point(px(0.), px(y - 0.5)),
                                                gpui::size(bounds.size.width, px(1.)),
                                            ),
                                            rgb(LINE),
                                        ));
                                        this.register_panel_resize(window, cx);
                                    });
                                },
                            )
                            .absolute()
                            .size_full()
                        })
                        .child(self.panel_splitter(0, cx))
                        .child(self.panel_splitter(1, cx))
                        .child(self.panel_splitter(2, cx)),
                )
            })
            .when(screen == WorkspaceScreen::Empty, |el| {
                el.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .child(div().text_size(px(20.)).child("열린 프로젝트가 없습니다"))
                        .child(label("+ 버튼으로 새 탭을 여세요")),
                )
            })
            .child(self.status_bar(cx))
            .when_some(
                self.branch_dialog.clone().filter(|_| !self.sync_busy),
                |el, dialog| el.child(dialog),
            )
            .when_some(
                self.commit_dialog.clone().filter(|_| !self.commit_busy),
                |el, dialog| el.child(dialog),
            )
            .when_some(self.settings_overlay(_window, cx), |el, dialog| {
                el.child(dialog)
            })
            .when_some(self.git_action_dialog.clone(), |el, dialog| el.child(dialog))
            .when_some(self.encoding_dialog.clone(), |el, dialog| el.child(dialog))
            .when_some(self.file_history_overlay.clone(), |el, dialog| {
                el.child(dialog)
            })
            .when_some(self.reference_delete_dialog(cx), |el, dialog| {
                el.child(dialog)
            })
            .when_some(self.merge_abort_dialog(cx), |el, dialog| el.child(dialog))
            .when_some(self.branch_checkout_dialog(cx), |el, dialog| el.child(dialog))
            .when_some(self.checkout_dialog(cx), |el, dialog| el.child(dialog))
            .when_some(self.discard_dialog(cx), |el, dialog| el.child(dialog))
            .when_some(self.repository_dialog.clone(), |el, dialog| {
                el.child(dialog)
            })
            .when_some(
                self.recent_removal
                    .as_ref()
                    .zip(self.removal_focus.as_ref()),
                |el, (entry, focus)| {
                    let workspace = cx.entity().downgrade();
                    el.child(crate::recent_repository_dialog::render(
                        entry,
                        focus,
                        &self.removal_hold,
                        _window,
                        move |choice, window, cx| {
                            if let Some(workspace) = workspace.upgrade() {
                                workspace.update(cx, |this, cx| {
                                    this.choose_recent_removal(choice, window, cx)
                                });
                            }
                        },
                    ))
                },
            )
            .when_some(
                self.alert.as_ref().zip(self.alert_focus.as_ref()),
                |el, (alert, focus)| {
                    let workspace = cx.entity().downgrade();
                    el.child(alert.render(focus, move |window, cx| {
                        if let Some(workspace) = workspace.upgrade() {
                            workspace.update(cx, |this, cx| {
                                if let Some(previous) = this.previous_focus.take() {
                                    previous.focus(window);
                                } else {
                                    window.blur();
                                }
                                this.dismiss_alert();
                                cx.notify();
                            });
                        }
                    }))
                },
            );
        self.window_frame(_window)
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.update_history_resize(event.position.x.into(), event.dragging()) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| {
                    if this.finish_history_resize(event.position.x.into()) {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| {
                    if this.finish_history_resize(event.position.x.into()) {
                        cx.notify();
                    }
                }),
            )
            .child(content)
            .when_some(crate::context_menu::active(cx), |root, menu| {
                root.child(menu)
            })
            .when(progress_pending, |root| {
                let root = root.capture_key_down(|_, _, cx| cx.stop_propagation());
                let overlay = ProgressOverlay::new("처리 중입니다…")
                    .top_inset(px(CUSTOM_TITLEBAR_HEIGHT))
                    .show_card(show_progress);
                root.child(if let Some(focus) = &self.progress_focus {
                    overlay.track_focus(focus)
                } else {
                    overlay
                })
            })
    }
}

#[cfg(test)]
mod tab_tests {
    use super::*;
    // 기존 검증은 동일한 요청을 별도 스레드에서 완료한 뒤 실제 결과 반영 함수를 사용합니다.
    pub(super) fn drive(state: &mut Workspace) {
        loop {
            let mut progressed = false;
            if let Some(request) = state.repository_pending.take() {
                let result = std::thread::spawn(move || request.execute())
                    .join()
                    .unwrap();
                state.finish_repository_list(result);
                progressed = true;
            }
            if let Some(request) = state.reference_pending.take() {
                let expected = request.clone();
                let result = std::thread::spawn(move || request.execute())
                    .join()
                    .unwrap();
                state.finish_reference(expected, result);
                progressed = true;
            }
            if let Some(request) = state.panel_pending.take() {
                let result = std::thread::spawn(move || request.execute())
                    .join()
                    .unwrap();
                state.finish_panel(result);
                progressed = true;
            }
            if let Some((revision, result)) = state.save_pending.take() {
                state.finish_save(revision, result.recv());
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
        state.session_writer.flush();
    }

    #[test]
    fn recent_entry_only_removal_preserves_files_tabs_and_persists() {
        let (dir, mut state) = workspace_with_repositories();
        state.session_path = Some(dir.path().join("session.json"));
        let entry = state.recent_repositories[0].clone();
        let tabs = state.open_projects.clone();
        state.recent_repositories.reverse();
        state.forget_recent(&entry);
        drive(&mut state);
        assert_eq!(state.open_projects, tabs);
        assert!(entry.path.join("HEAD").exists());
        assert!(entry.path.parent().unwrap().join("file.txt").exists());
        assert!(
            !state
                .recent_repositories
                .iter()
                .any(|item| item.path == entry.path)
        );
        let saved = session::read(state.session_path.as_ref().unwrap()).unwrap();
        assert!(!saved.recent.iter().any(|item| item.path == entry.path));
        assert_eq!(saved.open.len(), tabs.len());
    }

    #[test]
    fn recent_deletion_failure_keeps_entries_and_open_tabs() {
        let (_dir, mut state) = workspace_with_repositories();
        let recent = state.recent_repositories.clone();
        let tabs = state.open_projects.clone();
        state.deleting_recent = Some(recent[0].clone());
        assert!(state.repository_busy());
        assert!(!state.begin_repository_operation(Operation::Open));
        let error = state
            .finish_recent_deletion(Err(anyhow::anyhow!("locked")))
            .unwrap_err();
        assert!(error.contains("locked"));
        assert!(error.contains("일부 파일"));
        assert_eq!(state.recent_repositories, recent);
        assert_eq!(state.open_projects, tabs);
        assert!(!state.repository_busy());
    }

    #[test]
    fn recent_deletion_success_closes_only_affected_tabs() {
        let (dir, mut state) = workspace_with_repositories();
        state.session_path = Some(dir.path().join("session.json"));
        let entry = state.recent_repositories[0].clone();
        let id = state
            .open_projects
            .iter()
            .copied()
            .find(|&id| state.projects[id].path.as_ref() == Some(&entry.path))
            .unwrap();
        let other_tabs: Vec<_> = state
            .open_projects
            .iter()
            .copied()
            .filter(|&other| other != id)
            .collect();
        state.start_tab_active = true;
        state.deleting_recent = Some(entry.clone());
        repository::delete_repository_files(&entry.path).unwrap();
        state.finish_recent_deletion(Ok(())).unwrap();
        drive(&mut state);
        assert_eq!(state.open_projects, other_tabs);
        assert!(!entry.path.exists());
        assert!(
            !state
                .recent_repositories
                .iter()
                .any(|item| item.path == entry.path)
        );
        assert!(!state.histories.contains_key(&id));
        let saved = session::read(state.session_path.as_ref().unwrap()).unwrap();
        assert_eq!(saved.open.len(), other_tabs.len());
    }

    #[test]
    fn recent_deletion_of_last_repository_and_nested_tab_returns_to_start() {
        let (_dir, mut state) = workspace_with_repositories();
        let entry = state.recent_repositories[0].clone();
        let id = state
            .open_projects
            .iter()
            .copied()
            .find(|&id| state.projects[id].path.as_ref() == Some(&entry.path))
            .unwrap();
        let nested = entry.path.join("nested");
        git2::Repository::init(&nested).unwrap();
        let project = repository::execute(Request {
            operation: Operation::Open,
            source: String::new(),
            path: nested.to_string_lossy().into_owned(),
            name: "nested".into(),
        })
        .unwrap();
        state.projects.push(project);
        state.open_projects = vec![id, state.projects.len() - 1];
        state.project = id;
        state.start_tab_open = false;
        state.start_tab_active = false;
        state.recent_repositories.push(RepositoryEntry {
            name: "nested".into(),
            path: nested.clone(),
        });
        state.deleting_recent = Some(entry.clone());
        repository::delete_repository_files(&entry.path).unwrap();
        state.finish_recent_deletion(Ok(())).unwrap();
        assert!(state.open_projects.is_empty());
        assert!(state.start_tab_open && state.start_tab_active);
        assert!(state.viewer.is_none());
        assert!(
            !state
                .recent_repositories
                .iter()
                .any(|item| item.path.starts_with(&entry.path))
        );
    }

    #[test]
    fn panel_drag_changes_only_layout_and_keeps_queries_selection_and_columns() {
        let (_dir, mut state) = workspace_with_repositories();
        drive(&mut state);
        state.panel_width.set(1200.);
        state.panel_height.set(800.);
        let revisions = (
            state.panel_revision,
            state.repository_revision,
            state.reference_revision,
            state.save_revision,
        );
        let columns = state.history_columns.widths();
        let files = state.viewer_files.clone();
        let selected = state.file_selection.selected(state.changed_files().len());
        assert!(state.begin_panel_resize(0, [100., 100.]));
        assert!(state.update_panel_resize([200., 200.], true));
        assert_eq!(state.panel_layout.split([1200., 800.])[0], 410.);
        assert_eq!(state.panel_layout.split([1200., 800.])[1], 432.);
        assert!(state.finish_panel_resize([250., 200.]));
        assert_eq!(state.panel_layout.split([1200., 800.])[0], 460.);
        assert!(!state.update_panel_resize([500., 200.], true));
        assert!(state.begin_panel_resize(2, [100., 100.]));
        assert!(state.update_panel_resize([140., 130.], true));
        assert_eq!(state.panel_layout.split([1200., 800.]), [500., 462.]);
        state.finish_panel_resize([140., 130.]);
        assert!(state.begin_panel_resize(1, [100., 100.]));
        state.finish_panel_resize([150., 120.]);
        assert_eq!(state.panel_layout.split([1200., 800.]), [500., 482.]);
        assert_eq!(state.history_columns.widths(), columns);
        assert_eq!(state.viewer_files, files);
        assert_eq!(
            state.file_selection.selected(state.changed_files().len()),
            selected
        );
        assert_eq!(
            (
                state.panel_revision,
                state.repository_revision,
                state.reference_revision,
                state.save_revision
            ),
            revisions
        );
    }

    #[test]
    fn panel_drag_cancels_on_release_navigation_and_blocks_modal_operations() {
        let (_dir, mut state) = workspace_with_repositories();
        state.panel_width.set(1200.);
        state.panel_height.set(800.);
        assert!(state.begin_panel_resize(1, [100., 100.]));
        assert!(state.update_panel_resize([200., 200.], false));
        assert!(state.panel_resize.get().is_none());
        assert_eq!(state.panel_layout.split([1200., 800.])[1], 432.);
        assert!(state.begin_panel_resize(1, [100., 100.]));
        state.open_start_tab();
        assert!(state.panel_resize.get().is_none());
        assert!(!state.begin_panel_resize(0, [100., 100.]));
        state.start_tab_active = false;
        state.alert = Some(Alert::new("알림", "모달"));
        assert!(!state.begin_panel_resize(0, [100., 100.]));
        state.alert = None;
        state.repository_operation = Some(Operation::Open);
        assert!(!state.begin_panel_resize(0, [100., 100.]));
        state.repository_operation = None;
        assert!(state.begin_history_resize(0, 100., None));
        assert!(state.begin_panel_resize(1, [100., 100.]));
        assert!(state.history_resize.is_none());
        assert!(state.begin_history_resize(0, 100., None));
        assert!(state.panel_resize.get().is_none());
    }

    #[test]
    fn history_page_trigger_append_errors_and_stale_results_preserve_existing_state() {
        let (_dir, mut state) = workspace_with_repositories();
        drive(&mut state);
        let owner = state.project;
        let total = state.projects[owner].commits.len() + 1501;
        let path = state.projects[owner].path.clone().unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = parent.tree().unwrap();
        let sig = git2::Signature::now("Pager", "pager@example.invalid").unwrap();
        let mut latest = parent.id();
        for n in 0..1501 {
            let parent = repo.find_commit(latest).unwrap();
            latest = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("page {n}"),
                    &tree,
                    &[&parent],
                )
                .unwrap();
        }
        let loaded = repository::execute(Request {
            operation: Operation::Open,
            source: String::new(),
            path: path.to_string_lossy().into(),
            name: "pages".into(),
        })
        .unwrap();
        state.projects[owner] = loaded;
        state
            .histories
            .insert(owner, HistoryState::new(&state.projects[owner].commits));
        state.panel_pending = None;
        state.panel_waiting = false;
        state.commit_loading.reset();
        state.commit = 10;
        let vertical = state.histories[&owner].scroll.clone();
        state.histories[&owner]
            .horizontal_scroll
            .set_offset(gpui::point(px(-75.), px(0.)));
        let panel_revision = state.panel_revision;
        let reference_revision = state.reference_revision;
        assert!(!state.queue_history_page(owner, 1449));
        assert!(state.queue_history_page(owner, 1450));
        assert!(!state.queue_history_page(owner, 1500));
        let request = state.history_append_pending.take().unwrap();
        let result = request.clone().execute();
        assert!(state.finish_history_page(result));
        assert_eq!(state.projects[owner].commits.len(), total);
        assert!(!state.projects[owner].more_commits);
        assert!(!state.queue_history_page(owner, total));
        assert_eq!(state.commit, 10);
        assert_eq!(state.panel_revision, panel_revision);
        assert_eq!(state.reference_revision, reference_revision);
        assert!(state.panel_pending.is_none());
        assert!(std::rc::Rc::ptr_eq(
            &vertical.0,
            &state.histories[&owner].scroll.0
        ));
        assert_eq!(
            state.histories[&owner].horizontal_scroll.offset().x,
            px(-75.)
        );
        // 같은 길이여도 스냅샷 커서가 바뀌면 이전 결과를 거절한다.
        let reload = || {
            repository::execute(Request {
                operation: Operation::Open,
                source: String::new(),
                path: path.to_string_lossy().into(),
                name: "pages".into(),
            })
            .unwrap()
        };
        state.projects[owner] = reload();
        state
            .histories
            .insert(owner, HistoryState::new(&state.projects[owner].commits));
        assert!(state.queue_history_page(owner, 1500));
        let old = state.history_append_pending.take().unwrap();
        let old_result = old.execute();
        state.projects[owner] = reload();
        assert!(!state.finish_history_page(old_result));
        assert_eq!(state.projects[owner].commits.len(), 1500);
        assert!(state.queue_history_page(owner, 1500));
        let failed = state.history_append_pending.take().unwrap();
        assert!(
            state.finish_history_page(crate::workspace_backend::HistoryPageResult {
                request: failed,
                result: Err("테스트 조회 오류".into())
            })
        );
        assert_eq!(state.projects[owner].commits.len(), 1500);
        assert!(!state.queue_history_page(owner, 1500));
        assert!(
            state.histories[&owner]
                .append_error
                .as_ref()
                .unwrap()
                .contains("테스트")
        );
        state.histories.get_mut(&owner).unwrap().append_error = None;
        assert!(state.queue_history_page(owner, 1500));
        let retry = state.history_append_pending.take().unwrap();
        assert!(state.finish_history_page(retry.execute()));
        assert_eq!(state.projects[owner].commits.len(), total);
    }

    #[test]
    fn reference_jump_continues_pagination_from_replacement_cursor() {
        let (_dir, mut state) = workspace_with_repositories();
        drive(&mut state);
        let owner = state.project;
        let total = state.projects[owner].commits.len() + 1800;
        let path = state.projects[owner].path.clone().unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = parent.tree().unwrap();
        let sig = git2::Signature::now("Pager", "pager@example.invalid").unwrap();
        let mut latest = parent.id();
        for n in 0..1800 {
            let parent = repo.find_commit(latest).unwrap();
            latest = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("jump {n}"),
                    &tree,
                    &[&parent],
                )
                .unwrap();
        }
        let target = repo.revparse_single("HEAD~1550").unwrap();
        repo.tag_lightweight("jump-page", &target, false).unwrap();
        let target_id = target.id().to_string();
        let worktree = std::fs::read(repo.workdir().unwrap().join("file.txt")).unwrap();
        let loaded = repository::execute(Request {
            operation: Operation::Open,
            source: String::new(),
            path: path.to_string_lossy().into(),
            name: "jump pages".into(),
        })
        .unwrap();
        state.activate_repository(loaded);
        drive(&mut state);
        assert_eq!(state.projects[owner].commits.len(), 1500);
        let scroll = state.histories[&owner].scroll.clone();
        state.histories[&owner]
            .horizontal_scroll
            .set_offset(gpui::point(px(-35.), px(0.)));
        // 이전 스냅샷의 페이지를 읽었지만 UI에는 아직 전달하지 않는다.
        assert!(state.queue_history_page(owner, 1500));
        let old_page = state.history_append_pending.take().unwrap().execute();
        state.repository_tab = RepositoryTab::Tags;
        state.panel_references = Ok(vec!["jump-page".into()]);
        state.repository_selection.clear();
        assert!(state.select_repository_row(owner, RepositoryTab::Tags, 0));
        let request = state.reference_pending.take().unwrap();
        let replacement = request.execute();
        assert!(replacement.as_ref().unwrap().0.more_commits);
        let replacement_cursor = replacement
            .as_ref()
            .unwrap()
            .0
            .history_cursor
            .clone()
            .unwrap();
        assert!(state.finish_reference(request, replacement));
        drive(&mut state);
        assert!(state.projects[owner].commits.len() > 1500);
        let selected = state.commit;
        assert_eq!(state.projects[owner].commits[selected].id, target_id);
        assert!(!state.finish_history_page(old_page));
        let installed_cursor = state.projects[owner].history_cursor.clone().unwrap();
        while state.projects[owner].more_commits {
            let before = state.projects[owner].commits.len();
            assert!(state.queue_history_page(owner, before));
            let page = state.history_append_pending.take().unwrap().execute();
            assert!(state.finish_history_page(page));
            assert!(
                state.histories[&owner].append_error.is_none(),
                "바로 건너뛰기 후 추가 조회 오류: {:?}",
                state.histories[&owner].append_error
            );
            let added = state.projects[owner].commits.len() - before;
            assert!(added > 0 && added <= 200);
        }
        assert!(installed_cursor.same_session(&replacement_cursor));
        assert_eq!(state.projects[owner].commits.len(), total);
        assert!(state.projects[owner].history_cursor.is_none());
        assert!(!state.queue_history_page(owner, total));
        assert_eq!(state.commit, selected);
        assert!(std::rc::Rc::ptr_eq(
            &scroll.0,
            &state.histories[&owner].scroll.0
        ));
        assert_eq!(
            state.histories[&owner].horizontal_scroll.offset().x,
            px(-35.)
        );
        let mut walk = repo.revwalk().unwrap();
        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
            .unwrap();
        walk.push_head().unwrap();
        let expected: Vec<_> = walk.map(|id| id.unwrap().to_string()).collect();
        assert_eq!(
            state.projects[owner]
                .commits
                .iter()
                .map(|commit| commit.id.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(std::fs::read(repo.workdir().unwrap().join("file.txt")).unwrap(), worktree);
    }

    fn workspace_with_repositories() -> (tempfile::TempDir, Workspace) {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/workspace-tab-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let mut state = Workspace::default();
        for project in 0..4 {
            let path = dir.path().join(project.to_string());
            let repo = git2::Repository::init(&path).unwrap();
            let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
            for index in 0..3 {
                std::fs::write(path.join("file.txt"), format!("revision {index}\n")).unwrap();
                let mut staging = repo.index().unwrap();
                staging.add_path(std::path::Path::new("file.txt")).unwrap();
                staging.write().unwrap();
                let tree_id = staging.write_tree().unwrap();
                let tree = repo.find_tree(tree_id).unwrap();
                let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
                let parents: Vec<_> = parent.iter().collect();
                let id = repo
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        &format!("revision {index}"),
                        &tree,
                        &parents,
                    )
                    .unwrap();
                if index == 1 {
                    let object = repo.find_object(id, None).unwrap();
                    repo.tag_lightweight("v1", &object, false).unwrap();
                }
            }
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("develop", &head, false).unwrap();
            if project == 3 {
                std::fs::write(repo.workdir().unwrap().join("file.txt"), "working change\n").unwrap();
            }
            state.activate_repository(
                repository::execute(Request {
                    operation: Operation::Open,
                    source: String::new(),
                    path: path.to_string_lossy().into(),
                    name: format!("repository-{project}"),
                })
                .unwrap(),
            );
            drive(&mut state);
        }
        state.commit = 2;
        (dir, state)
    }

    #[test]
    fn worktree_starts_common_loading_and_uses_150ms_boundary() {
        let (_dir, mut state) = workspace_with_repositories();
        state.file_history = false;
        state.refresh_panel_data();
        assert!(state.panel_waiting);
        assert!(state.panel_loading.is_pending());
        let revision = state.panel_revision;
        let start = std::time::Instant::now();
        state.panel_loading.begin(revision, start);
        assert!(
            !state
                .panel_loading
                .reveal(revision, start + std::time::Duration::from_millis(149))
        );
        assert!(!state.panel_loading.is_visible(revision));
        assert!(state.panel_loading.reveal(revision, start + LOADING_DELAY));
        assert!(state.panel_loading.is_visible(revision));
        let request = state.panel_pending.take().unwrap();
        assert!(!request.historical);
        let result = std::thread::spawn(move || request.execute())
            .join()
            .unwrap();
        assert!(state.finish_panel(result));
        assert!(!state.panel_loading.is_pending());
        assert!(!state.panel_loading.is_visible(revision));
        assert!(!state.panel_waiting);
        assert!(!state.changed_files().is_empty());
    }

    #[test]
    fn worktree_fast_completion_empty_and_error_clear_common_loading() {
        use std::sync::atomic::Ordering;
        let (_dir, mut state) = workspace_with_repositories();
        state.file_history = false;
        for outcome in 0..3 {
            state.refresh_panel_data();
            assert!(state.panel_loading.is_pending());
            let revision = state.panel_revision;
            let start = std::time::Instant::now();
            state.panel_loading.begin(revision, start);
            if outcome == 0 {
                state
                    .panel_loading
                    .completion()
                    .store(true, Ordering::Release);
                assert!(!state.panel_loading.reveal(revision, start + LOADING_DELAY));
            } else {
                assert!(state.panel_loading.reveal(revision, start + LOADING_DELAY));
            }
            let request = state.panel_pending.take().unwrap();
            let mut result = std::thread::spawn(move || request.execute())
                .join()
                .unwrap();
            if outcome == 1 {
                result.files = Ok(Vec::new());
                result.displayed_files = Ok(Vec::new());
            } else if outcome == 2 {
                result.files = Err("작업 트리 조회 오류".into());
                result.displayed_files = Err("작업 트리 조회 오류".into());
            }
            assert!(state.finish_panel(result));
            assert!(!state.panel_waiting);
            assert!(!state.panel_loading.is_pending());
            assert!(!state.panel_loading.is_visible(revision));
            if outcome == 1 {
                assert!(state.changed_files().is_empty());
            }
            if outcome == 2 {
                assert!(state.viewer_files.is_err());
            }
        }
    }

    #[test]
    fn history_to_worktree_keeps_new_loading_isolated_from_previous_request() {
        use std::sync::atomic::Ordering;
        let (_dir, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_history_row(project, 0));
        let old_revision = state.panel_revision;
        let old_completion = state.panel_loading.completion();
        let old = state.panel_pending.take().unwrap();
        state.file_history = false;
        state.refresh_panel_data();
        assert!(state.panel_loading.is_pending());
        let revision = state.panel_revision;
        let start = std::time::Instant::now();
        state.panel_loading.begin(revision, start);
        old_completion.store(true, Ordering::Release);
        assert!(!state.panel_loading.completion().load(Ordering::Acquire));
        assert!(
            !state
                .panel_loading
                .reveal(old_revision, start + LOADING_DELAY)
        );
        assert!(state.panel_loading.reveal(revision, start + LOADING_DELAY));
        let old_result = std::thread::spawn(move || old.execute()).join().unwrap();
        assert!(!state.finish_panel(old_result));
        assert!(state.panel_loading.is_visible(revision));
        drive(&mut state);
        assert!(!state.file_history);
        assert!(!state.panel_waiting);
        assert!(!state.panel_loading.is_pending());
    }

    #[test]
    fn commit_grid_loading_tracks_reference_reads_and_graph_preparation_only() {
        use std::sync::atomic::Ordering;
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.commit_grid_loading().is_none());
        state.projects[project].branches = vec!["unloaded-one".into(), "unloaded-two".into()];
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        assert!(state.commit_grid_loading().is_some());
        let revision = state.reference_revision;
        let start = std::time::Instant::now();
        state.commit_loading.begin(revision, start);
        assert!(
            !state
                .commit_loading
                .reveal(revision, start + std::time::Duration::from_millis(149))
        );
        assert!(state.commit_loading.reveal(revision, start + LOADING_DELAY));
        assert!(!state.select_repository_row(project, RepositoryTab::Local, 0));
        assert!(state.commit_loading.is_visible(revision));
        let completed = state.commit_loading.completion();
        completed.store(true, Ordering::Release);
        assert!(!state.commit_loading.is_visible(revision));
        let request = state.reference_pending.take().unwrap();
        assert!(state.finish_reference(request, Err("reference error".into())));
        assert!(state.commit_grid_loading().is_none());
        assert!(state.select_repository_row(project, RepositoryTab::Local, 1));
        assert!(state.commit_loading.is_pending());
        let next = state.reference_revision;
        assert!(!state.commit_loading.is_visible(next));
        state.select_repository_tab(RepositoryTab::Remote);
        assert!(!state.commit_loading.is_pending());
        assert!(state.commit_grid_loading().is_none());
        state.histories.insert(
            project,
            HistoryState::from_prepared(PreparedHistory::empty()),
        );
        state.panel_waiting = true;
        state.panel_loading.begin(state.panel_revision, start);
        assert!(state.commit_grid_loading().is_some());
        state.install_history(
            project,
            PreparedHistory::new(&state.projects[project].commits),
        );
        assert!(state.panel_loading.is_pending());
        assert!(state.commit_grid_loading().is_none());
    }

    #[test]
    fn history_loading_uses_shared_delay_and_clears_on_success_or_fast_completion() {
        let (_dir, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_history_row(project, 0));
        let revision = state.panel_revision;
        let start = std::time::Instant::now();
        state.panel_loading.begin(revision, start);
        assert!(!state.reveal_history_loading(
            revision,
            false,
            start + std::time::Duration::from_millis(149)
        ));
        assert!(!state.history_loading_visible());
        assert!(state.reveal_history_loading(revision, false, start + LOADING_DELAY));
        assert!(state.history_loading_visible());
        let request = state.panel_pending.take().unwrap();
        let result = std::thread::spawn(move || request.execute())
            .join()
            .unwrap();
        assert!(state.finish_panel(result));
        assert!(!state.history_loading_visible());
        assert!(!state.reveal_history_loading(
            revision,
            false,
            start + LOADING_DELAY + LOADING_DELAY
        ));
        assert!(state.select_history_row(project, 1));
        let revision = state.panel_revision;
        state.panel_loading.begin(revision, start);
        // The backend may have returned before its UI completion callback runs.
        assert!(!state.reveal_history_loading(revision, true, start + LOADING_DELAY));
        assert!(!state.history_loading_visible());
        drive(&mut state);
        assert!(!state.history_loading_visible());
    }

    #[test]
    fn obsolete_loading_timer_and_result_cannot_change_latest_loading_or_error() {
        let (_dir, mut state) = workspace_with_repositories();
        let project = state.project;
        let start = std::time::Instant::now();
        state.select_history_row(project, 0);
        let old_revision = state.panel_revision;
        let old = state.panel_pending.take().unwrap();
        state.select_history_row(project, 1);
        let revision = state.panel_revision;
        state.panel_loading.begin(revision, start + LOADING_DELAY);
        assert!(!state.reveal_history_loading(
            old_revision,
            false,
            start + LOADING_DELAY + LOADING_DELAY
        ));
        assert!(!state.history_loading_visible());
        assert!(state.reveal_history_loading(
            revision,
            false,
            start + LOADING_DELAY + LOADING_DELAY
        ));
        let old = std::thread::spawn(move || old.execute()).join().unwrap();
        assert!(!state.finish_panel(old));
        assert!(state.history_loading_visible());
        let current = state.panel_pending.take().unwrap();
        let mut result = std::thread::spawn(move || current.execute())
            .join()
            .unwrap();
        result.files = Err("history failure".into());
        result.displayed_files = Err("history failure".into());
        assert!(state.finish_panel(result));
        assert!(!state.history_loading_visible());
        assert_eq!(state.history_files.as_ref().unwrap_err(), "history failure");
        assert!(!state.reveal_history_loading(
            revision,
            false,
            start + std::time::Duration::from_secs(1)
        ));
    }

    #[test]
    fn navigation_hides_history_loading_and_rejects_previous_timer() {
        let (_dir, mut state) = workspace_with_repositories();
        for action in 0..4 {
            state.select_project(3);
            drive(&mut state);
            state.file_history = false;
            state.commit = 1;
            assert!(state.select_history_row(3, 0));
            let revision = state.panel_revision;
            let start = std::time::Instant::now();
            state.panel_loading.begin(revision, start);
            assert!(state.reveal_history_loading(revision, false, start + LOADING_DELAY));
            assert!(state.history_loading_visible());
            let panel_cancelled = state.panel_reads.observe_cancellation();
            let reference_cancelled = state.reference_reads.observe_cancellation();
            match action {
                0 => {
                    state.file_history = false;
                    state.refresh_panel_data();
                }
                1 => state.open_start_tab(),
                2 => state.select_project(0),
                _ => state.close_project(3),
            }
            assert!(panel_cancelled());
            assert!(reference_cancelled());
            assert!(!state.history_loading_visible());
            assert!(!state.reveal_history_loading(
                revision,
                false,
                start + LOADING_DELAY + LOADING_DELAY
            ));
            drive(&mut state);
        }
    }

    #[test]
    fn commit_detail_row_keeps_display_selection_during_consecutive_commit_reads() {
        let (_dir, mut state) = workspace_with_repositories();
        state.file_history = true;
        state.refresh_file_data();
        drive(&mut state);
        assert!(state.display_file_list().1);
        assert_eq!(state.file_list_selected_row(), Some(0));
        for commit in [1, 2] {
            assert!(state.select_history_row(state.project, commit));
            assert!(state.panel_waiting);
            assert!(state.display_file_list().1);
            assert_eq!(state.file_list_selected_row(), Some(0));
            assert!(!state.select_file_list_row(state.project, state.panel_revision, true, 0));
        }
        drive(&mut state);
        assert_eq!(state.file_list_selected_row(), Some(0));
    }

    #[test]
    fn worktree_file_switch_supplies_consecutive_file_targets_without_empty_target() {
        let (_dir, mut state) = workspace_with_repositories();
        let template = state.viewer_files.as_ref().unwrap()[0].clone();
        let files: Vec<_> = ["first.txt", "second.txt"].into_iter().map(|name| {
            let mut file = template.clone();
            file.path = name.as_bytes().to_vec();
            file.label = name.into();
            file
        }).collect();
        state.file_history = false;
        state.projects[state.project].files = workspace_backend::changed_files(&files, false);
        state.viewer_files = Ok(files);
        for (row, expected) in [(0, "first.txt"), (1, "second.txt")] {
            assert!(state.select_file_list_row(state.project, state.panel_revision, false, row));
            assert!(matches!(state.viewer_target(), ViewerTarget::File { file, .. } if file.label == expected));
        }
    }

    #[test]
    fn delayed_lists_retain_display_only_until_result_or_invalidation() {
        let (_dir, mut state) = workspace_with_repositories();
        state.file_history = false;
        state.projects[state.project].files = vec![ChangedFile { path: "previous.rs".into(), status: "M".into() }];
        state.refresh_file_data();
        assert!(state.changed_files().is_empty());
        assert_eq!(state.display_file_list().0[0].path, "previous.rs");
        assert!(!state.select_file_list_row(state.project, state.panel_revision, false, 0));
        state.refresh_file_data();
        assert_eq!(state.display_file_list().0[0].path, "previous.rs");
        let request = state.panel_pending.take().unwrap();
        assert!(state.finish_panel(request.execute()));
        assert!(state.retained_file_list.is_none());
        state.repository_tab = RepositoryTab::Tags;
        state.panel_references = Ok(vec!["previous-tag".into()]);
        state.refresh_repository_list();
        assert!(state.repository_names().is_empty());
        assert_eq!(state.display_repository_names(), &["previous-tag"]);
        assert!(!state.select_repository_row(state.project, RepositoryTab::Tags, 0));
        state.refresh_repository_list();
        assert_eq!(state.display_repository_names(), &["previous-tag"]);
        let request = state.repository_pending.take().unwrap();
        assert!(state.finish_repository_list(request.execute()));
        assert!(state.retained_repository_names.is_none());
        state.projects[state.project].files = vec![ChangedFile { path: "before-error.rs".into(), status: "M".into() }];
        state.refresh_file_data();
        let stale = state.panel_pending.take().unwrap().execute();
        state.refresh_file_data();
        assert!(!state.finish_panel(stale));
        assert_eq!(state.display_file_list().0[0].path, "before-error.rs");
        let mut failed = state.panel_pending.take().unwrap().execute();
        failed.files = Err("read failed".into());
        failed.displayed_files = Err("read failed".into());
        assert!(state.finish_panel(failed));
        assert!(state.retained_file_list.is_none());
        assert!(state.display_file_list().0.is_empty());
        state.refresh_file_data();
        state.refresh_repository_list();
        state.invalidate_requests();
        assert!(state.retained_file_list.is_none());
        assert!(state.retained_repository_names.is_none());
    }

    #[test]
    fn invalid_navigation_does_not_cancel_the_current_panel_request() {
        let (_dir, mut state) = workspace_with_repositories();
        assert!(state.select_history_row(state.project, 1));
        let revision = state.panel_revision;
        let cancelled = state.panel_reads.observe_cancellation();
        state.select_project(usize::MAX);
        state.close_project(usize::MAX);
        state.close_start_tab();
        assert_eq!(state.panel_revision, revision);
        assert!(!cancelled());
        assert!(state.panel_waiting && state.panel_pending.is_some());
        let request = state.panel_pending.take().unwrap();
        let result = std::thread::spawn(move || request.execute())
            .join()
            .unwrap();
        assert!(state.finish_panel(result));
        assert_eq!(state.commit, 1);
    }

    #[test]
    fn ui_selection_returns_before_delayed_work_and_latest_panel_result_wins() {
        let (_dir, mut state) = workspace_with_repositories();
        let project = state.project;
        let ui = std::thread::current().id();
        let (release, wait) = std::sync::mpsc::channel();
        let (entered, ready) = std::sync::mpsc::channel();
        let (old, new) = crate::backend::with_test_delay(50, ui, || {
            assert!(state.select_history_row(project, 0));
            assert!(state.file_history && state.commit_detail_selected());
            assert!(state.changed_files().is_empty() && state.panel_waiting);
            let old = state.panel_pending.take().unwrap();
            let worker = std::thread::spawn(move || {
                entered.send(std::thread::current().id()).unwrap();
                wait.recv().unwrap();
                crate::backend::with_test_delay(5, ui, || {
                    let result = old.execute();
                    assert!(crate::backend::test_calls().iter().all(|(_, on_ui)| !on_ui));
                    result
                })
            });
            assert_ne!(ready.recv().unwrap(), ui);
            assert!(!worker.is_finished());
            assert!(state.select_history_row(project, 1));
            assert_eq!(state.commit, 1);
            let new = state.panel_pending.take().unwrap();
            assert!(crate::backend::test_calls().is_empty());
            (worker, new)
        });
        let new = std::thread::spawn(move || new.execute()).join().unwrap();
        assert!(state.finish_panel(new));
        let files = state.viewer_files.clone().unwrap();
        let references = state.panel_references.clone().unwrap();
        release.send(()).unwrap();
        let mut old = old.join().unwrap();
        old.files = Err("late error".into());
        assert!(!state.finish_panel(old));
        assert_eq!(state.commit, 1);
        assert_eq!(state.viewer_files.as_ref().unwrap(), &files);
        assert_eq!(state.panel_references.as_ref().unwrap(), &references);
        assert!(!state.panel_waiting);
    }

    #[test]
    fn panel_results_are_rejected_after_project_mode_start_or_close_changes() {
        let (_dir, mut state) = workspace_with_repositories();
        for action in 0..4 {
            state.select_project(3);
            drive(&mut state);
            state.refresh_panel_data();
            let request = state.panel_pending.take().unwrap();
            match action {
                0 => state.select_project(0),
                1 => {
                    state.file_history = !state.file_history;
                    state.refresh_panel_data();
                }
                2 => state.open_start_tab(),
                _ => state.close_project(3),
            }
            let result = std::thread::spawn(move || request.execute())
                .join()
                .unwrap();
            assert!(!state.finish_panel(result));
            drive(&mut state);
        }
    }

    #[test]
    fn delayed_reference_results_do_not_override_new_commit_or_reference_selection() {
        let (_dir, mut state) = workspace_with_repositories();
        let project = state.project;
        state.projects[project].branches = vec!["missing-a".into(), "missing-b".into()];
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        let old = state.reference_pending.take().unwrap();
        assert!(state.select_history_row(project, 1));
        let notice = state.notice.clone();
        assert!(!state.finish_reference(old, Err("late failure".into())));
        assert_eq!(state.commit, 1);
        assert_eq!(state.notice, notice);
        assert!(state.select_repository_row(project, RepositoryTab::Local, 1));
        let old = state.reference_pending.take().unwrap();
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        let current = state.reference_pending.take().unwrap();
        let snapshot = state.projects[project].clone();
        let graph = PreparedHistory::new(&snapshot.commits);
        assert!(!state.finish_reference(old, Ok((snapshot, graph))));
        assert!(state.finish_reference(current, Err("current failure".into())));
        assert!(state.notice.contains("current failure"));
        assert_eq!(state.repository_selection.selected(2), Some(0));
    }

    #[test]
    fn all_ui_navigation_and_background_write_notifications_avoid_backend_calls() {
        let (dir, mut state) = workspace_with_repositories();
        state.session_path = Some(dir.path().join("responsive-session.json"));
        let ui = std::thread::current().id();
        crate::backend::with_test_delay(50, ui, || {
            state.select_history_row(3, 0);
            state.select_project(0);
            state.repository_tab = RepositoryTab::Tags;
            state.refresh_panel_data();
            state.open_start_tab();
            state.close_start_tab();
            state.close_project(0);
            let root = state.projects[1].path.clone().unwrap();
            state.viewer_changed(&ViewerChanged { root });
            assert!(crate::backend::test_calls().is_empty());
        });
        drive(&mut state);
        let saved = session::read(state.session_path.as_ref().unwrap()).unwrap();
        assert_eq!(saved.open, state.session_snapshot().open);
        assert_eq!(saved.active, state.session_snapshot().active);
        assert_eq!(saved.recent, state.recent_repositories);
    }

    #[test]
    fn stale_session_write_errors_do_not_replace_newer_save_status() {
        let mut state = Workspace {
            save_revision: 2,
            save_error: Some("latest".into()),
            ..Default::default()
        };
        state.finish_save(1, Ok(Err("old failure".into())));
        assert_eq!(state.save_error.as_deref(), Some("latest"));
        state.finish_save(2, Ok(Ok(())));
        assert!(state.save_error.is_none());
    }

    #[test]
    fn startup_screen_excludes_normal_workspace_until_success_or_failure() {
        let (mut state, ()) = Workspace::start_restore(|| ());
        assert_eq!(state.screen(), WorkspaceScreen::Loading);
        // Loading takes precedence even if a normal page selection is present.
        state.start_tab_active = false;
        assert_eq!(state.screen(), WorkspaceScreen::Loading);
        state.start_tab_active = true;
        state.finish_restore(None, Err("read failure".into()));
        assert_eq!(state.screen(), WorkspaceScreen::Start);
        assert!(state.session_warning().is_some());
        state.close_start_tab();
        drive(&mut state);
        assert_eq!(state.screen(), WorkspaceScreen::Empty);
        let (dir, previous) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        session::write(&path, &previous.session_snapshot()).unwrap();
        let result = RestoredWorkspace::read(&path);
        let (mut state, ()) = Workspace::start_restore(|| ());
        state.finish_restore(Some(path), result);
        assert_eq!(state.screen(), WorkspaceScreen::Project);
    }

    #[test]
    fn startup_returns_while_repository_worker_is_waiting_and_blocks_session_mutations() {
        let (dir, mut previous) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        previous.session_path = Some(path.clone());
        previous.select_project(1);
        drive(&mut previous);
        let original = std::fs::read(&path).unwrap();
        let worker_path = path.clone();
        let (release, wait) = std::sync::mpsc::channel();
        let (started, entered) = std::sync::mpsc::channel();
        let ui_thread = std::thread::current().id();
        let (mut state, worker) = Workspace::start_restore(|| {
            std::thread::spawn(move || {
                started.send(std::thread::current().id()).unwrap();
                wait.recv().unwrap();
                RestoredWorkspace::read(&worker_path)
            })
        });
        assert_ne!(entered.recv().unwrap(), ui_thread);
        assert!(!worker.is_finished());
        assert!(state.restoring && state.start_tab_active);
        assert!(state.notice.contains("복원"));
        assert!(state.projects.is_empty());
        // Even if a settings path is available, startup cannot overwrite saved tabs.
        state.session_path = Some(path.clone());
        state.open_start_tab();
        drive(&mut state);
        state.close_start_tab();
        drive(&mut state);
        state.select_project(1);
        drive(&mut state);
        state.close_project(1);
        drive(&mut state);
        state.save_session();
        drive(&mut state);
        assert!(state.start_tab_active && state.start_tab_open);
        assert!(state.open_projects.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        release.send(()).unwrap();
        state.finish_restore(Some(path.clone()), worker.join().unwrap());
        assert!(!state.restoring);
        assert_eq!(state.project, 1);
        assert_eq!(state.open_projects.len(), 4);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        state.open_start_tab();
        drive(&mut state);
        state.close_start_tab();
        drive(&mut state);
        assert!(!state.start_tab_open);
    }

    #[test]
    fn restore_application_uses_preloaded_files_without_accessing_repository_again() {
        let (dir, mut previous) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        previous.session_path = Some(path.clone());
        previous.select_project(3);
        drive(&mut previous);
        let restored = RestoredWorkspace::read(&path).unwrap();
        let expected = restored.viewer_files.as_ref().unwrap().clone();
        assert!(!expected.is_empty());
        let active_path = restored.projects[restored.project].path.clone().unwrap();
        // Destroy only this test's temporary repositories after background reading.
        drop(previous);
        dir.close().unwrap();
        assert!(!active_path.exists());
        let (mut state, ()) = Workspace::start_restore(|| ());
        state.finish_restore(Some(path), Ok(restored));
        assert!(!state.restoring);
        assert_eq!(state.viewer_files.as_ref().unwrap(), &expected);
        assert_eq!(state.projects[state.project].files.len(), expected.len());
        assert!(state.session_warning().is_none());
        assert_eq!(state.histories[&state.project].len(), 3);
    }

    #[test]
    fn failed_async_restore_releases_controls_and_preserves_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.json");
        std::fs::write(&path, "broken JSON").unwrap();
        let result = RestoredWorkspace::read(&path);
        let (mut state, ()) = Workspace::start_restore(|| ());
        state.finish_restore(Some(path.clone()), result);
        assert!(!state.restoring);
        assert!(state.start_tab_active);
        assert!(state.session_warning().unwrap().contains("손상"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken JSON");
        state.close_start_tab();
        drive(&mut state);
        assert!(!state.start_tab_open);
    }

    #[test]
    fn closing_workspace_before_delayed_restore_finishes_is_safe() {
        let (dir, previous) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        session::write(&path, &previous.session_snapshot()).unwrap();
        let original = std::fs::read(&path).unwrap();
        let worker_path = path.clone();
        let (release, wait) = std::sync::mpsc::channel();
        let (state, worker) = Workspace::start_restore(|| {
            std::thread::spawn(move || {
                wait.recv().unwrap();
                RestoredWorkspace::read(&worker_path)
            })
        });
        drop(state);
        release.send(()).unwrap();
        assert!(worker.join().unwrap().is_ok());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }

    #[test]
    fn session_restores_tab_order_selection_and_recent_without_reordering() {
        let (dir, mut state) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        state.session_path = Some(path.clone());
        state.select_project(0);
        drive(&mut state);
        state.close_project(1);
        drive(&mut state);
        let expected = state.session_snapshot();
        let restored = Workspace::load_from_path(path);
        let actual = restored.session_snapshot();
        assert_eq!(actual.open, expected.open);
        assert_eq!(actual.active, expected.active);
        assert_eq!(actual.recent, expected.recent);
        assert!(!restored.start_tab_open && !restored.start_tab_active);
        assert!(restored.session_warning().is_none());
        assert_eq!(restored.projects.len(), 3);
        assert!(!restored.histories[&restored.project].is_empty());
    }

    #[test]
    fn session_keeps_start_tab_and_closed_projects_stay_closed() {
        let (dir, mut state) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        state.session_path = Some(path.clone());
        state.select_project(1);
        drive(&mut state);
        state.open_start_tab();
        drive(&mut state);
        let mut restored = Workspace::load_from_path(path.clone());
        assert!(restored.start_tab_open && restored.start_tab_active);
        assert_eq!(restored.project, 1);
        restored.select_project(2);
        drive(&mut restored);
        let restored = Workspace::load_from_path(path.clone());
        assert!(restored.start_tab_open && !restored.start_tab_active);
        assert_eq!(restored.project, 2);
        state.close_start_tab();
        drive(&mut state);
        for index in 0..4 {
            state.close_project(index);
            drive(&mut state);
        }
        let empty = Workspace::load_from_path(path);
        assert!(empty.projects.is_empty());
        assert!(empty.start_tab_open && empty.start_tab_active);
        assert_eq!(empty.recent_repositories.len(), 4);
    }

    #[test]
    fn session_skips_missing_repositories_and_duplicate_tabs_without_losing_recent() {
        let (dir, state) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        let mut saved = state.session_snapshot();
        let missing = RepositoryEntry {
            name: "없어진 저장소".into(),
            path: dir.path().join("missing"),
        };
        saved.open.insert(1, missing.clone());
        saved.open.push(saved.open[0].clone());
        saved.active = Some(missing.path.clone());
        saved.recent.insert(0, missing.clone());
        session::write(&path, &saved).unwrap();
        let restored = Workspace::load_from_path(path);
        assert_eq!(restored.open_projects.len(), 4);
        assert_eq!(restored.projects.len(), 4);
        assert!(restored.open_projects.contains(&restored.project));
        assert!(
            restored
                .session_warning()
                .unwrap()
                .contains("없어진 저장소")
        );
        assert_eq!(restored.recent_repositories, saved.recent);
        assert!(!missing.path.exists());
    }

    #[test]
    fn missing_corrupt_and_unreadable_session_fall_back_without_crashing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.json");
        let first = Workspace::load_from_path(path.clone());
        assert!(first.start_tab_active && first.projects.is_empty());
        assert!(first.session_warning().is_none());
        std::fs::write(&path, "broken JSON").unwrap();
        let corrupt = Workspace::load_from_path(path.clone());
        assert!(corrupt.start_tab_active && corrupt.projects.is_empty());
        assert!(corrupt.session_warning().unwrap().contains("손상"));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "broken JSON");
        let unreadable = Workspace::load_from_path(dir.path().to_path_buf());
        assert!(unreadable.session_warning().is_some());
    }

    #[test]
    fn session_write_failure_is_visible_and_does_not_block_tabs() {
        let (dir, mut state) = workspace_with_repositories();
        state.session_path = Some(dir.path().to_path_buf());
        state.select_project(0);
        drive(&mut state);
        assert_eq!(state.project, 0);
        assert!(state.session_warning().unwrap().contains("저장 실패"));
        state.close_project(0);
        drive(&mut state);
        assert!(!state.open_projects.contains(&0));
        state.session_path = Some(dir.path().join("workspace.json"));
        state.open_start_tab();
        drive(&mut state);
        assert!(state.save_error.is_none());
        assert!(session::read(state.session_path.as_ref().unwrap()).is_ok());
    }

    #[test]
    fn repository_path_display_keeps_request_and_session_paths_unchanged() {
        for (source, expected) in [
            (r"\\?\C:\한글 저장소", r"C:\한글 저장소"),
            (r"\\?\UNC\server\share\repo", r"\\server\share\repo"),
        ] {
            let entry = RepositoryEntry {
                name: "repo".into(),
                path: std::path::PathBuf::from(source),
            };
            let mut state = Workspace {
                projects: vec![Project {
                    name: entry.name.clone(),
                    path: Some(entry.path.clone()),
                    ..Project::default()
                }],
                open_projects: vec![0],
                start_tab_active: false,
                recent_repositories: vec![entry.clone()],
                ..Workspace::default()
            };
            state.update_tab_notice();
            assert_eq!(state.notice, expected);
            assert_eq!(display_path(&state.recent_repositories[0].path), expected);
            assert_eq!(
                Workspace::entry_request(&state.recent_repositories[0]).path,
                source
            );
            let snapshot = state.session_snapshot();
            assert_eq!(snapshot.open[0].path, entry.path);
            assert_eq!(snapshot.recent[0].path, entry.path);
            assert_eq!(state.projects[0].path.as_ref(), Some(&entry.path));
        }
    }

    #[test]
    fn global_repository_operations_reject_duplicates_and_clear_on_failure() {
        let (_repositories, mut state) = workspace_with_repositories();
        let before = serde_json::to_value(state.session_snapshot()).unwrap();
        let project = state.project;
        let commit = state.commit;
        for operation in [Operation::Open, Operation::Create, Operation::Clone] {
            assert!(state.begin_repository_operation(operation));
            assert_eq!(state.repository_operation, Some(operation));
            assert!(!state.begin_repository_operation(Operation::Open));
            assert!(!state.select_file_tab(true));
            assert!(!state.select_repository_tab(RepositoryTab::Tags));
            assert!(!state.select_history_row(project, 0));
            assert!(!state.select_file_list_row(project, state.panel_revision, false, 0));
            assert!(!state.select_repository_row(project, RepositoryTab::Local, 0));
            state.open_start_tab();
            state.select_project(0);
            state.close_project(project);
            assert_eq!(state.project, project);
            assert_eq!(state.commit, commit);
            assert!(!state.start_tab_active);
            assert_eq!(
                serde_json::to_value(state.session_snapshot()).unwrap(),
                before
            );
            assert!(
                state
                    .finish_repository_operation(Err(anyhow::anyhow!("저장소 작업 실패")))
                    .is_err()
            );
            assert!(state.repository_operation.is_none());
            assert_eq!(
                serde_json::to_value(state.session_snapshot()).unwrap(),
                before
            );
        }
    }

    #[test]
    fn global_repository_operations_finish_success_without_a_dialog() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.projects[state.project].clone();
        let count = state.open_projects.len();
        for operation in [Operation::Open, Operation::Create, Operation::Clone] {
            assert!(state.repository_dialog.is_none());
            assert!(state.begin_repository_operation(operation));
            state
                .finish_repository_operation(Ok(project.clone()))
                .unwrap();
            assert!(state.repository_operation.is_none());
            assert!(state.repository_dialog.is_none());
            drive(&mut state);
            assert_eq!(state.open_projects.len(), count);
            assert!(!state.start_tab_active);
        }
    }

    #[test]
    fn global_repository_operation_is_blocked_during_restore_or_alert() {
        let mut state = Workspace {
            restoring: true,
            ..Workspace::default()
        };
        assert!(!state.begin_repository_operation(Operation::Open));
        assert!(state.repository_operation.is_none());
        state.restoring = false;
        state.alert = Some(Alert::new("오류", "먼저 확인하세요"));
        assert!(!state.begin_repository_operation(Operation::Create));
        assert!(state.repository_operation.is_none());
        state.alert = None;
        assert!(state.begin_repository_operation(Operation::Clone));
        assert!(
            state
                .finish_repository_operation(Err(anyhow::anyhow!("실패")))
                .is_err()
        );
        assert!(state.repository_operation.is_none());
    }

    #[test]
    fn recent_reopen_reuses_tabs_and_failed_open_preserves_saved_state() {
        let (dir, mut state) = workspace_with_repositories();
        let path = dir.path().join("workspace.json");
        state.session_path = Some(path.clone());
        state.select_project(1);
        drive(&mut state);
        let entry = state.recent_repositories[0].clone();
        state.close_project(1);
        drive(&mut state);
        state.open_start_tab();
        drive(&mut state);
        let project = repository::execute(Workspace::entry_request(&entry));
        state.repository_operation = Some(Operation::Open);
        state.finish_repository_operation(project).unwrap();
        drive(&mut state);
        assert!(state.repository_operation.is_none());
        assert_eq!(state.open_projects.len(), 4);
        assert_eq!(state.projects.len(), 4);
        assert_eq!(state.project, 1);
        assert!(!state.start_tab_active);
        assert_eq!(state.recent_repositories[0], entry);
        let project = repository::execute(Workspace::entry_request(&entry));
        state.finish_repository_operation(project).unwrap();
        drive(&mut state);
        assert_eq!(state.open_projects.len(), 4);
        assert_eq!(state.recent_repositories.len(), 4);
        let before = std::fs::read(&path).unwrap();
        let snapshot = serde_json::to_value(state.session_snapshot()).unwrap();
        let missing = RepositoryEntry {
            name: "missing".into(),
            path: dir.path().join("missing"),
        };
        state.repository_operation = Some(Operation::Open);
        let result = repository::execute(Workspace::entry_request(&missing));
        assert!(state.finish_repository_operation(result).is_err());
        drive(&mut state);
        assert!(state.repository_operation.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(
            serde_json::to_value(state.session_snapshot()).unwrap(),
            snapshot
        );
        let restored = Workspace::load_from_path(path);
        assert_eq!(restored.recent_repositories[0], entry);
    }

    #[test]
    fn new_workspace_has_only_an_active_start_tab() {
        let mut state = Workspace::default();
        assert!(state.projects.is_empty());
        assert!(state.open_projects.is_empty());
        assert!(state.histories.is_empty());
        assert!(state.start_tab_open && state.start_tab_active);
        assert_eq!(state.commit, 0);
        assert!(state.footer_directory().is_none());
        state.close_start_tab();
        drive(&mut state);
        state.select_project(99);
        drive(&mut state);
        state.close_project(99);
        drive(&mut state);
        assert!(state.open_projects.is_empty());
        state.open_start_tab();
        drive(&mut state);
        assert!(state.start_tab_active);
    }
    #[test]
    fn panel_scrolls_are_separate_per_project_and_tab_and_removed_on_close() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let key = ("branches", project, 0);
        let first = state.panel_scrolls.entry(key).or_default().clone();
        let remote = state
            .panel_scrolls
            .entry(("branches", project, 1))
            .or_default()
            .clone();
        let files = state
            .panel_scrolls
            .entry(("files", project, 0))
            .or_default()
            .clone();
        assert!(!std::rc::Rc::ptr_eq(&first.vertical.0, &remote.vertical.0));
        assert!(!std::rc::Rc::ptr_eq(&first.vertical.0, &files.vertical.0));
        first
            .vertical
            .0
            .borrow()
            .base_handle
            .set_offset(gpui::point(px(0.), px(-48.)));
        state.select_project(0);
        drive(&mut state);
        state.select_project(project);
        drive(&mut state);
        assert_eq!(
            state.panel_scrolls[&key]
                .vertical
                .0
                .borrow()
                .base_handle
                .offset()
                .y,
            px(-48.)
        );
        state.close_project(project);
        drive(&mut state);
        assert!(
            !state
                .panel_scrolls
                .keys()
                .any(|(_, owner, _)| *owner == project)
        );
    }

    #[test]
    fn select_all_shortcut_requires_only_control_a() {
        assert!(select_all_files_shortcut(&gpui::Keystroke::parse("ctrl-a").unwrap()));
        for key in ["a", "ctrl-shift-a", "ctrl-alt-a", "cmd-a", "ctrl-d"] {
            assert!(!select_all_files_shortcut(&gpui::Keystroke::parse(key).unwrap()), "{key}");
        }
    }

    #[test]
    fn select_all_worktree_files_handles_repeated_empty_and_blocked_states() {
        let (_repos, mut state) = workspace_with_repositories();
        let template = state.viewer_files.as_ref().unwrap()[0].clone();
        let files: Vec<_> = (0..100).map(|index| {
            let mut file = template.clone();
            file.path = format!("file-{index}.txt").into_bytes();
            file.label = format!("file-{index}.txt");
            file.staged = index % 2 == 0;
            file
        }).collect();
        state.projects[state.project].files = workspace_backend::changed_files(&files, false);
        state.viewer_files = Ok(files.clone());
        state.file_selection.select(3, files.len());
        let count = state.changed_files().len();
        assert_eq!(count, 100);
        assert!(state.select_all_worktree_files());
        assert_eq!(state.file_selection.indices(count), (0..count).collect::<Vec<_>>());
        let ViewerTarget::Files { files: targets, .. } = state.viewer_target() else {
            panic!("다중 파일 뷰어로 연결되어야 합니다.");
        };
        assert_eq!(targets, files);
        assert!(state.select_all_worktree_files());
        let before = state.file_selection.clone();
        state.file_history = true;
        assert!(!state.select_all_worktree_files());
        state.file_history = false;
        state.panel_waiting = true;
        assert!(!state.select_all_worktree_files());
        state.panel_waiting = false;
        state.footer_action_busy = true;
        assert!(!state.select_all_worktree_files());
        state.footer_action_busy = false;
        state.start_tab_active = true;
        assert!(!state.select_all_worktree_files());
        state.start_tab_active = false;
        state.viewer_files = Err("읽기 실패".into());
        assert!(!state.select_all_worktree_files());
        assert_eq!(state.file_selection, before);
        state.viewer_files = Ok(Vec::new());
        state.projects[state.project].files.clear();
        assert!(state.select_all_worktree_files());
        assert!(state.file_selection.indices(count).is_empty());
    }

    #[test]
    fn worktree_modifier_selection_drives_multiple_targets_and_restores_identities() {
        let (_repos, mut state) = workspace_with_repositories();
        let project = state.project;
        let template = state.viewer_files.as_ref().unwrap()[0].clone();
        let files: Vec<_> = (0..6)
            .map(|index| {
                let mut file = template.clone();
                file.path = format!("file-{index}.txt").into_bytes();
                file.label = format!("file-{index}.txt");
                file.staged = index == 0;
                file
            })
            .collect();
        state.projects[project].files = workspace_backend::changed_files(&files, false);
        state.viewer_files = Ok(files.clone());
        let revision = state.panel_revision;
        let plain = gpui::Modifiers::default();
        let shift = gpui::Modifiers {
            shift: true,
            ..plain
        };
        let ctrl = gpui::Modifiers {
            control: true,
            ..plain
        };
        assert!(state.select_file_list_row_with_modifiers(project, revision, false, 1, plain));
        assert!(state.select_file_list_row_with_modifiers(project, revision, false, 4, shift));
        assert_eq!(state.file_selection.indices(6), [1, 2, 3, 4]);
        let ViewerTarget::Files { files: targets, .. } = state.viewer_target() else {
            panic!("다중 파일 대상이어야 합니다.");
        };
        assert_eq!(targets, files[1..5]);
        state.select_file_list_row_with_modifiers(project, revision, false, 2, ctrl);
        assert_eq!(state.file_selection.indices(6), [1, 3, 4]);
        let before = state.file_selection.clone();
        assert!(!state.select_file_list_row_with_modifiers(
            project,
            revision.wrapping_sub(1),
            false,
            5,
            ctrl
        ));
        assert_eq!(state.file_selection, before);
        state.select_file_list_row_with_modifiers(project, revision, false, 4, plain);
        assert!(matches!(state.viewer_target(), ViewerTarget::File { .. }));
        state.select_file_list_row_with_modifiers(project, revision, false, 4, ctrl);
        assert!(matches!(state.viewer_target(), ViewerTarget::Text { .. }));
        state.file_selection.restore([1, 4], 6);
        let root = state.projects[project].path.clone().unwrap();
        let before = state.viewer_target();
        state.viewer_changed(&ViewerChanged { root });
        assert!(state.panel_waiting);
        assert_eq!(state.file_selection.indices(6), [1, 4]);
        assert_eq!(state.viewer_target(), before);
        assert!(state.viewer_refresh_pending);
        assert!(state.panel_loading.is_pending());
        assert!(!state.panel_loading.reveal(state.panel_revision, Instant::now() + LOADING_DELAY));
        assert!(state.selected_stage_requests().is_none());
        let mut request = state.panel_pending.take().unwrap();
        assert_eq!(
            request.selected_files,
            [
                (files[1].path.clone(), false),
                (files[4].path.clone(), false)
            ]
        );
        assert!(request.graph.is_none());
        let mut reversed: Vec<_> = files.iter().cloned().rev().collect();
        reversed[1].staged = true;
        request.selected_files[1].1 = true;
        assert!(state.finish_panel(PanelResult {
            work_state: Ok(Default::default()),
            displayed_files: Ok(workspace_backend::changed_files(&reversed, false)),
            files: Ok(reversed),
            history: None,
            request
        }));
        assert_eq!(state.file_selection.indices(6), [1, 4]);
        let ViewerTarget::Files { files: targets, .. } = state.viewer_target() else {
            panic!("다중 선택이 복원되어야 합니다.");
        };
        let mut staged = files[4].clone();
        staged.staged = true;
        assert_eq!(targets, [staged, files[1].clone()]);

        let root = state.projects[project].path.clone().unwrap();
        state.viewer_changed(&ViewerChanged { root });
        let request = state.panel_pending.take().unwrap();
        assert!(state.finish_panel(PanelResult {
            request,
            work_state: Ok(Default::default()),
            files: Err("read failed".into()),
            history: None,
            displayed_files: Err("read failed".into()),
        }));
        assert!(state.file_selection.indices(6).is_empty());
        assert!(matches!(state.viewer_target(), ViewerTarget::Text { lines, .. } if lines == ["read failed"]));
        state.file_history = true;
        state.history_files = Ok(workspace_backend::changed_files(&files, true));
        state.viewer_files = Ok(files);
        let revision = state.panel_revision;
        state.select_file_list_row_with_modifiers(project, revision, true, 2, shift);
        state.select_file_list_row_with_modifiers(project, revision, true, 4, ctrl);
        assert_eq!(state.file_selection.indices(6), [3]);
        assert!(matches!(
            state.viewer_target(),
            ViewerTarget::File {
                historical: true,
                ..
            }
        ));
    }

    #[test]
    fn file_status_colors_are_unchanged_by_stage_and_deletions_are_red() {
        for status in ["A", "D", "M", "U", "R", "T"] {
            assert_eq!(
                file_status_color(status),
                file_status_color(&format!("S·{status}"))
            );
        }
        assert_eq!(file_status_color("A"), ACCENT);
        assert_eq!(file_status_color("D"), 0xffa7b2);
        assert_eq!(file_status_color("S·D"), 0xffa7b2);
        assert_eq!(file_status_color("M"), 0xe6bc73);
    }

    #[test]
    fn right_click_selects_unselected_files_but_keeps_existing_multi_selection() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let mut files = Vec::new();
        for index in 0..3 {
            let mut file = state.viewer_files.as_ref().unwrap()[0].clone();
            file.path = format!("file{index}.txt").into_bytes();
            file.label = format!("file{index}.txt");
            files.push(file);
        }
        state.projects[project].files = workspace_backend::changed_files(&files, false);
        state.viewer_files = Ok(files.clone());
        let request = FileMenuRequest {
            project,
            revision: state.panel_revision,
            historical: false,
            row: 1,
        };
        state.file_selection.restore([0, 2], 3);
        assert_eq!(
            state.prepare_file_menu_selection(FileMenuRequest { row: 2, ..request }),
            Some(false)
        );
        assert_eq!(state.file_selection.indices(3), [0, 2]);
        assert_eq!(state.prepare_file_menu_selection(request), Some(true));
        assert_eq!(state.file_selection.indices(3), [1]);
        assert_eq!(state.prepare_file_menu_selection(request), Some(false));
        assert_eq!(
            state.prepare_file_menu_selection(FileMenuRequest {
                revision: request.revision.wrapping_sub(1),
                ..request
            }),
            None
        );
        assert_eq!(state.file_selection.indices(3), [1]);
        state.file_selection.clear();
        assert_eq!(state.prepare_file_menu_selection(request), Some(true));
        assert_eq!(state.panel_revision, request.revision);
        assert!(state.panel_pending.is_none());
        state.file_history = true;
        state.history_files = Ok(workspace_backend::changed_files(&files, true));
        state.file_selection.clear();
        let history = FileMenuRequest {
            historical: true,
            ..request
        };
        assert_eq!(
            state.prepare_file_menu_selection(FileMenuRequest { row: 0, ..history }),
            None
        );
        assert!(state.file_selection.indices(3).is_empty());
        assert_eq!(
            state.prepare_file_menu_selection(FileMenuRequest { row: 2, ..history }),
            Some(true)
        );
        assert_eq!(state.file_selection.indices(3), [1]);
        state.panel_waiting = true;
        assert_eq!(state.prepare_file_menu_selection(history), None);
        assert_eq!(state.file_selection.indices(3), [1]);
    }

    #[test]
    fn file_row_menu_separates_clicked_file_from_selected_paths_in_both_tabs() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let root = state.projects[project].workdir.clone().unwrap();
        let mut first = state.viewer_files.as_ref().unwrap()[0].clone();
        first.path = b"folder/one.txt".to_vec();
        first.label = "folder/one.txt".into();
        first.staged = false;
        let mut second = first.clone();
        second.path = "한 글[1].txt".as_bytes().to_vec();
        second.label = "한 글[1].txt".into();
        let mut staged = first.clone();
        staged.staged = true;
        let files = vec![first.clone(), second.clone(), staged];
        state.projects[project].files = workspace_backend::changed_files(&files, false);
        state.viewer_files = Ok(files.clone());
        state.file_selection.restore([0, 2], 3);
        let request = FileMenuRequest {
            project,
            revision: state.panel_revision,
            historical: false,
            row: 1,
        };
        assert_eq!(state.file_menu_row(request), Some(&second));
        let target = state.viewer_target();
        let commit = state.commit;
        assert_eq!(state.file_history_target(request).unwrap().1, second);
        assert_eq!(state.viewer_target(), target);
        assert_eq!(state.commit, commit);
        assert_eq!(state.file_selection.indices(3), [0, 2]);
        assert_eq!(
            state
                .selected_absolute_paths(project, request.revision, false)
                .unwrap(),
            [root.join("folder/one.txt")]
        );
        assert_eq!(state.file_selection.indices(3), [0, 2]);
        assert!(
            state
                .file_menu_row(FileMenuRequest {
                    row: usize::MAX,
                    ..request
                })
                .is_none()
        );
        assert!(
            state
                .file_menu_row(FileMenuRequest {
                    revision: request.revision.wrapping_sub(1),
                    ..request
                })
                .is_none()
        );
        assert!(
            state
                .file_menu_row(FileMenuRequest {
                    project: project + 1,
                    ..request
                })
                .is_none()
        );
        state.file_selection.clear();
        assert!(state.file_menu_row(request).is_some());
        assert!(
            state
                .selected_absolute_paths(project, request.revision, false)
                .is_none()
        );
        state.file_history = true;
        state.history_files = Ok(workspace_backend::changed_files(&files, true));
        assert!(state.file_list_has_detail());
        let history = FileMenuRequest {
            historical: true,
            row: 0,
            ..request
        };
        assert!(state.file_menu_row(history).is_none());
        assert_eq!(
            state.file_menu_row(FileMenuRequest { row: 2, ..history }),
            Some(&second)
        );
        assert!(state.file_menu_row(request).is_none());
        state.file_selection.restore([0, 1], 3);
        assert_eq!(
            state
                .selected_absolute_paths(project, request.revision, true)
                .unwrap(),
            [root.join("folder/one.txt"), root.join("한 글[1].txt")]
        );
        state.panel_waiting = true;
        assert!(
            state
                .file_menu_row(FileMenuRequest { row: 2, ..history })
                .is_none()
        );
        assert!(
            state
                .selected_absolute_paths(project, request.revision, true)
                .is_none()
        );
        state.panel_waiting = false;
        state.file_stage_busy = true;
        assert!(
            state
                .file_menu_row(FileMenuRequest { row: 2, ..history })
                .is_none()
        );
    }

    #[test]
    fn copy_paths_shortcut_requires_control_shift_c() {
        assert!(copy_paths_shortcut(
            &gpui::Keystroke::parse("ctrl-shift-c").unwrap()
        ));
        for key in [
            "c",
            "ctrl-c",
            "shift-c",
            "ctrl-alt-shift-c",
            "cmd-shift-c",
            "ctrl-shift-d",
            "space",
        ] {
            assert!(
                !copy_paths_shortcut(&gpui::Keystroke::parse(key).unwrap()),
                "{key}"
            );
        }
    }

    #[test]
    fn delayed_progress_waits_for_threshold_and_restarts_after_completion() {
        let mut state = Workspace::default();
        let start = Instant::now();
        let before_delay = LOADING_DELAY - std::time::Duration::from_millis(1);
        let schedule = || gpui::Task::ready(());

        state.track_delayed_progress(true, start, schedule);
        state.track_delayed_progress(true, start + before_delay, || panic!("timer restarted"));
        assert!(!state.delayed_progress_visible(start + before_delay));
        assert!(state.delayed_progress_visible(start + LOADING_DELAY));
        state.track_delayed_progress(false, start + LOADING_DELAY, || panic!("timer scheduled"));
        assert!(!state.delayed_progress_visible(start + LOADING_DELAY));

        state.track_delayed_progress(true, start + LOADING_DELAY, schedule);
        state.track_delayed_progress(false, start + LOADING_DELAY + before_delay, schedule);
        assert!(!state.delayed_progress_visible(start + LOADING_DELAY * 2));

        let next = start + LOADING_DELAY * 2;
        state.track_delayed_progress(true, next, schedule);
        assert!(!state.delayed_progress_visible(next));
        assert!(state.delayed_progress_visible(next + LOADING_DELAY));
    }

    #[test]
    fn discard_shortcut_requires_only_control_d() {
        assert!(discard_shortcut(&gpui::Keystroke::parse("ctrl-d").unwrap()));
        for key in [
            "d",
            "shift-d",
            "alt-d",
            "ctrl-shift-d",
            "ctrl-alt-d",
            "cmd-d",
            "ctrl-space",
            "enter",
        ] {
            assert!(
                !discard_shortcut(&gpui::Keystroke::parse(key).unwrap()),
                "{key}"
            );
        }
    }

    #[test]
    fn discard_confirmation_cancel_and_stale_project_do_not_execute_or_refresh() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let root = state.projects[project].path.clone().unwrap();
        let files = state.viewer_files.as_ref().unwrap().clone();
        let index = std::fs::read(root.join("index")).unwrap();
        let revision = state.panel_revision;
        let target = state.viewer_target();
        let prepare = || diff_git::prepare_discard(&root, &files).unwrap();
        state.discard_prompt = Some(DiscardPrompt {
            project,
            root: root.clone(),
            preview: prepare(),
        });
        assert!(state.repository_busy());
        assert!(!state.global_progress_visible());
        assert!(state.selected_stage_requests().is_none());
        assert!(!state.select_history_row(project, 0));
        assert!(state.take_discard_choice(false).is_none());
        assert!(state.discard_prompt.is_none());
        assert!(!state.file_stage_busy);
        assert!(state.take_discard_choice(true).is_none());
        assert_eq!(state.panel_revision, revision);
        assert_eq!(state.viewer_target(), target);
        assert_eq!(std::fs::read(root.join("index")).unwrap(), index);
        state.discard_prompt = Some(DiscardPrompt {
            project,
            root: root.clone(),
            preview: prepare(),
        });
        state.project += 1;
        assert!(state.take_discard_choice(true).is_none());
        state.project = project;
        state.discard_prompt = Some(DiscardPrompt {
            project,
            root: root.clone(),
            preview: prepare(),
        });
        assert!(state.take_discard_choice(true).is_some());
        assert!(state.take_discard_choice(true).is_none());
        assert!(discard_cancel_key("escape"));
        assert!(!discard_cancel_key("enter"));
        assert!(!discard_cancel_key("space"));
        assert_eq!(std::fs::read(root.join("index")).unwrap(), index);
    }

    #[test]
    fn worktree_menu_rejects_empty_or_stale_selection_and_preserves_zero_header() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let revision = state.panel_revision;
        state.file_selection.clear();
        assert!(
            matches!(state.viewer_target(), ViewerTarget::Text { title, .. } if title == "선택된 파일 0개")
        );
        assert!(state.menu_stage_requests(project, revision).is_none());
        assert!(state.file_selection.select(0, state.changed_files().len()));
        assert!(state.menu_stage_requests(project, revision).is_some());
        assert!(state.menu_stage_requests(project + 1, revision).is_none());
        assert!(
            state
                .menu_stage_requests(project, revision.wrapping_sub(1))
                .is_none()
        );
        assert_eq!(
            state.file_selection.selected(state.changed_files().len()),
            Some(0)
        );
        state.file_stage_busy = true;
        assert!(state.menu_stage_requests(project, revision).is_none());
        state.file_stage_busy = false;
        state.file_history = true;
        assert!(state.menu_stage_requests(project, revision).is_none());
    }

    #[test]
    fn files_space_requests_require_current_worktree_selection_and_idle_state() {
        let (_repos, mut state) = workspace_with_repositories();
        let template = state.viewer_files.as_ref().unwrap()[0].clone();
        let files: Vec<_> = (0..100).map(|index| {
            let mut file = template.clone();
            file.path = format!("file-{index}.txt").into_bytes();
            file.label = format!("file-{index}.txt");
            file.staged = index % 2 == 0;
            file
        }).collect();
        state.projects[state.project].files = workspace_backend::changed_files(&files, false);
        state.viewer_files = Ok(files.clone());
        state.file_selection.select(3, files.len());
        let count = state.changed_files().len();
        assert_eq!(count, 100);
        assert!(state.selected_stage_requests().is_none());
        state.file_selection.select(0, count);
        assert!(state.selected_stage_requests().is_some());
        state.file_stage_busy = true;
        assert!(state.selected_stage_requests().is_none());
        state.file_stage_busy = false;
        state.panel_waiting = true;
        assert!(state.selected_stage_requests().is_none());
        state.panel_waiting = false;
        state.file_history = true;
        assert!(state.selected_stage_requests().is_none());
        state.file_history = false;
        state.start_tab_active = true;
        assert!(state.selected_stage_requests().is_none());
        state.start_tab_active = false;
        assert!(state.selected_stage_requests().is_some());
        for key in ["shift-space", "ctrl-space", "alt-space", "enter"] {
            assert!(!super::plain_space(&gpui::Keystroke::parse(key).unwrap()));
        }
        assert!(super::plain_space(
            &gpui::Keystroke::parse("space").unwrap()
        ));
    }

    #[test]
    fn repeated_repository_rows_preserve_pending_and_completed_state() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        for tab in [
            RepositoryTab::Local,
            RepositoryTab::Remote,
            RepositoryTab::Tags,
            RepositoryTab::Stashes,
        ] {
            state.repository_tab = tab;
            state.repository_selection.clear();
            state.projects[project].branches = vec!["unloaded-reference".into()];
            state.projects[project].remotes = vec!["origin/unloaded-reference".into()];
            state.panel_references = Ok(vec!["unloaded-reference".into()]);
            assert!(state.select_repository_row(project, tab, 0));
            let revision = state.reference_revision;
            let panel_revision = state.panel_revision;
            let pending = state
                .reference_pending
                .as_ref()
                .map(|request| request.full_name.clone());
            let cancelled = state.reference_reads.observe_cancellation();
            let target = state.viewer_target();
            let notice = state.notice.clone();
            assert!(!state.select_repository_row(project, tab, 0));
            assert_eq!(state.reference_revision, revision);
            assert_eq!(state.panel_revision, panel_revision);
            assert_eq!(
                state
                    .reference_pending
                    .as_ref()
                    .map(|request| request.full_name.clone()),
                pending
            );
            assert!(!cancelled());
            assert_eq!(state.viewer_target(), target);
            assert_eq!(state.notice, notice);
            state.reference_pending = None;
            assert!(!state.select_repository_row(project, tab, 0));
            assert_eq!(state.reference_revision, revision);
            assert!(state.reference_pending.is_none());
        }
    }

    #[test]
    fn repeated_worktree_commit_preserves_mode_selection_and_requests() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(!state.file_history);
        assert!(state.file_selection.select(0, state.changed_files().len()));
        let target = state.viewer_target();
        let revision = state.panel_revision;
        let reference_revision = state.reference_revision;
        assert!(!state.select_history_row(project, state.commit));
        assert!(!state.file_history);
        assert_eq!(state.panel_revision, revision);
        assert_eq!(state.reference_revision, reference_revision);
        assert_eq!(
            state.file_selection.selected(state.changed_files().len()),
            Some(0)
        );
        assert_eq!(state.viewer_target(), target);
        assert!(state.select_history_row(project, 1));
        assert!(state.file_history);
    }

    #[test]
    fn repeated_history_row_preserves_file_viewer_scroll_and_pending_request() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_history_row(project, 1));
        // 조회 중인 동일 행도 요청을 교체하거나 취소하지 않는다.
        let revision = state.panel_revision;
        let reference_revision = state.reference_revision;
        let cancelled = state.panel_reads.observe_cancellation();
        assert!(state.panel_pending.is_some());
        assert!(!state.select_history_row(project, 1));
        assert_eq!(state.panel_revision, revision);
        assert_eq!(state.reference_revision, reference_revision);
        assert!(state.panel_pending.is_some());
        assert!(!cancelled());
        drive(&mut state);
        let files = state.changed_files().len();
        assert!(files > 0);
        assert!(state.file_selection.select(0, files));
        let target = state.viewer_target();
        let revision = state.panel_revision;
        let scroll = state
            .panel_scrolls
            .entry(("files", project, 1))
            .or_default()
            .clone();
        scroll.horizontal.set_offset(gpui::point(px(-29.), px(0.)));
        assert!(!state.select_history_row(project, 1));
        assert_eq!(state.panel_revision, revision);
        assert!(state.panel_pending.is_none());
        assert_eq!(state.file_selection.selected(files), Some(0));
        assert_eq!(state.viewer_target(), target);
        assert_eq!(scroll.horizontal.offset().x, px(-29.));
        assert!(state.select_history_row(project, 0));
        drive(&mut state);
        assert!(state.commit_detail_selected());
    }

    #[test]
    fn commit_row_selection_opens_history_and_selects_detail_from_any_file_selection() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(!state.file_history);
        for index in [1, 0, 1] {
            let files = state.changed_files().len();
            assert!(files > 0);
            assert!(state.file_selection.select(0, files));
            let expected = state.projects[project].commits[index].id.clone();
            assert!(state.select_history_row(project, index));
            drive(&mut state);
            assert!(state.file_history);
            assert_eq!(state.commit, index);
            assert!(state.commit_detail_selected());
            assert_eq!(
                state.file_selection.selected(state.changed_files().len()),
                None
            );
            assert!(
                matches!(state.viewer_target(), ViewerTarget::Detail { id, .. } if id == expected)
            );
        }
    }

    #[test]
    fn invalid_commit_row_events_preserve_panel_commit_and_file_selection() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let files = state.changed_files().len();
        assert!(state.file_selection.select(0, files));
        let original_commit = state.commit;
        for (owner, index) in [(project + 1, 0), (project, usize::MAX)] {
            assert!(!state.select_history_row(owner, index));
            drive(&mut state);
            assert!(!state.file_history);
            assert_eq!(state.commit, original_commit);
            assert_eq!(state.file_selection.selected(files), Some(0));
        }
        state.start_tab_active = true;
        assert!(!state.select_history_row(project, 0));
        drive(&mut state);
        state.start_tab_active = false;
        state.open_projects.retain(|&owner| owner != project);
        assert!(!state.select_history_row(project, 0));
        drive(&mut state);
        assert!(!state.file_history);
        assert_eq!(state.commit, original_commit);
        assert_eq!(state.file_selection.selected(files), Some(0));
    }

    #[test]
    fn commit_row_detail_stays_selected_when_history_file_lookup_fails() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let missing_id = "0".repeat(40);
        state.projects[project].commits[0].id = missing_id.clone();
        let files = state.changed_files().len();
        assert!(state.file_selection.select(0, files));
        assert!(state.select_history_row(project, 0));
        drive(&mut state);
        assert!(state.file_history);
        assert!(state.history_files.is_err());
        assert!(state.commit_detail_selected());
        assert!(
            matches!(state.viewer_target(), ViewerTarget::Detail { id, .. } if id == missing_id)
        );
    }

    #[test]
    fn reference_selection_preserves_worktree_panel_mode() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(!state.file_history);
        assert!(!state.repository_names().is_empty());
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        drive(&mut state);
        assert!(!state.file_history);
        assert!(!state.commit_detail_selected());
        assert_eq!(state.commit, 0);
        assert!(!state.changed_files().is_empty());
    }

    #[test]
    fn detail_list_item_only_appears_after_successful_history_load() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(!state.file_list_has_detail());
        assert!(state.select_history_row(project, 0));
        assert!(state.panel_waiting);
        assert!(!state.file_list_has_detail());
        assert_eq!(state.file_list_len(), 0);
        assert_eq!(state.file_list_selected_row(), None);
        assert!(!state.select_file_list_row(project, state.panel_revision, true, 0));
        drive(&mut state);
        assert!(state.file_list_has_detail());
        assert_eq!(state.file_list_len(), state.changed_files().len() + 1);
        assert_eq!(state.file_list_selected_row(), Some(0));
        assert!(matches!(state.viewer_target(), ViewerTarget::Detail { .. }));
    }

    #[test]
    fn detail_list_item_handles_empty_error_missing_commit_and_worktree() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_history_row(project, 0));
        drive(&mut state);
        state.history_files = Ok(Vec::new());
        state.viewer_files = Ok(Vec::new());
        assert!(state.file_list_has_detail());
        assert_eq!(state.file_list_len(), 1);
        assert_eq!(state.file_list_selected_row(), Some(0));
        state.history_files = Err("조회 실패".into());
        assert!(!state.file_list_has_detail());
        assert_eq!(state.file_list_len(), 0);
        assert_eq!(state.file_list_selected_row(), None);
        assert!(!state.select_file_list_row(project, state.panel_revision, true, 0));
        state.history_files = Ok(Vec::new());
        state.projects[project].commits.clear();
        assert!(!state.file_list_has_detail());
        assert_eq!(state.file_list_len(), 0);
        state.file_history = false;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(!state.file_list_has_detail());
        assert_eq!(state.file_list_len(), state.changed_files().len());
    }

    #[test]
    fn detail_list_row_mapping_preserves_same_named_file_and_rejects_stale_events() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_history_row(project, 0));
        drive(&mut state);
        let original = state.viewer_files.as_ref().unwrap()[0].clone();
        let mut named = original.clone();
        named.label = "커밋상세".into();
        let mut last = original;
        last.path = b"last.txt".to_vec();
        last.label = "last.txt".into();
        let named_path = named.path.clone();
        let last_path = last.path.clone();
        state.history_files = Ok(vec![
            ChangedFile {
                status: "M".into(),
                path: "커밋상세".into(),
            },
            ChangedFile {
                status: "M".into(),
                path: "last.txt".into(),
            },
        ]);
        state.viewer_files = Ok(vec![named, last]);
        let revision = state.panel_revision;
        assert_eq!(state.file_list_len(), 3);
        assert!(state.select_file_list_row(project, revision, true, 1));
        assert_eq!(state.file_selection.selected(2), Some(0));
        assert_eq!(state.file_list_selected_row(), Some(1));
        assert!(
            matches!(state.viewer_target(), ViewerTarget::File { file, historical: true, .. } if file.path == named_path)
        );
        assert!(state.select_file_list_row(project, revision, true, 2));
        assert_eq!(state.file_selection.selected(2), Some(1));
        assert_eq!(state.file_list_selected_row(), Some(2));
        assert!(
            matches!(state.viewer_target(), ViewerTarget::File { file, .. } if file.path == last_path)
        );
        for (owner, rev, history, row) in [
            (project, revision, true, 3),
            (project, revision.wrapping_sub(1), true, 0),
            (project + 1, revision, true, 0),
            (project, revision, false, 0),
        ] {
            assert!(!state.select_file_list_row(owner, rev, history, row));
            assert_eq!(state.file_list_selected_row(), Some(2));
        }
        assert!(state.select_file_list_row(project, revision, true, 0));
        assert_eq!(state.file_selection.selected(2), None);
        assert_eq!(state.file_list_selected_row(), Some(0));
        assert!(matches!(state.viewer_target(), ViewerTarget::Detail { .. }));
        state.file_history = false;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(state.select_file_list_row(project, state.panel_revision, false, 0));
        assert_eq!(
            state.file_selection.selected(state.changed_files().len()),
            Some(0)
        );
        assert_eq!(state.file_list_selected_row(), Some(0));
    }

    #[test]
    fn detail_list_defaults_to_selected_and_is_exclusive_with_history_files() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(!state.commit_detail_selected());
        state.file_history = true;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(state.commit_detail_selected());
        state.history_files = Ok(vec![ChangedFile {
            status: "M".into(),
            path: "file.rs".into(),
        }]);
        assert!(state.file_selection.select(0, 1));
        assert!(!state.commit_detail_selected());
        assert!(state.select_commit_detail(project));
        assert!(state.commit_detail_selected());
        assert_eq!(state.file_selection.selected(1), None);
        assert!(state.file_selection.select(0, 1));
        assert!(!state.select_history_row(project, state.commit));
        assert!(!state.commit_detail_selected());
        assert_eq!(state.file_selection.selected(1), Some(0));
        assert!(state.select_commit_detail(project));
        assert!(state.commit_detail_selected());
        state.file_history = false;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(!state.commit_detail_selected());
        assert!(!state.select_commit_detail(project));
    }

    #[test]
    fn detail_list_handles_empty_errors_and_stale_events_without_changing_commit() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let commit = state.commit;
        state.file_history = true;
        state.history_files = Ok(Vec::new());
        assert!(state.commit_detail_selected());
        assert!(state.select_commit_detail(project));
        state.history_files = Err("lookup failed".into());
        assert!(state.commit_detail_selected());
        assert!(!state.file_list_has_detail());
        assert!(!state.select_commit_detail(project));
        assert!(!state.select_commit_detail(project + 1));
        state.start_tab_active = true;
        assert!(!state.select_commit_detail(project));
        state.start_tab_active = false;
        state.open_projects.retain(|id| *id != project);
        assert!(!state.select_commit_detail(project));
        assert_eq!(state.commit, commit);
    }

    #[test]
    fn boundary_drag_uses_all_measured_widths_and_preserves_total_on_repeated_resize() {
        let (_repositories, mut state) = workspace_with_repositories();
        let start = [45., 700., 110., 140., 96.];
        assert!(state.begin_history_resize(1, 100., Some(start)));
        state.finish_history_resize(100.);
        assert_eq!(state.history_columns.widths(), start);
        assert!(state.begin_history_resize(1, 100., Some(start)));
        assert!(state.update_history_resize(120., true));
        assert_eq!(state.history_columns.widths(), [45., 720., 90., 140., 96.]);
        state.finish_history_resize(120.);
        state.select_project(0);
        drive(&mut state);
        let after = state.history_columns.widths();
        assert_eq!(after.iter().sum::<f32>(), start.iter().sum::<f32>());
        assert!(state.begin_history_resize(1, 100., Some(after)));
        state.finish_history_resize(80.);
        assert_eq!(state.history_columns.widths(), start);
    }

    #[test]
    fn graph_width_ignores_lane_count_and_can_expand_from_zero() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.histories.get_mut(&state.project).unwrap().lanes = 50;
        assert!(state.begin_history_resize(0, 100., None));
        assert_eq!(state.history_resize.unwrap().start_widths[0], 92.);
        state.finish_history_resize(0.);
        let collapsed = state.history_columns.widths();
        assert_eq!(collapsed[0], 0.);
        assert!(state.begin_history_resize(0, 0., Some(collapsed)));
        state.finish_history_resize(25.);
        assert_eq!(state.history_columns.widths()[0], 25.);
        assert_eq!(
            state.history_columns.widths().iter().sum::<f32>(),
            collapsed.iter().sum::<f32>()
        );
    }

    #[test]
    fn history_column_drag_preserves_total_and_selection_and_stops_on_release() {
        let (_repositories, mut state) = workspace_with_repositories();
        let selected = state.commit;
        let start = [45., 700., 110., 140., 96.];
        assert!(!state.begin_history_resize(4, 100., None));
        assert!(!state.begin_history_resize(1, f32::NAN, None));
        assert!(state.begin_history_resize(1, 100., Some(start)));
        assert!(state.update_history_resize(140., true));
        assert_eq!(state.history_columns.widths(), [45., 740., 70., 140., 96.]);
        state.update_history_resize(160., true);
        assert_eq!(state.history_columns.widths(), [45., 750., 60., 140., 96.]);
        assert_eq!(state.commit, selected);
        assert!(state.update_history_resize(170., false));
        assert!(state.history_resize.is_none());
        assert!(!state.update_history_resize(200., true));
        assert_eq!(
            state.history_columns.widths().iter().sum::<f32>(),
            start.iter().sum::<f32>()
        );
        assert!(!state.finish_history_resize(300.));
    }

    #[test]
    fn column_widths_survive_project_switches_and_active_drags_are_cancelled() {
        let (_repositories, mut state) = workspace_with_repositories();
        assert!(state.begin_history_resize(2, 100., None));
        state.update_history_resize(130., true);
        let widths = state.history_columns;
        state.select_project(0);
        drive(&mut state);
        assert!(state.history_resize.is_none());
        assert_eq!(state.history_columns, widths);
        assert!(!state.update_history_resize(300., true));
        assert!(state.begin_history_resize(3, 100., None));
        state.open_start_tab();
        drive(&mut state);
        assert!(state.history_resize.is_none());
        assert_eq!(state.history_columns, widths);
        assert!(!state.begin_history_resize(3, 100., None));
    }

    #[test]
    fn reference_icons_are_distinct() {
        let local = reference_icon(ReferenceKind::Local).unwrap();
        assert_eq!(local, ReferenceIcon::Computer);
        let remote = reference_icon(ReferenceKind::Remote).unwrap();
        let tag = reference_icon(ReferenceKind::Tag).unwrap();
        assert_ne!(local, remote);
        assert_ne!(local, tag);
        assert_ne!(remote, tag);
        assert_eq!(reference_icon(ReferenceKind::Other), None);
    }

    #[test]
    fn repository_selection_expands_old_tag_and_preserves_worktree_and_selected_row() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/repository-selection-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        std::fs::write(dir.path().join("tracked.txt"), "unchanged").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Tester", "test@example.invalid").unwrap();
        let first = repo
            .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        let mut latest = first;
        for _ in 0..repository::INITIAL_HISTORY_ROWS + 5 {
            let parent = repo.find_commit(latest).unwrap();
            latest = repo
                .commit(Some("HEAD"), &sig, &sig, "next", &tree, &[&parent])
                .unwrap();
        }
        let object = repo.find_object(first, None).unwrap();
        repo.tag("oldest", &object, &sig, "old tag", false).unwrap();
        let blob = repo.blob(b"not a commit").unwrap();
        let blob = repo.find_object(blob, None).unwrap();
        repo.tag_lightweight("blob", &blob, false).unwrap();
        let loaded = repository::execute(Request {
            operation: Operation::Open,
            source: String::new(),
            name: "fixture".into(),
            path: dir.path().to_string_lossy().into(),
        })
        .unwrap();
        assert_eq!(loaded.commits.len(), repository::INITIAL_HISTORY_ROWS);
        let (_repositories, mut state) = workspace_with_repositories();
        state.activate_repository(loaded);
        drive(&mut state);
        let project = state.project;
        let old_scroll = state.histories[&project].scroll.clone();
        state.histories[&project]
            .horizontal_scroll
            .set_offset(gpui::point(px(-75.), px(0.)));
        state.repository_tab = RepositoryTab::Tags;
        state.file_history = true;
        state.refresh_panel_data();
        drive(&mut state);
        let selected = state
            .repository_names()
            .iter()
            .position(|name| name == "oldest")
            .unwrap();
        assert!(state.select_repository_row(project, RepositoryTab::Tags, selected));
        drive(&mut state);
        assert_eq!(
            state.projects[project].commits[state.commit].id,
            first.to_string()
        );
        assert!(state.projects[project].commits.len() > repository::INITIAL_HISTORY_ROWS);
        assert!(!state.projects[project].more_commits);
        assert!(state.projects[project].history_cursor.is_none());
        assert_eq!(
            state
                .repository_selection
                .selected(state.repository_names().len()),
            Some(selected)
        );
        assert_eq!(state.history_files.as_ref().unwrap()[0].path, "tracked.txt");
        assert!(std::rc::Rc::ptr_eq(
            &old_scroll.0,
            &state.histories[&project].scroll.0
        ));
        assert_eq!(
            state.histories[&project].horizontal_scroll.offset().x,
            px(-75.)
        );
        let previous = state.commit;
        let invalid = state
            .repository_names()
            .iter()
            .position(|name| name == "blob")
            .unwrap();
        assert!(state.select_repository_row(project, RepositoryTab::Tags, invalid));
        drive(&mut state);
        assert_eq!(state.commit, previous);
        assert!(state.notice.contains("blob"));
        assert_eq!(repo.head().unwrap().target(), Some(latest));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("tracked.txt")).unwrap(),
            "unchanged"
        );
    }

    #[test]
    fn repository_row_selection_uses_exact_reference_and_scrolls_history() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        state.projects[project].branches = vec!["shared".into()];
        state.projects[project].remotes = vec!["origin/shared".into()];
        for (index, full_name, name) in [
            (0, "refs/heads/shared", "shared"),
            (1, "refs/remotes/origin/shared", "origin/shared"),
            (2, "refs/tags/shared", "shared"),
        ] {
            state.projects[project].commits[index].references = vec![repository::CommitReference {
                full_name: full_name.into(),
                name: name.into(),
                label: name.into(),
                is_remote: full_name.starts_with("refs/remotes/"),
            }];
        }
        state.file_history = true;
        for (tab, expected) in [
            (RepositoryTab::Local, 0),
            (RepositoryTab::Remote, 1),
            (RepositoryTab::Tags, 2),
        ] {
            state.select_repository_tab(tab);
            drive(&mut state);
            if tab == RepositoryTab::Tags {
                state.panel_references = Ok(vec!["shared".into()]);
            }
            assert!(state.select_repository_row(project, tab, 0));
            drive(&mut state);
            assert_eq!(state.commit, expected);
            assert!(state.history_files.is_ok());
            let scroll = state.histories[&project].scroll.0.borrow();
            assert_eq!(
                scroll.deferred_scroll_to_item.as_ref().unwrap().item_index,
                expected
            );
        }
        state.select_repository_tab(RepositoryTab::Local);
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        drive(&mut state);
        assert_eq!(state.repository_selection.selected(1), Some(0));
        state.commit = 1;
        let revision = state.panel_revision;
        assert!(!state.select_repository_row(project, RepositoryTab::Local, 0));
        assert_eq!(state.panel_revision, revision);
        assert_eq!(state.commit, 1);
    }

    #[test]
    fn repository_selection_ignores_stale_events_and_keeps_commit_for_stash_or_missing_target() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let original = state.commit;
        assert!(!state.select_repository_row(project + 1, RepositoryTab::Local, 0));
        drive(&mut state);
        assert!(!state.select_repository_row(project, RepositoryTab::Remote, 0));
        drive(&mut state);
        assert!(!state.select_repository_row(project, RepositoryTab::Local, usize::MAX));
        drive(&mut state);
        state.start_tab_active = true;
        assert!(!state.select_repository_row(project, RepositoryTab::Local, 0));
        drive(&mut state);
        state.start_tab_active = false;
        state.open_projects.retain(|id| *id != project);
        assert!(!state.select_repository_row(project, RepositoryTab::Local, 0));
        drive(&mut state);
        state.open_projects.push(project);
        state.projects[project].branches = vec!["missing".into()];
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        drive(&mut state);
        assert_eq!(state.commit, original);
        assert!(state.notice.contains("missing"));
        state.select_repository_tab(RepositoryTab::Stashes);
        drive(&mut state);
        state.panel_references = Ok(vec!["0: fixture".into()]);
        assert!(state.select_repository_row(project, RepositoryTab::Stashes, 0));
        drive(&mut state);
        assert_eq!(state.commit, original);
        assert_eq!(state.repository_selection.selected(1), Some(0));
    }

    #[test]
    fn panel_selections_are_independent_and_clear_on_data_refresh() {
        let (_repositories, mut state) = workspace_with_repositories();
        let repositories = state.repository_names().len();
        let files = state.changed_files().len();
        assert!(state.repository_selection.select(0, repositories));
        assert_eq!(state.file_selection.selected(files), None);
        assert!(state.file_selection.select(0, files));
        state.repository_selection.select(1, repositories);
        assert_eq!(state.file_selection.selected(files), Some(0));
        state.refresh_panel_data();
        drive(&mut state);
        assert_eq!(state.repository_selection.selected(repositories), None);
        assert_eq!(state.file_selection.selected(files), None);
    }

    #[test]
    fn history_selection_checks_context_and_refreshes_files() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let original = state.commit;
        assert!(!state.select_history_row(project + 1, 0));
        drive(&mut state);
        assert!(!state.select_history_row(project, usize::MAX));
        drive(&mut state);
        state.start_tab_active = true;
        assert!(!state.select_history_row(project, 0));
        drive(&mut state);
        state.start_tab_active = false;
        state.open_projects.retain(|id| *id != project);
        assert!(!state.select_history_row(project, 0));
        drive(&mut state);
        assert_eq!(state.commit, original);
        state.open_projects.push(project);
        state.file_history = true;
        assert!(state.file_selection.select(0, 1));
        assert!(!state.select_history_row(project, original));
        assert_eq!(state.file_selection.selected(1), Some(0));
        state.file_history = false;
        assert!(state.select_history_row(project, 0));
        drive(&mut state);
        assert_eq!(state.commit, 0);
        assert!(state.history_files.is_ok());
        assert_eq!(state.file_selection.selected(1), None);
    }

    #[test]
    fn file_tabs_and_commit_rows_preserve_completed_repository_lists() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        for tab in [
            RepositoryTab::Local,
            RepositoryTab::Remote,
            RepositoryTab::Tags,
            RepositoryTab::Stashes,
        ] {
            state.repository_tab = tab;
            state.file_history = false;
            state.refresh_panel_data();
            drive(&mut state);
            match tab {
                RepositoryTab::Local => {
                    state.projects[project].branches = vec!["local-name".into()]
                }
                RepositoryTab::Remote => {
                    state.projects[project].remotes = vec!["remote-name".into()]
                }
                _ => state.panel_references = Ok(vec!["stored-name".into()]),
            }
            assert!(
                state
                    .repository_selection
                    .select(0, state.repository_names().len())
            );
            let names = state.repository_names().to_vec();
            let revision = state.repository_revision;
            let completion = state.repository_loading.completion();
            let cancelled = state.repository_reads.observe_cancellation();
            let scroll = state
                .panel_scrolls
                .entry(("branches", project, tab as usize))
                .or_default()
                .clone();
            scroll.horizontal.set_offset(gpui::point(px(-36.), px(0.)));
            for history in [true, false] {
                assert!(state.select_file_tab(history));
                assert_eq!(state.repository_revision, revision);
                assert!(state.repository_pending.is_none());
                assert!(!state.repository_waiting);
                assert!(!cancelled());
                assert!(std::sync::Arc::ptr_eq(
                    &completion,
                    &state.repository_loading.completion()
                ));
                assert_eq!(state.repository_names(), names);
                assert_eq!(state.repository_selection.selected(names.len()), Some(0));
                assert_eq!(scroll.horizontal.offset().x, px(-36.));
                assert!(state.panel_waiting && state.panel_pending.is_some());
                drive(&mut state);
                assert_eq!(state.repository_names(), names);
            }
            assert!(!state.select_file_tab(false));
            let other = usize::from(state.commit == 0);
            for index in [other, 1 - other, other] {
                assert!(state.select_history_row(project, index));
                assert_eq!(state.repository_revision, revision);
                assert!(state.repository_pending.is_none());
                assert!(!cancelled());
                assert_eq!(state.repository_names(), names);
                assert_eq!(scroll.horizontal.offset().x, px(-36.));
                drive(&mut state);
                assert_eq!(state.repository_names(), names);
                assert!(state.commit_detail_selected());
            }
        }
    }

    #[test]
    fn file_navigation_preserves_inflight_repository_loading_and_applies_result() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        for tab in [RepositoryTab::Tags, RepositoryTab::Stashes] {
            assert!(state.select_repository_tab(tab));
            let request = state.repository_pending.take().unwrap();
            let revision = state.repository_revision;
            let start = std::time::Instant::now();
            state.repository_loading.begin(revision, start);
            assert!(
                state
                    .repository_loading
                    .reveal(revision, start + LOADING_DELAY)
            );
            let completion = state.repository_loading.completion();
            let cancelled = state.repository_reads.observe_cancellation();
            state.file_history = false;
            assert!(state.select_file_tab(true));
            let next = (state.commit + 1) % state.projects[project].commits.len();
            assert!(state.select_history_row(project, next));
            assert!(state.select_file_tab(false));
            assert_eq!(state.repository_revision, revision);
            assert!(state.repository_loading.is_visible(revision));
            assert!(state.repository_waiting);
            assert!(!cancelled());
            assert!(std::sync::Arc::ptr_eq(
                &completion,
                &state.repository_loading.completion()
            ));
            assert!(state.repository_pending.is_none());
            let mut result = std::thread::spawn(move || request.execute())
                .join()
                .unwrap();
            result.names = Ok(vec!["loaded-after-file-navigation".into()]);
            assert!(state.finish_repository_list(result));
            assert_eq!(state.repository_names(), &["loaded-after-file-navigation"]);
            assert!(!state.repository_loading.is_pending());
            assert!(state.panel_waiting);
            drive(&mut state);
        }
    }

    #[test]
    fn file_navigation_preserves_repository_errors_and_invalidates_only_old_reference_selection() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        drive(&mut state);
        state.panel_references = Err("기존 태그 오류".into());
        let revision = state.repository_revision;
        assert!(state.select_file_tab(true));
        assert!(state.select_history_row(project, 0));
        assert_eq!(
            state.panel_references.as_ref().unwrap_err(),
            "기존 태그 오류"
        );
        assert_eq!(state.repository_revision, revision);
        assert!(state.repository_pending.is_none());
        drive(&mut state);
        assert!(state.select_repository_tab(RepositoryTab::Local));
        state.projects[project].branches = vec!["missing-target".into()];
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        let request = state.reference_pending.as_ref().unwrap().clone();
        let reference_revision = state.reference_revision;
        let cancelled = state.reference_reads.observe_cancellation();
        assert!(state.select_file_tab(false));
        assert!(!cancelled());
        assert_eq!(state.reference_revision, reference_revision);
        assert!(state.reference_pending.is_some());
        assert!(state.select_history_row(project, 1));
        assert!(cancelled());
        assert!(state.reference_pending.is_none());
        assert!(!state.finish_reference(request, Err("늦은 선택 오류".into())));
        assert_eq!(state.commit, 1);
    }

    #[test]
    fn project_navigation_still_refreshes_repository_lists_after_start_tab() {
        let (_repositories, mut state) = workspace_with_repositories();
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        drive(&mut state);
        let revision = state.repository_revision;
        state.open_start_tab();
        assert!(!state.repository_loading.is_pending());
        state.select_project(0);
        assert!(!state.start_tab_active);
        assert!(state.repository_revision != revision);
        let request = state.repository_pending.as_ref().unwrap();
        assert_eq!(request.project, 0);
        assert_eq!(Some(&request.path), state.projects[0].path.as_ref());
        assert!(state.repository_loading.is_pending());
        assert!(state.panel_pending.is_some());
        drive(&mut state);
        assert!(!state.repository_waiting);
        assert!(!state.panel_waiting);
    }

    #[test]
    fn active_file_change_only_refreshes_files_and_keeps_repository_loading() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(!state.file_history);
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        let revision = state.repository_revision;
        let completion = state.repository_loading.completion();
        let request_revision = state.repository_pending.as_ref().unwrap().revision;
        let cancelled = state.repository_reads.observe_cancellation();
        let root = state.projects[project].path.clone().unwrap();
        state.viewer_changed(&ViewerChanged { root });
        assert_eq!(state.repository_revision, revision);
        assert_eq!(
            state.repository_pending.as_ref().unwrap().revision,
            request_revision
        );
        assert!(!cancelled());
        assert!(std::sync::Arc::ptr_eq(
            &completion,
            &state.repository_loading.completion()
        ));
        assert!(state.repository_waiting);
        assert!(state.panel_pending.is_some());
        drive(&mut state);
    }

    #[test]
    fn repository_loading_uses_common_delay_for_tags_and_stashes_only() {
        let (_repositories, mut state) = workspace_with_repositories();
        let panel_revision = state.panel_revision;
        let before = state.viewer_target();
        assert!(!state.repository_loading.is_pending());
        for tab in [RepositoryTab::Tags, RepositoryTab::Stashes] {
            assert!(state.select_repository_tab(tab));
            assert!(state.repository_loading.is_pending());
            let revision = state.repository_revision;
            let start = std::time::Instant::now();
            state.repository_loading.begin(revision, start);
            assert!(
                !state
                    .repository_loading
                    .reveal(revision, start + std::time::Duration::from_millis(149))
            );
            assert!(!state.repository_loading.is_visible(revision));
            assert!(
                state
                    .repository_loading
                    .reveal(revision, start + LOADING_DELAY)
            );
            assert!(state.repository_loading.is_visible(revision));
            assert!(!state.panel_loading.is_pending());
            assert_eq!(state.panel_revision, panel_revision);
            assert_eq!(state.viewer_target(), before);
            drive(&mut state);
            assert!(!state.repository_loading.is_pending());
            assert!(!state.repository_loading.is_visible(revision));
        }
        for tab in [RepositoryTab::Local, RepositoryTab::Remote] {
            assert!(state.select_repository_tab(tab));
            assert!(!state.repository_loading.is_pending());
            assert!(state.repository_pending.is_none());
        }
    }

    #[test]
    fn repository_loading_clears_for_fast_completion_empty_results_and_errors() {
        use std::sync::atomic::Ordering;
        let (_repositories, mut state) = workspace_with_repositories();
        for tab in [RepositoryTab::Tags, RepositoryTab::Stashes] {
            assert!(state.select_repository_tab(tab));
            for outcome in 0..3 {
                state.refresh_repository_list();
                assert!(state.repository_loading.is_pending());
                let revision = state.repository_revision;
                let start = std::time::Instant::now();
                state.repository_loading.begin(revision, start);
                if outcome == 0 {
                    state
                        .repository_loading
                        .completion()
                        .store(true, Ordering::Release);
                    assert!(
                        !state
                            .repository_loading
                            .reveal(revision, start + LOADING_DELAY)
                    );
                } else {
                    assert!(
                        state
                            .repository_loading
                            .reveal(revision, start + LOADING_DELAY)
                    );
                }
                let request = state.repository_pending.take().unwrap();
                let mut result = std::thread::spawn(move || request.execute())
                    .join()
                    .unwrap();
                result.names = if outcome == 2 {
                    Err("목록 조회 실패".into())
                } else {
                    Ok(Vec::new())
                };
                assert!(state.finish_repository_list(result));
                assert!(!state.repository_waiting);
                assert!(!state.repository_loading.is_pending());
                assert!(!state.repository_loading.is_visible(revision));
                assert_eq!(state.panel_references.is_err(), outcome == 2);
                assert!(state.panel_pending.is_none());
            }
        }
    }

    #[test]
    fn repository_loading_isolates_old_requests_and_resets_on_navigation() {
        use std::sync::atomic::Ordering;
        let (_repositories, mut state) = workspace_with_repositories();
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        let old_revision = state.repository_revision;
        let old_completion = state.repository_loading.completion();
        let old = state.repository_pending.take().unwrap();
        assert!(state.select_repository_tab(RepositoryTab::Stashes));
        assert!(state.repository_loading.is_pending());
        let revision = state.repository_revision;
        let start = std::time::Instant::now();
        state.repository_loading.begin(revision, start);
        old_completion.store(true, Ordering::Release);
        assert!(
            !state
                .repository_loading
                .completion()
                .load(Ordering::Acquire)
        );
        assert!(
            !state
                .repository_loading
                .reveal(old_revision, start + LOADING_DELAY)
        );
        assert!(
            state
                .repository_loading
                .reveal(revision, start + LOADING_DELAY)
        );
        let result = std::thread::spawn(move || old.execute()).join().unwrap();
        assert!(!state.finish_repository_list(result));
        assert!(state.repository_loading.is_visible(revision));
        assert!(state.select_repository_tab(RepositoryTab::Local));
        assert!(!state.repository_loading.is_pending());
        assert!(
            !state
                .repository_loading
                .reveal(revision, start + LOADING_DELAY)
        );
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        let old = state.repository_pending.take().unwrap();
        state.open_start_tab();
        assert!(!state.repository_loading.is_pending());
        let result = std::thread::spawn(move || old.execute()).join().unwrap();
        assert!(!state.finish_repository_list(result));
        assert!(!state.repository_loading.is_pending());
    }

    #[test]
    fn repository_tab_changes_preserve_file_data_selection_scroll_and_viewer() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        for historical in [false, true] {
            state.file_history = historical;
            state.refresh_panel_data();
            drive(&mut state);
            let count = state.changed_files().len();
            assert!(count > 0);
            assert!(state.file_selection.select(0, count));
            let before = state.viewer_target();
            let names: Vec<_> = state
                .changed_files()
                .iter()
                .map(|file| (file.status.clone(), file.path.clone()))
                .collect();
            let revision = state.panel_revision;
            let cancelled = state.panel_reads.observe_cancellation();
            let scroll = state
                .panel_scrolls
                .entry(("files", project, usize::from(historical)))
                .or_default()
                .clone();
            scroll.horizontal.set_offset(gpui::point(px(-24.), px(0.)));
            for tab in [
                RepositoryTab::Remote,
                RepositoryTab::Tags,
                RepositoryTab::Stashes,
                RepositoryTab::Local,
            ] {
                crate::backend::with_test_delay(50, std::thread::current().id(), || {
                    assert!(state.select_repository_tab(tab));
                    assert!(crate::backend::test_calls().is_empty());
                });
                assert_eq!(state.panel_revision, revision);
                assert!(state.panel_pending.is_none());
                assert!(!state.panel_waiting);
                assert!(!state.panel_loading.is_pending());
                assert!(!cancelled());
                assert_eq!(state.file_selection.selected(count), Some(0));
                assert_eq!(state.viewer_target(), before);
                assert_eq!(
                    state
                        .changed_files()
                        .iter()
                        .map(|file| (file.status.clone(), file.path.clone()))
                        .collect::<Vec<_>>(),
                    names
                );
                assert_eq!(scroll.horizontal.offset().x, px(-24.));
                drive(&mut state);
                assert_eq!(state.viewer_target(), before);
            }
            assert!(!state.select_repository_tab(RepositoryTab::Local));
            assert!(state.panel_pending.is_none());
        }
    }

    #[test]
    fn repository_tab_change_does_not_cancel_or_replace_inflight_file_request() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        assert!(state.select_history_row(project, 0));
        let request = state.panel_pending.take().unwrap();
        let revision = state.panel_revision;
        let completed = state.panel_loading.completion();
        let cancelled = state.panel_reads.observe_cancellation();
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        assert!(state.select_repository_tab(RepositoryTab::Stashes));
        assert!(state.panel_waiting && state.panel_loading.is_pending());
        assert!(std::sync::Arc::ptr_eq(
            &completed,
            &state.panel_loading.completion()
        ));
        assert_eq!(state.panel_revision, revision);
        assert!(state.panel_pending.is_none());
        assert!(!cancelled());
        let result = std::thread::spawn(move || request.execute())
            .join()
            .unwrap();
        assert!(state.finish_panel(result));
        assert!(!state.panel_waiting);
        assert!(state.file_list_has_detail());
        assert!(state.repository_waiting);
        drive(&mut state);
        assert!(!state.repository_waiting);
    }

    #[test]
    fn repository_list_results_ignore_old_tabs_contexts_and_preserve_file_state() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        let before = state.viewer_target();
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        let old = state.repository_pending.take().unwrap();
        assert!(state.select_repository_tab(RepositoryTab::Stashes));
        let current = state.repository_pending.take().unwrap();
        let mut old_result = std::thread::spawn(move || old.execute()).join().unwrap();
        old_result.names = Err("이전 태그 오류".into());
        assert!(!state.finish_repository_list(old_result));
        assert!(state.repository_waiting);
        let mut current_result = std::thread::spawn(move || current.execute())
            .join()
            .unwrap();
        current_result.names = Err("현재 스태시 오류".into());
        assert!(state.finish_repository_list(current_result));
        assert_eq!(
            state.panel_references.as_ref().unwrap_err(),
            "현재 스태시 오류"
        );
        assert!(!state.repository_waiting);
        assert_eq!(state.viewer_target(), before);
        assert!(state.select_repository_tab(RepositoryTab::Tags));
        let request = state.repository_pending.take().unwrap();
        let mut changed_path = request.clone();
        changed_path.path.push("different");
        let result = std::thread::spawn(move || changed_path.execute())
            .join()
            .unwrap();
        assert!(!state.finish_repository_list(result));
        let result = std::thread::spawn(move || request.execute())
            .join()
            .unwrap();
        state.close_project(project);
        assert!(!state.finish_repository_list(result));
        drive(&mut state);
    }

    #[test]
    fn repository_tab_change_cancels_previous_reference_selection_only() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = state.project;
        state.projects[project].branches = vec!["missing-reference".into()];
        assert!(state.select_repository_row(project, RepositoryTab::Local, 0));
        let request = state.reference_pending.take().unwrap();
        let cancelled = state.reference_reads.observe_cancellation();
        let panel_revision = state.panel_revision;
        let before = state.viewer_target();
        assert!(state.select_repository_tab(RepositoryTab::Remote));
        assert!(cancelled());
        assert_eq!(state.panel_revision, panel_revision);
        assert_eq!(state.repository_selection.selected(1), None);
        assert!(!state.finish_reference(request, Err("늦은 참조 선택 오류".into())));
        assert_eq!(state.viewer_target(), before);
    }

    #[test]
    fn panel_list_sources_follow_tabs_projects_and_errors() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.projects[state.project].branches = vec!["main".into(), "feature".into()];
        state.projects[state.project].remotes = vec!["origin/main".into()];
        assert_eq!(state.repository_names(), &["main", "feature"]);
        state.repository_tab = RepositoryTab::Remote;
        assert_eq!(state.repository_names(), &["origin/main"]);
        for tab in [RepositoryTab::Tags, RepositoryTab::Stashes] {
            state.repository_tab = tab;
            state.panel_references = Ok(vec!["reference".into()]);
            assert_eq!(state.repository_names(), &["reference"]);
            state.panel_references = Err("lookup failed".into());
            assert!(state.repository_names().is_empty());
        }
        state.projects[state.project].files = vec![ChangedFile {
            status: "A".into(),
            path: "working.rs".into(),
        }];
        assert_eq!(state.changed_files()[0].path, "working.rs");
        state.file_history = true;
        state.history_files = Ok(vec![ChangedFile {
            status: "M".into(),
            path: "commit.rs".into(),
        }]);
        assert_eq!(state.changed_files()[0].status, "M");
        assert_eq!(state.changed_files()[0].path, "commit.rs");
        state.history_files = Err("lookup failed".into());
        assert!(state.changed_files().is_empty());
        state.repository_tab = RepositoryTab::Local;
        state.file_history = false;
        state.projects[0].branches = vec!["other-project".into()];
        state.projects[0].files.clear();
        state.select_project(0);
        drive(&mut state);
        assert_eq!(state.repository_names(), &["other-project"]);
        assert!(state.changed_files().is_empty());
    }

    #[test]
    fn panel_selection_refreshes_on_project_and_commit_changes() {
        let (_repositories, mut state) = workspace_with_repositories();
        assert!(state.repository_tab == RepositoryTab::Local);
        assert!(!state.file_history);
        state.repository_tab = RepositoryTab::Remote;
        state.file_history = true;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(state.history_files.is_ok());
        state.select_project(0);
        drive(&mut state);
        assert!(state.repository_tab == RepositoryTab::Remote);
        assert!(state.file_history && state.history_files.is_ok());
        state.file_history = false;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(state.history_files.unwrap().is_empty());
    }

    #[test]
    fn history_cache_survives_switches_and_resets_on_snapshot_replacement_or_close() {
        let (_repositories, mut state) = workspace_with_repositories();
        let project = Project {
            name: "cached".into(),
            path: Some(std::path::PathBuf::from("D:/cached")),
            commits: vec![Commit {
                id: "one".into(),
                message: "old".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        state.activate_repository(project.clone());
        drive(&mut state);
        let index = state.project;
        let scroll = state.histories[&index].scroll.clone();
        state.histories[&index]
            .horizontal_scroll
            .set_offset(gpui::point(px(-100.), px(0.)));
        state.select_project(0);
        drive(&mut state);
        state.select_project(index);
        drive(&mut state);
        assert_eq!(
            state.histories[&index].horizontal_scroll.offset().x,
            px(-100.)
        );
        assert!(std::rc::Rc::ptr_eq(
            &scroll.0,
            &state.histories[&index].scroll.0
        ));
        let rows = state
            .histories
            .get_mut(&index)
            .unwrap()
            .prepare_rows(&state.projects[index], 0..1);
        assert_eq!(rows[0].title.as_ref(), "old");
        let mut updated = project;
        updated.commits[0].message = "new".into();
        state.activate_repository(updated);
        drive(&mut state);
        assert_eq!(state.histories[&index].horizontal_scroll.offset().x, px(0.));
        assert!(!std::rc::Rc::ptr_eq(
            &scroll.0,
            &state.histories[&index].scroll.0
        ));
        let rows = state
            .histories
            .get_mut(&index)
            .unwrap()
            .prepare_rows(&state.projects[index], 0..1);
        assert_eq!(rows[0].title.as_ref(), "new");
        state.close_project(index);
        drive(&mut state);
        assert!(!state.histories.contains_key(&index));
    }

    #[test]
    #[ignore = "manual performance measurement"]
    fn benchmark_history_project_switches() {
        let commits: Vec<_> = (0..200)
            .map(|i| Commit {
                id: format!("{i:040x}"),
                parents: if i < 199 {
                    vec![format!("{:040x}", i + 1)]
                } else {
                    vec![]
                },
                message: format!("commit {i}: rendering preparation"),
                author: "Tester".into(),
                date: "2026-11-12".into(),
                ..Default::default()
            })
            .collect();
        let mut state = Workspace {
            projects: vec![
                Project {
                    name: "a".into(),
                    commits: commits.clone(),
                    ..Default::default()
                },
                Project {
                    name: "b".into(),
                    commits,
                    ..Default::default()
                },
            ],
            open_projects: vec![0, 1],
            project: 0,
            ..Default::default()
        };
        state.select_project(0);
        state.select_project(1);
        let start = std::time::Instant::now();
        for i in 0..1000 {
            state.select_project(i % 2);
            std::hint::black_box(state.project);
        }
        eprintln!(
            "benchmark_history_project_switches: 200 commits, 1000 warm switches = {} us",
            start.elapsed().as_micros()
        );
    }

    #[test]
    fn created_repository_uses_real_empty_data_and_switches_between_real_repositories() {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/repository-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("actual");
        let project = repository::execute(Request {
            operation: Operation::Create,
            source: String::new(),
            path: path.join(".git").to_string_lossy().into(),
            name: "actual project".into(),
        })
        .unwrap();
        let (_repositories, mut state) = workspace_with_repositories();
        state.open_start_tab();
        drive(&mut state);
        state.activate_repository(project);
        drive(&mut state);
        let real = state.project;
        assert!(state.projects[real].commits.is_empty());
        assert!(state.projects[real].files.is_empty());
        assert!(state.histories[&state.project].is_empty());
        assert!(!state.start_tab_open && state.projects[real].path.is_some());
        state.select_project(0);
        drive(&mut state);
        assert_eq!(state.histories[&state.project].len(), 3);
        state.select_project(real);
        drive(&mut state);
        assert!(state.histories[&state.project].is_empty());
        assert_eq!(state.commit, 0);
        state.close_project(real);
        drive(&mut state);
        assert_eq!(state.histories[&state.project].len(), 3);
    }

    #[test]
    fn repository_success_replaces_start_tab_and_reuses_existing_project() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.open_start_tab();
        drive(&mut state);
        let project = Project {
            name: "real".into(),
            path: Some(std::path::PathBuf::from("D:/real")),
            ..Default::default()
        };
        state.activate_repository(project.clone());
        drive(&mut state);
        assert!(!state.start_tab_open && !state.start_tab_active);
        assert_eq!(state.projects[state.project].name, "real");
        assert!(state.projects[state.project].commits.is_empty());
        assert!(state.histories[&state.project].is_empty());
        let count = state.open_projects.len();
        state.open_start_tab();
        drive(&mut state);
        state.activate_repository(project.clone());
        drive(&mut state);
        assert_eq!(state.open_projects.len(), count);
        state.close_project(state.project);
        drive(&mut state);
        state.open_start_tab();
        drive(&mut state);
        state.activate_repository(project);
        drive(&mut state);
        assert_eq!(state.open_projects.len(), count);
    }

    #[test]
    fn start_tab_is_single_and_reactivates_from_a_project() {
        let (_repositories, mut state) = workspace_with_repositories();
        assert!(!state.start_tab_open && !state.start_tab_active);
        for _ in 0..5 {
            state.open_start_tab();
            drive(&mut state);
        }
        assert!(state.start_tab_open && state.start_tab_active);
        assert_eq!(state.open_projects.len(), 4);
        state.select_project(1);
        drive(&mut state);
        assert!(state.start_tab_open && !state.start_tab_active);
        state.open_start_tab();
        drive(&mut state);
        assert!(state.start_tab_open && state.start_tab_active);
        assert_eq!(state.project, 1);
    }

    #[test]
    fn closing_start_tab_restores_project_and_can_reopen() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.select_project(1);
        drive(&mut state);
        state.open_start_tab();
        drive(&mut state);
        state.close_start_tab();
        drive(&mut state);
        assert!(!state.start_tab_open && !state.start_tab_active);
        assert_eq!(state.project, 1);
        state.open_start_tab();
        drive(&mut state);
        assert!(state.start_tab_open && state.start_tab_active);
        state.select_project(2);
        drive(&mut state);
        state.close_start_tab();
        drive(&mut state);
        assert_eq!(state.project, 2);
        assert!(!state.start_tab_active);
    }

    #[test]
    fn last_project_closes_into_start_tab_and_empty_workspace_is_safe() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.open_start_tab();
        drive(&mut state);
        state.select_project(3);
        drive(&mut state);
        for project in 0..4 {
            state.close_project(project);
            drive(&mut state);
        }
        assert!(state.open_projects.is_empty());
        assert!(state.start_tab_open && state.start_tab_active);
        state.close_start_tab();
        drive(&mut state);
        assert!(!state.start_tab_open && !state.start_tab_active);
        state.select_project(99);
        drive(&mut state);
        state.close_project(99);
        drive(&mut state);
        state.open_start_tab();
        drive(&mut state);
        assert!(state.start_tab_open && state.start_tab_active);
    }

    #[test]
    fn closing_projects_in_background_preserves_start_tab_selection() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.open_start_tab();
        drive(&mut state);
        state.close_project(3);
        drive(&mut state);
        assert!(state.start_tab_active);
        assert_eq!(state.project, 2);
        state.close_start_tab();
        drive(&mut state);
        assert_eq!(state.project, 2);
    }

    #[test]
    fn new_workspace_always_opens_start_tab_without_restoring_projects() {
        let (_repositories, previous) = workspace_with_repositories();
        drop(previous);
        let next = Workspace::default();
        assert!(next.start_tab_open && next.start_tab_active);
        assert!(next.projects.is_empty());
        assert!(next.open_projects.is_empty());
    }

    #[test]
    fn closing_inactive_tab_preserves_selection_and_order() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.close_project(1);
        drive(&mut state);
        assert_eq!(state.open_projects, vec![0, 2, 3]);
        assert_eq!(state.project, 3);
    }
    #[test]
    fn closing_active_tab_selects_right_then_left_neighbor() {
        let (_repositories, mut state) = workspace_with_repositories();
        state.project = 1;
        state.close_project(1);
        drive(&mut state);
        assert_eq!(state.project, 2);
        state.project = 3;
        state.close_project(3);
        drive(&mut state);
        assert_eq!(state.project, 2);
    }
    #[test]
    fn closing_all_tabs_is_safe_and_unknown_close_is_ignored() {
        let (_repositories, mut state) = workspace_with_repositories();
        for project in 0..4 {
            state.close_project(project);
            drive(&mut state);
        }
        assert!(state.open_projects.is_empty());
        state.close_project(99);
        drive(&mut state);
        assert!(state.open_projects.is_empty());
    }
}

#[cfg(test)]
mod viewer_tests {
    use super::tab_tests::drive;
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let root = std::env::current_dir()
            .unwrap()
            .join("target/diff-viewer-ui-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let git = |args: &[&str]| {
            let result = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args([
                    "-c",
                    "core.autocrlf=false",
                    "-c",
                    "commit.gpgsign=false",
                    "-c",
                    "user.name=Viewer Test",
                    "-c",
                    "user.email=viewer@example.invalid",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        };
        git(&["init", "-b", "main"]);
        git(&["config", "core.autocrlf", "false"]);
        std::fs::write(dir.path().join("file.txt"), "one\ntwo\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "title\n\nfull body"]);
        std::fs::write(dir.path().join("file.txt"), "staged\ntwo\n").unwrap();
        git(&["add", "."]);
        std::fs::write(dir.path().join("file.txt"), "staged\nworktree\n").unwrap();
        dir
    }
    #[test]
    fn global_f5_refresh_reads_local_state_from_history_and_restores_selected_file() {
        let dir = fixture(); let mut state = Workspace::default();
        state.activate_repository(repository::execute(Request { operation: Operation::Open, source: String::new(), path: dir.path().display().to_string(), name: "Refresh".into() }).unwrap());
        drive(&mut state);
        state.file_selection.select(0, state.changed_files().len());
        let file = state.viewer_files.as_ref().unwrap()[0].clone();
        std::fs::write(dir.path().join("external.txt"), "external\n").unwrap();
        let repo = git2::Repository::open(dir.path()).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = git2::Signature::now("external", "external@example.invalid").unwrap();
        let next = repo.commit(Some("HEAD"), &sig, &sig, "external commit", &parent.tree().unwrap(), &[&parent]).unwrap();
        repo.branch("external-branch", &repo.find_commit(next).unwrap(), false).unwrap();
        let request = workspace_backend::ToolbarRequest { revision: state.sync_revision, project: state.project,
            path: state.footer_directory().unwrap(), name: "Refresh".into(), action: state.local_refresh_action().unwrap() };
        assert!(state.finish_toolbar_action(request.execute()).unwrap().1.is_none()); drive(&mut state);
        assert!(state.viewer_refresh_pending);
        assert_eq!(state.projects[state.project].head.as_deref(), Some(next.to_string().as_str()));
        assert!(state.projects[state.project].commits.iter().any(|commit| commit.message == "external commit"));
        assert!(state.projects[state.project].branches.iter().any(|branch| branch == "external-branch"));
        assert!(state.viewer_files.as_ref().unwrap().iter().any(|file| file.path == b"external.txt"));
        let selected = state.file_selection.selected(state.changed_files().len()).unwrap();
        assert_eq!(state.viewer_files.as_ref().unwrap()[selected].path, file.path);
        state.file_history = true; assert!(state.local_refresh_action().is_some());
        state.sync_busy = true; assert!(state.local_refresh_action().is_none()); state.sync_busy = false;
        state.alert = Some(crate::alert::Alert::new("blocked", "blocked")); assert!(state.local_refresh_action().is_none()); state.alert = None;
        state.start_tab_active = true; assert!(state.local_refresh_action().is_none());
        for key in ["ctrl-f5", "shift-f5", "alt-f5", "cmd-f5", "f4"] { assert!(!local_refresh_shortcut(&gpui::Keystroke::parse(key).unwrap())); }
        assert!(local_refresh_shortcut(&gpui::Keystroke::parse("f5").unwrap()));
    }

    #[test]
    fn files_and_detail_list_item_route_to_correct_comparison_source() {
        let dir = fixture();
        let mut state = Workspace::default();
        let empty = tempfile::tempdir_in(dir.path().parent().unwrap()).unwrap();
        git2::Repository::init(empty.path()).unwrap();
        state.activate_repository(
            repository::execute(Request {
                operation: Operation::Open,
                source: String::new(),
                path: empty.path().to_string_lossy().into(),
                name: "empty".into(),
            })
            .unwrap(),
        );
        drive(&mut state);
        state.activate_repository(
            repository::execute(Request {
                operation: Operation::Open,
                source: String::new(),
                path: dir.path().display().to_string(),
                name: "Viewer".into(),
            })
            .unwrap(),
        );
        drive(&mut state);
        let project = state.project;
        assert_eq!(state.changed_files().len(), 2);
        assert!(state.changed_files()[0].status.starts_with("S·"));
        state.file_selection.select(0, 2);
        assert!(
            matches!(state.viewer_target(),ViewerTarget::File{file,historical:false,..}if file.staged)
        );
        state.file_selection.select(1, 2);
        assert!(
            matches!(state.viewer_target(),ViewerTarget::File{file,historical:false,..}if !file.staged)
        );
        state.file_history = true;
        state.refresh_panel_data();
        drive(&mut state);
        assert!(state.commit_detail_selected());
        let id = state.projects[project].commits[0].id.clone();
        assert!(matches!(state.viewer_target(),ViewerTarget::Detail{id:actual,..}if actual==id));
        state.file_selection.select(0, state.changed_files().len());
        assert!(matches!(
            state.viewer_target(),
            ViewerTarget::File {
                historical: true,
                ..
            }
        ));
        assert!(state.select_commit_detail(project));
        assert!(matches!(state.viewer_target(), ViewerTarget::Detail { .. }));
        state.select_project(0);
        drive(&mut state);
        assert!(matches!(state.viewer_target(), ViewerTarget::Text { .. }));
    }
    #[test]
    fn write_refresh_preserves_file_identity_and_background_results_preserve_current_selection() {
        let dir = fixture();
        let mut state = Workspace::default();
        let empty = tempfile::tempdir_in(dir.path().parent().unwrap()).unwrap();
        git2::Repository::init(empty.path()).unwrap();
        state.activate_repository(
            repository::execute(Request {
                operation: Operation::Open,
                source: String::new(),
                path: empty.path().to_string_lossy().into(),
                name: "empty".into(),
            })
            .unwrap(),
        );
        drive(&mut state);
        state.activate_repository(
            repository::execute(Request {
                operation: Operation::Open,
                source: String::new(),
                path: dir.path().display().to_string(),
                name: "Viewer".into(),
            })
            .unwrap(),
        );
        drive(&mut state);
        let owner = state.project;
        let root = state.projects[owner].path.clone().unwrap();
        state.file_selection.select(1, 2);
        let file = state.viewer_files.as_ref().unwrap()[1].clone();
        let data = diff_git::diff(&root, &file).unwrap();
        let row = data.lines.iter().position(|l| l.kind == '+').unwrap();
        crate::partial_stage::apply(
            &root,
            &file,
            data.partial.as_ref().unwrap(),
            &data.lines,
            &[row],
        )
        .unwrap();
        let repository_revision = state.repository_revision;
        let reference_revision = state.reference_revision;
        let commits: Vec<_> = state.projects[owner]
            .commits
            .iter()
            .map(|commit| commit.id.clone())
            .collect();
        let cached_history = state.histories.remove(&owner).unwrap();
        let scroll_offset = cached_history.scroll.0.borrow().base_handle.offset();
        state.viewer_changed(&ViewerChanged { root: root.clone() });
        let request = state.panel_pending.as_ref().unwrap();
        assert!(!request.historical);
        assert!(request.graph.is_none());
        assert_eq!(state.repository_revision, repository_revision);
        assert_eq!(state.reference_revision, reference_revision);
        assert!(state.repository_pending.is_none());
        state.histories.insert(owner, cached_history);
        let result = state.panel_pending.take().unwrap().execute();
        assert!(result.history.is_none());
        assert!(state.finish_panel(result));
        assert_eq!(
            state.histories[&owner]
                .scroll
                .0
                .borrow()
                .base_handle
                .offset(),
            scroll_offset
        );
        assert_eq!(
            state.projects[owner]
                .commits
                .iter()
                .map(|commit| commit.id.clone())
                .collect::<Vec<_>>(),
            commits
        );
        let index = state
            .file_selection
            .selected(state.changed_files().len())
            .unwrap();
        let selected = &state.viewer_files.as_ref().unwrap()[index];
        assert_eq!(selected.path, file.path);
        assert!(!selected.staged);
        assert_ne!(selected.before, file.before);
        state.select_project(0);
        drive(&mut state);
        state.file_selection.select(0, state.changed_files().len());
        let before = state.viewer_target();
        state.viewer_changed(&ViewerChanged { root });
        drive(&mut state);
        assert_eq!(state.viewer_target(), before);
    }
}

#[cfg(test)]
mod status_label_tests {
    #[test]
    fn default_notice_omits_project_name_and_keeps_repository_path() {
        let mut state = super::Workspace {
            restoring: false,
            start_tab_active: false,
            project: 0,
            open_projects: vec![0],
            projects: vec![crate::repository::Project::default()],
            ..Default::default()
        };
        state.projects[0].name = "프로젝트명접두사".into();
        state.projects[0].path = Some(std::path::PathBuf::from("D:/repo"));
        state.update_tab_notice();
        assert_eq!(
            state.notice,
            crate::path_display::display_path(std::path::Path::new("D:/repo"))
        );
        state.projects[0].path = None;
        state.update_tab_notice();
        assert_eq!(state.notice, "저장소 경로가 없습니다");
    }
}

fn status_button(
    id: &'static str,
    text: impl Into<gpui::SharedString>,
    enabled: bool,
) -> gpui::Stateful<Div> {
    badge_base(TEXT)
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .when(enabled, |el| {
            el.cursor_pointer().hover(|style| style.bg(rgb(SELECTED)))
        })
        .when(!enabled, |el| el.opacity(0.4))
        .child(text.into())
}

#[cfg(test)]
mod footer_action_tests {
    use super::*;
    #[test]
    fn removed_pull_rebase_selection_restores_to_merge_and_cannot_be_chosen() {
        let root = std::env::current_dir().unwrap().join("target/pull-option-removal-tests");
        std::fs::create_dir_all(&root).unwrap();
        let dir = tempfile::tempdir_in(root).unwrap();
        let path = dir.path().join("workspace.json");
        let saved = session::Session { pull_mode: "rebase".into(), ..Default::default() };
        session::write(&path, &saved).unwrap();
        let mut state = Workspace::load_from_path(path);
        assert_eq!(state.selected_sync_mode, repository::SyncMode::Merge);
        assert!(!state.choose_sync_mode(repository::SyncMode::Rebase));
        assert_eq!(state.selected_sync_mode, repository::SyncMode::Merge);
        assert!(!state.sync_busy);
        assert_eq!(repository::SyncMode::OPTIONS, [repository::SyncMode::Fetch, repository::SyncMode::Merge, repository::SyncMode::FastForwardOnly]);
    }
    #[test]
    fn selected_sync_mode_is_saved_restored_and_does_not_execute() {
        let mut state = Workspace::default();
        assert_eq!(state.selected_sync_mode, repository::SyncMode::Merge);
        assert!(state.choose_sync_mode(repository::SyncMode::FastForwardOnly));
        assert_eq!(state.selected_sync_mode, repository::SyncMode::FastForwardOnly);
        assert!(!state.sync_busy);
        assert_eq!(state.sync_revision, 0);
        let saved = state.session_snapshot();
        assert_eq!(saved.pull_mode, "ff-only");
        let root = std::env::current_dir()
            .unwrap()
            .join("target/split-button-tests");
        std::fs::create_dir_all(&root).unwrap();
        let temp = tempfile::tempdir_in(root).unwrap();
        let path = temp.path().join("workspace.json");
        session::write(&path, &saved).unwrap();
        let mut restored = Workspace::load_from_path(path);
        assert_eq!(restored.selected_sync_mode, repository::SyncMode::FastForwardOnly);
        restored.sync_busy = true;
        assert!(!restored.choose_sync_mode(repository::SyncMode::Fetch));
        assert_eq!(restored.selected_sync_mode, repository::SyncMode::FastForwardOnly);
        assert_eq!(restored.sync_revision, 0);
    }
    #[test]
    fn sync_keeps_the_actual_head_selected_when_remote_commits_precede_it() {
        let path = std::path::PathBuf::from("D:/sync-test");
        let old = Commit {
            id: "old-head".into(),
            ..Default::default()
        };
        let mut state = Workspace {
            notice: "기존 하단 문구".into(),
            projects: vec![Project {
                path: Some(path.clone()),
                head: Some(old.id.clone()),
                commits: vec![old],
                ..Default::default()
            }],
            project: 0,
            open_projects: vec![0],
            start_tab_active: false,
            sync_busy: true,
            sync_revision: 1,
            ..Default::default()
        };
        assert!(state.global_progress_visible());
        let request = workspace_backend::SyncRequest {
            revision: 1,
            project: 0,
            path: path.clone(),
            name: "test".into(),
            mode: repository::SyncMode::Fetch,
        };
        let project = Project {
            path: Some(path),
            head: Some("old-head".into()),
            commits: vec![
                Commit {
                    id: "new-remote".into(),
                    ..Default::default()
                },
                Commit {
                    id: "old-head".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let history = PreparedHistory::new(&project.commits);
        assert!(
            state
                .finish_sync(workspace_backend::SyncResult {
                    conflict_paths: Vec::new(),
                    request,
                    outcome: Ok(()),
                    refreshed: Ok((project, history))
                })
                .is_none()
        );
        assert_eq!(state.commit, 1);
        assert_eq!(state.projects[0].head.as_deref(), Some("old-head"));
        assert!(!state.global_progress_visible());
        assert_eq!(state.notice, "기존 하단 문구");
    }
    #[test]
    fn sync_blocks_writes_and_applies_refreshed_state_even_when_operation_failed() {
        let path = std::path::PathBuf::from("D:/sync-test");
        let mut state = Workspace {
            notice: "기존 하단 문구".into(),
            projects: vec![Project {
                path: Some(path.clone()),
                ..Default::default()
            }],
            project: 0,
            open_projects: vec![0],
            start_tab_active: false,
            sync_busy: true,
            sync_revision: 4,
            ..Default::default()
        };
        assert!(state.repository_busy());
        assert!(state.directory_action_context().is_none());
        assert!(!state.queue_history_page(0, 1500));
        assert!(state.global_progress_visible());
        let request = workspace_backend::SyncRequest {
            revision: 4,
            project: 0,
            path: path.clone(),
            name: "test".into(),
            mode: repository::SyncMode::Merge,
        };
        let project = Project {
            path: Some(path),
            branch: "updated".into(),
            remotes: vec!["origin/main".into()],
            ..Default::default()
        };
        let error = state.finish_sync(workspace_backend::SyncResult {
                    conflict_paths: Vec::new(),
            request,
            outcome: Err("merge conflict".into()),
            refreshed: Ok((project, PreparedHistory::empty())),
        });
        assert_eq!(error.as_deref(), Some("merge conflict"));
        assert!(!state.sync_busy);
        assert_eq!(state.projects[0].branch, "updated");
        assert_eq!(state.projects[0].remotes, ["origin/main"]);
        assert!(state.histories.contains_key(&0));
        assert!(state.panel_pending.is_some());
        assert!(!state.global_progress_visible());
        assert_eq!(state.notice, "기존 하단 문구");
    }
    #[test]
    fn stale_sync_result_does_not_clear_newer_request_or_replace_the_project() {
        let path = std::path::PathBuf::from("D:/sync-test");
        let mut state = Workspace {
            notice: "기존 하단 문구".into(),
            projects: vec![Project {
                path: Some(path.clone()),
                branch: "current".into(),
                ..Default::default()
            }],
            project: 0,
            open_projects: vec![0],
            start_tab_active: false,
            sync_busy: true,
            sync_revision: 5,
            ..Default::default()
        };
        assert!(state.global_progress_visible());
        let request = workspace_backend::SyncRequest {
            revision: 4,
            project: 0,
            path,
            name: "test".into(),
            mode: repository::SyncMode::Fetch,
        };
        assert!(
            state
                .finish_sync(workspace_backend::SyncResult {
                    conflict_paths: Vec::new(),
                    request,
                    outcome: Ok(()),
                    refreshed: Ok((Project::default(), PreparedHistory::empty()))
                })
                .is_none()
        );
        assert!(state.sync_busy);
        assert_eq!(state.projects[0].branch, "current");
        assert!(state.global_progress_visible());
        assert_eq!(state.notice, "기존 하단 문구");
    }
    #[test]
    fn footer_directory_targets_current_project_and_blocks_empty_busy_and_stale_actions() {
        let mut state = Workspace::default();
        assert!(state.directory_action_context().is_none());
        state.projects = vec![
            Project {
                path: Some(std::path::PathBuf::from("D:/한글 repo & (one)")),
                ..Default::default()
            },
            Project {
                path: Some(std::path::PathBuf::from("D:/two")),
                ..Default::default()
            },
        ];
        state.open_projects = vec![0, 1];
        state.start_tab_active = false;
        state.project = 0;
        let (owner, root) = state.directory_action_context().unwrap();
        assert_eq!(owner, 0);
        assert_eq!(root, std::path::PathBuf::from("D:/한글 repo & (one)"));
        assert!(state.directory_request_current(owner, &root));
        state.footer_action_busy = true;
        assert!(state.directory_action_context().is_none());
        state.footer_action_busy = false;
        state.file_stage_busy = true;
        assert!(state.directory_action_context().is_none());
        state.file_stage_busy = false;
        state.alert = Some(Alert::new("test", "test"));
        assert!(state.directory_action_context().is_none());
        state.alert = None;
        state.project = 1;
        assert!(!state.directory_request_current(owner, &root));
        state.project = 0;
        state.start_tab_active = true;
        assert!(!state.directory_request_current(owner, &root));
        state.start_tab_active = false;
        state.open_projects.remove(0);
        assert!(!state.directory_request_current(owner, &root));
        state.open_projects.push(0);
        state.restoring = true;
        assert!(state.directory_action_context().is_none());
        state.restoring = false;
        state.projects[0].path = None;
        assert!(state.directory_action_context().is_none());
    }
}

fn reference_badge_icon_color(kind: ReferenceKind, text_color: u32) -> u32 {
    match kind {
        ReferenceKind::Local | ReferenceKind::Tag => 0xffffff,
        _ => text_color,
    }
}

#[cfg(test)]
mod badge_color_tests {
    use super::*;
    #[test]
    fn only_local_and_tag_badge_icons_are_white() {
        for color in [ACCENT, 0xb9a2ef, 0xffa7b2] {
            assert_eq!(
                reference_badge_icon_color(ReferenceKind::Local, color),
                0xffffff
            );
            assert_eq!(
                reference_badge_icon_color(ReferenceKind::Tag, color),
                0xffffff
            );
            assert_eq!(
                reference_badge_icon_color(ReferenceKind::Remote, color),
                color
            );
            assert_eq!(
                reference_badge_icon_color(ReferenceKind::Other, color),
                color
            );
        }
    }
}

#[cfg(test)]
mod commit_action_tests {
    use super::*;
    fn state() -> Workspace {
        Workspace {
            projects: vec![Project {
                path: Some("D:/commit-fixture".into()),
                head: Some("before".into()),
                commits: vec![Commit {
                    id: "before".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            project: 0,
            open_projects: vec![0],
            start_tab_active: false,
            commit_busy: true,
            commit_revision: 3,
            commit_owner: Some((0, "D:/commit-fixture".into())),
            notice: "existing notice".into(),
            ..Default::default()
        }
    }
    fn request(revision: u64) -> workspace_backend::CommitWorkRequest {
        workspace_backend::CommitWorkRequest {
            revision,
            project: 0,
            path: "D:/commit-fixture".into(),
            name: "fixture".into(),
            commit: repository::CommitRequest {
                expected_head: Some("before".into()),
                expected_reference: "refs/heads/main".into(),
                message: "subject".into(),
                push: true,
                amend: false,
            },
        }
    }
    #[test]
    fn partial_push_success_closes_the_commit_path_and_refreshes_the_created_head() {
        let mut state = state();
        assert!(state.repository_busy());
        assert!(state.global_progress_visible());
        assert!(state.directory_action_context().is_none());
        let project = Project {
            path: Some("D:/commit-fixture".into()),
            head: Some("created".into()),
            commits: vec![Commit {
                id: "created".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let history = PreparedHistory::new(&project.commits);
        let result = state
            .finish_commit_work(workspace_backend::CommitWorkResult {
                request: request(3),
                outcome: Ok(repository::CommitOutcome {
                    id: "created".into(),
                    error: Some("push rejected".into()),
                }),
                refreshed: Ok((project, history)),
            })
            .unwrap();
        assert_eq!(result.commit_id.as_deref(), Some("created"));
        assert_eq!(result.error.as_deref(), Some("push rejected"));
        assert_eq!(state.projects[0].head.as_deref(), Some("created"));
        assert!(!state.commit_busy);
        assert!(!state.global_progress_visible());
        assert!(state.panel_pending.is_some());
        assert_eq!(state.notice, "existing notice");
    }
    #[test]
    fn commit_failure_keeps_the_dialog_owner_for_retry_and_reports_refresh_failure() {
        let mut state = state();
        let owner = state.commit_owner.clone();
        let result = state
            .finish_commit_work(workspace_backend::CommitWorkResult {
                request: request(3),
                outcome: Err("index lock".into()),
                refreshed: Err("refresh failed".into()),
            })
            .unwrap();
        assert!(result.commit_id.is_none());
        let error = result.error.unwrap();
        assert!(error.contains("index lock") && error.contains("refresh failed"));
        assert_eq!(state.commit_owner, owner);
        assert_eq!(state.projects[0].head.as_deref(), Some("before"));
        assert!(!state.global_progress_visible());
    }
    #[test]
    fn stale_commit_result_cannot_clear_a_newer_operation() {
        let mut state = state();
        assert!(
            state
                .finish_commit_work(workspace_backend::CommitWorkResult {
                    request: request(2),
                    outcome: Ok(repository::CommitOutcome {
                        id: "stale".into(),
                        error: None
                    }),
                    refreshed: Err("ignored".into())
                })
                .is_none()
        );
        assert!(state.commit_busy);
        assert!(state.global_progress_visible());
        assert_eq!(state.projects[0].head.as_deref(), Some("before"));
    }
}

#[cfg(test)]
mod toolbar_action_tests {
    use super::*;
    fn state() -> Workspace {
        Workspace {
            projects: vec![Project {
                path: Some("D:/toolbar-fixture".into()),
                head: Some("head".into()),
                ..Default::default()
            }],
            project: 0,
            open_projects: vec![0],
            start_tab_active: false,
            sync_busy: true,
            sync_revision: 4,
            branch_owner: Some((0, "D:/toolbar-fixture".into())),
            notice: "unchanged".into(),
            ..Default::default()
        }
    }
    fn request(revision: u64) -> workspace_backend::ToolbarRequest {
        workspace_backend::ToolbarRequest {
            revision,
            project: 0,
            path: "D:/toolbar-fixture".into(),
            name: "fixture".into(),
            action: workspace_backend::ToolbarAction::CreateBranch {
                name: "feature".into(),
                checkout: true,
            },
        }
    }
    #[test]
    fn created_branch_partial_success_refreshes_and_releases_global_progress() {
        let mut state = state();
        assert!(state.global_progress_visible());
        assert!(state.repository_busy());
        let project = Project {
            path: Some("D:/toolbar-fixture".into()),
            head: Some("head".into()),
            branches: vec!["feature".into()],
            ..Default::default()
        };
        let history = PreparedHistory::new(&project.commits);
        let (completed, error) = state
            .finish_toolbar_action(workspace_backend::ToolbarResult {
                conflict_paths: Vec::new(),
                request: request(4),
                outcome: Ok(Some("created but checkout failed".into())),
                refreshed: Ok((project, history)),
            })
            .unwrap();
        assert!(completed);
        assert!(error.unwrap().contains("checkout failed"));
        assert_eq!(state.projects[0].branches, vec!["feature"]);
        assert!(!state.global_progress_visible());
        assert!(state.panel_pending.is_some());
        assert_eq!(state.notice, "unchanged");
    }
    #[test]
    fn failed_branch_keeps_owner_and_combines_refresh_error() {
        let mut state = state();
        let owner = state.branch_owner.clone();
        let (completed, error) = state
            .finish_toolbar_action(workspace_backend::ToolbarResult {
                conflict_paths: Vec::new(),
                request: request(4),
                outcome: Err("duplicate".into()),
                refreshed: Err("refresh failed".into()),
            })
            .unwrap();
        assert!(!completed);
        let error = error.unwrap();
        assert!(error.contains("duplicate") && error.contains("refresh failed"));
        assert_eq!(owner, state.branch_owner);
        assert!(!state.sync_busy);
    }
    #[test]
    fn stale_results_do_not_release_newer_work_or_replace_another_repository() {
        let mut state = state();
        assert!(
            state
                .finish_toolbar_action(workspace_backend::ToolbarResult {
                    conflict_paths: Vec::new(),
                    request: request(3),
                    outcome: Ok(None),
                    refreshed: Err("ignored".into())
                })
                .is_none()
        );
        assert!(state.sync_busy);
        let mut wrong = request(4);
        wrong.path = "D:/other".into();
        let result = state
            .finish_toolbar_action(workspace_backend::ToolbarResult {
                conflict_paths: Vec::new(),
                request: wrong,
                outcome: Ok(None),
                refreshed: Err("ignored".into()),
            })
            .unwrap();
        assert!(!result.0);
        assert_eq!(state.projects[0].head.as_deref(), Some("head"));
    }
    #[test]
    fn checkout_routes_only_double_clicks_of_local_remote_and_tags_and_rejects_stale_rows() {
        let mut state = state();
        state.sync_busy = false;
        state.projects[0].branches = vec!["main".into()];
        state.projects[0].remotes = vec!["origin/main".into()];
        state.panel_references = Ok(vec!["v1".into()]);
        for (tab, expected) in [
            (RepositoryTab::Local, "refs/heads/main"),
            (RepositoryTab::Remote, "refs/remotes/origin/main"),
            (RepositoryTab::Tags, "refs/tags/v1"),
        ] {
            state.repository_tab = tab;
            assert!(state.repository_checkout_target(0, tab, 0, 1).is_none());
            assert!(state.repository_checkout_target(0, tab, 0, 3).is_none());
            assert_eq!(
                state.repository_checkout_target(0, tab, 0, 2).as_deref(),
                Some(expected)
            );
            assert!(state.repository_checkout_target(1, tab, 0, 2).is_none());
            assert!(state.repository_checkout_target(0, tab, 1, 2).is_none());
            state.sync_busy = true;
            assert!(state.repository_checkout_target(0, tab, 0, 2).is_none());
            state.sync_busy = false;
        }
        state.repository_tab = RepositoryTab::Stashes;
        assert!(
            state
                .repository_checkout_target(0, RepositoryTab::Stashes, 0, 2)
                .is_none()
        );
        assert!(
            state
                .repository_checkout_target(0, RepositoryTab::Local, 0, 2)
                .is_none()
        );
    }
    #[test]
    fn checkout_menu_rejects_changed_selection_tabs_and_busy_state() {
        let mut state = state();
        state.sync_busy = false;
        state.projects[0].branches = vec!["main".into()];
        state.repository_tab = RepositoryTab::Local;
        state.repository_selection.select(0, 1);
        let row = (0, Some(RepositoryTab::Local), 0);
        assert_eq!(
            state.checkout_menu_target(row).as_deref(),
            Some("refs/heads/main")
        );
        state.repository_selection.clear();
        assert!(state.checkout_menu_target(row).is_none());
        state.repository_selection.select(0, 1);
        state.repository_tab = RepositoryTab::Remote;
        assert!(state.checkout_menu_target(row).is_none());
        state.commit_loading.reset();
        state.projects[0].commits = vec![Commit {
            id: "0123456789012345678901234567890123456789".into(),
            ..Default::default()
        }];
        state.commit = 0;
        let commit_row = (0, None, 0);
        assert_eq!(
            state.checkout_menu_target(commit_row),
            Some(state.projects[0].commits[0].id.clone())
        );
        assert!(state.checkout_menu_target((0, None, 999)).is_none());
        assert!(state.checkout_menu_target((1, None, 0)).is_none());
        state.sync_busy = true;
        assert!(state.checkout_menu_target(commit_row).is_none());
        state.sync_busy = false;
        state.start_tab_active = true;
        assert!(state.checkout_menu_target(commit_row).is_none());
    }

    #[test]
    fn checkout_confirmation_consumes_once_and_rejects_cancel_stale_and_busy_requests() {
        let mut state = state();
        state.sync_busy = false;
        let path = state.projects[0].path.clone().unwrap();
        let pending = (0, path, "refs/tags/v1".to_owned());
        state.checkout_confirmation = Some(pending.clone());
        assert!(state.repository_busy());
        assert!(state.take_confirmed_checkout(false).is_none());
        assert!(state.checkout_confirmation.is_none());
        state.checkout_confirmation = Some(pending.clone());
        assert_eq!(
            state.take_confirmed_checkout(true).as_deref(),
            Some("refs/tags/v1")
        );
        assert!(state.take_confirmed_checkout(true).is_none());
        state.checkout_confirmation = Some((1, pending.1.clone(), pending.2.clone()));
        assert!(state.take_confirmed_checkout(true).is_none());
        state.checkout_confirmation = Some((0, "different".into(), pending.2.clone()));
        assert!(state.take_confirmed_checkout(true).is_none());
        state.checkout_confirmation = Some(pending);
        state.sync_busy = true;
        assert!(state.take_confirmed_checkout(true).is_none());
    }

    #[test]
    fn repository_delete_confirmation_handles_stash_cancel_and_stale_selection() {
        let mut state = state();
        state.sync_busy = false;
        state.repository_tab = RepositoryTab::Stashes;
        state.panel_references = Ok(vec!["0: saved".into()]);
        state.repository_selection.select(0, 1);
        assert_eq!(
            state
                .repository_delete_target(0, RepositoryTab::Stashes, 0)
                .as_deref(),
            Some("stash@{0}")
        );
        let path = state.projects[0].path.clone().unwrap();
        let prompt = || ReferenceDeletePrompt {
            project: 0,
            path: path.clone(),
            tab: RepositoryTab::Stashes,
            index: 0,
            request: repository::ReferenceDeletion {
                reference: "stash@{0}".into(),
                expected: "id".into(),
                description: None,
                remote: None,
            },
        };
        state.reference_delete = Some(prompt());
        assert!(state.repository_busy());
        assert!(state.take_reference_delete(false).is_none());
        assert!(state.reference_delete.is_none());
        state.reference_delete = Some(prompt());
        assert!(state.take_reference_delete(true).is_some());
        assert!(state.take_reference_delete(true).is_none());
        state.reference_delete = Some(prompt());
        state.repository_selection.clear();
        assert!(state.take_reference_delete(true).is_none());
        state.repository_selection.select(0, 1);
        state.reference_delete = Some(prompt());
        state.repository_tab = RepositoryTab::Local;
        assert!(state.take_reference_delete(true).is_none());
        state.repository_tab = RepositoryTab::Stashes;
        state.reference_delete = Some(prompt());
        state.projects[0].path = Some("changed".into());
        assert!(state.take_reference_delete(true).is_none());
    }

    #[test]
    fn remote_checkout_reuses_exact_short_local_name() {
        let mut state = state();
        state.projects[0].remote_names = vec!["origin".into(), "team/upstream".into()];
        state.projects[0].branches = vec!["feature/login".into()];
        assert_eq!(
            state
                .existing_remote_local_branch("refs/remotes/origin/feature/login")
                .as_deref(),
            Some("refs/heads/feature/login")
        );
        assert_eq!(
            state
                .existing_remote_local_branch("refs/remotes/team/upstream/feature/login")
                .as_deref(),
            Some("refs/heads/feature/login")
        );
        assert!(
            state
                .existing_remote_local_branch("refs/remotes/origin/missing")
                .is_none()
        );
        assert!(
            state
                .existing_remote_local_branch("refs/tags/feature/login")
                .is_none()
        );
    }

    #[test]
    fn tag_dialog_preserves_commit_identity_and_rejects_changed_repository_owner() {
        let mut state = state();
        let path = state.projects[0].path.clone().unwrap();
        state.branch_owner = Some((0, path));
        state.tag_dialog_commit = Some("selected".into());
        assert!(state.tag_dialog_context("selected"));
        assert!(!state.tag_dialog_context("another"));
        state.commit = 999;
        assert!(state.tag_dialog_context("selected"));
        state.projects[0].path = Some("changed".into());
        assert!(!state.tag_dialog_context("selected"));
        state.projects[0].path = state.branch_owner.as_ref().map(|(_, path)| path.clone());
        state.start_tab_active = true;
        assert!(!state.tag_dialog_context("selected"));
    }

    #[test]
    fn history_double_click_requires_two_clicks_and_valid_idle_history() {
        let mut state = state();
        state.sync_busy = false;
        state.projects[0].commits = vec![Commit {
            id: "selected".into(),
            ..Default::default()
        }];
        state.commit_loading.reset();
        state.panel_waiting = false;
        assert_eq!(
            state.history_checkout_target(0, 0, 2).as_deref(),
            Some("selected")
        );
        for count in [0, 1, 3] {
            assert!(state.history_checkout_target(0, 0, count).is_none());
        }
        assert!(state.history_checkout_target(1, 0, 2).is_none());
        assert!(state.history_checkout_target(0, 1, 2).is_none());
        state.sync_busy = true;
        assert!(state.history_checkout_target(0, 0, 2).is_none());
        state.sync_busy = false;
        state.commit_loading.begin(1, std::time::Instant::now());
        assert!(state.history_checkout_target(0, 0, 2).is_none());
    }

    #[test]
    fn remote_tag_delete_option_is_explicit_and_reset_on_cancel_or_completion() {
        let mut state = state();
        state.sync_busy = false;
        state.repository_tab = RepositoryTab::Tags;
        state.panel_references = Ok(vec!["v1".into()]);
        state.repository_selection.select(0, 1);
        let path = state.projects[0].path.clone().unwrap();
        let prompt = || ReferenceDeletePrompt {
            project: 0,
            path: path.clone(),
            tab: RepositoryTab::Tags,
            index: 0,
            request: repository::ReferenceDeletion {
                reference: "refs/tags/v1".into(),
                expected: "id".into(),
                description: None,
                remote: None,
            },
        };
        assert!(!state.reference_delete_remote_tag);
        state.reference_delete = Some(prompt());
        assert!(matches!(
            state.take_reference_delete_action(true),
            Some(workspace_backend::ToolbarAction::DeleteReference {
                remote_tag: false,
                ..
            })
        ));
        state.reference_delete = Some(prompt());
        state.reference_delete_remote_tag = true;
        assert!(matches!(
            state.take_reference_delete_action(true),
            Some(workspace_backend::ToolbarAction::DeleteReference {
                remote_tag: true,
                ..
            })
        ));
        assert!(!state.reference_delete_remote_tag);
        state.reference_delete = Some(prompt());
        state.reference_delete_remote_tag = true;
        assert!(state.take_reference_delete_action(false).is_none());
        assert!(!state.reference_delete_remote_tag);
    }

    #[test]
    fn stash_dialog_requires_matching_repository_owner() {
        let mut state = state();
        let path = state.projects[0].path.clone().unwrap();
        state.branch_owner = Some((0, path));
        assert!(!state.stash_dialog_context());
        state.stash_dialog = true;
        assert!(state.stash_dialog_context());
        state.projects[0].path = Some("changed".into());
        assert!(!state.stash_dialog_context());
        state.projects[0].path = state.branch_owner.as_ref().map(|(_, path)| path.clone());
        state.start_tab_active = true;
        assert!(!state.stash_dialog_context());
    }

    #[test]
    fn stash_apply_menu_target_requires_stash_tab_current_selection_and_idle_owner() {
        let mut state = state();
        state.sync_busy = false;
        state.repository_tab = RepositoryTab::Stashes;
        state.panel_references = Ok(vec!["0: first".into(), "1: second".into()]);
        state.repository_selection.select(1, 2);
        let target = state.stash_apply_target(0, 1).unwrap();
        assert_eq!(target, ("stash@{1}".into(), "1: second".into()));
        assert!(state.stash_apply_target(0, 0).is_none());
        assert!(state.stash_apply_target(1, 1).is_none());
        assert!(state.stash_apply_target(0, 2).is_none());
        state.panel_references = Ok(vec!["0: first".into(), "1: changed".into()]);
        assert_ne!(state.stash_apply_target(0, 1), Some(target));
        state.repository_tab = RepositoryTab::Tags;
        assert!(state.stash_apply_target(0, 1).is_none());
        state.repository_tab = RepositoryTab::Stashes;
        state.sync_busy = true;
        assert!(state.stash_apply_target(0, 1).is_none());
    }

    #[test]
    fn stash_conflict_paths_ignore_stale_or_unrelated_results() {
        let mut state = state();
        let mut result = workspace_backend::ToolbarResult {
            request: request(4),
            outcome: Err("conflict".into()),
            refreshed: Err("not used".into()),
            conflict_paths: vec![std::path::PathBuf::from("folder/file.txt")],
        };
        result.request.action = workspace_backend::ToolbarAction::ApplyStash {
            request: repository::ReferenceDeletion {
                reference: "stash@{0}".into(),
                expected: "id".into(),
                description: None,
                remote: None,
            },
        };
        assert_eq!(
            state.toolbar_failure_paths(&result),
            vec!["folder/file.txt".to_owned()]
        );
        result.request.revision = 3;
        assert!(state.toolbar_failure_paths(&result).is_empty());
        result.request.revision = 4;
        result.request.path = "other".into();
        assert!(state.toolbar_failure_paths(&result).is_empty());
        result.request.path = state.projects[0].path.clone().unwrap();
        result.request.action = workspace_backend::ToolbarAction::Push;
        assert!(state.toolbar_failure_paths(&result).is_empty());
        state.start_tab_active = true;
        assert!(state.toolbar_failure_paths(&result).is_empty());
    }

    #[test]
    fn stash_display_ordinal_must_match_the_selected_row() {
        let mut state = state();
        state.sync_busy = false;
        state.repository_tab = RepositoryTab::Stashes;
        state.panel_references = Ok(vec!["9: mismatched".into()]);
        state.repository_selection.select(0, 1);
        assert!(
            state
                .repository_delete_target(0, RepositoryTab::Stashes, 0)
                .is_none()
        );
        state.panel_references = Ok(vec!["0: On main: named".into()]);
        assert_eq!(
            state
                .repository_delete_target(0, RepositoryTab::Stashes, 0)
                .as_deref(),
            Some("stash@{0}")
        );
    }

    #[test]
    fn checkout_result_follows_new_head_and_refreshes_files() {
        let mut state = state();
        state.projects[0].commits = vec![Commit {
            id: "head".into(),
            ..Default::default()
        }];
        state.commit = 0;
        let mut request = request(4);
        request.action = workspace_backend::ToolbarAction::Checkout {
            reference: "refs/tags/v1".into(),
        };
        let project = Project {
            path: Some("D:/toolbar-fixture".into()),
            head: Some("target".into()),
            commits: vec![Commit {
                id: "target".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let history = PreparedHistory::new(&project.commits);
        let result = state
            .finish_toolbar_action(workspace_backend::ToolbarResult {
                conflict_paths: Vec::new(),
                request,
                outcome: Ok(None),
                refreshed: Ok((project, history)),
            })
            .unwrap();
        assert!(result.0 && result.1.is_none());
        assert!(!state.sync_busy);
        assert_eq!(state.projects[0].head.as_deref(), Some("target"));
        assert_eq!(state.commit, 0);
        assert!(state.panel_pending.is_some());
    }
}

#[cfg(test)]
mod history_operation_tests {
    use super::*;
    fn workspace() -> Workspace {
        Workspace {
            projects: vec![Project { path: Some("D:/status-fixture".into()), ..Default::default() }],
            project: 0, open_projects: vec![0], start_tab_active: false,
            ..Default::default()
        }
    }
    fn checkout_state() -> Workspace {
        let mut state = workspace();
        state.commit_loading.reset();
        state.projects[0].commits = vec![repository::Commit { id: "selected".into(), ..Default::default() }];
        state
    }
    fn checkout_reference(name: &str) -> repository::CommitReference {
        repository::CommitReference { full_name: name.into(), name: name.into(), label: name.into(), is_remote: name.starts_with("refs/remotes/") }
    }
    #[test]
    fn history_checkout_candidates_prefer_local_then_remote_and_ignore_tags_and_head_alias() {
        let mut state = checkout_state(); let row = (0, None, 0);
        state.projects[0].commits[0].references = ["refs/heads/main", "refs/remotes/origin/main", "refs/remotes/origin/HEAD", "refs/tags/v1"].into_iter().map(checkout_reference).collect();
        assert_eq!(state.history_checkout_candidates(row).unwrap(), ["refs/heads/main"]);
        state.projects[0].commits[0].references.push(checkout_reference("refs/heads/feature"));
        assert_eq!(state.history_checkout_candidates(row).unwrap(), ["refs/heads/feature", "refs/heads/main"]);
        state.projects[0].commits[0].references.retain(|entry| !entry.full_name.starts_with("refs/heads/"));
        assert_eq!(state.history_checkout_candidates(row).unwrap(), ["refs/remotes/origin/main"]);
        state.projects[0].commits[0].references.retain(|entry| entry.full_name.starts_with("refs/tags/"));
        assert!(state.history_checkout_candidates(row).unwrap().is_empty());
    }
    #[test]
    fn history_checkout_branch_choice_consumes_once_and_rejects_cancel_changed_candidates_and_owner() {
        let mut state = checkout_state();
        state.projects[0].commits[0].references = ["refs/heads/one", "refs/heads/two"].into_iter().map(checkout_reference).collect();
        let pending = BranchCheckoutPrompt { project: 0, path: state.projects[0].path.clone().unwrap(), index: 0,
            commit: "selected".into(), candidates: vec!["refs/heads/one".into(), "refs/heads/two".into()], selected: 1,
            scroll: Default::default() };
        state.checkout_branch_prompt = Some(pending.clone());
        assert!(state.repository_busy());
        assert!(state.take_branch_checkout(false).is_none());
        state.checkout_branch_prompt = Some(pending.clone());
        assert_eq!(state.take_branch_checkout(true), Some(("refs/heads/two".into(), "selected".into())));
        assert!(state.take_branch_checkout(true).is_none());
        state.checkout_branch_prompt = Some(BranchCheckoutPrompt { selected: 2, ..pending.clone() });
        assert_eq!(state.take_branch_checkout(true), Some(("selected".into(), "selected".into())));
        state.checkout_branch_prompt = Some(pending.clone());
        state.projects[0].commits[0].references.pop();
        assert!(state.take_branch_checkout(true).is_none());
        state.projects[0].commits[0].references.push(checkout_reference("refs/heads/two"));
        state.checkout_branch_prompt = Some(BranchCheckoutPrompt { path: "different".into(), ..pending.clone() });
        assert!(state.take_branch_checkout(true).is_none());
        state.checkout_branch_prompt = Some(pending.clone()); state.sync_busy = true;
        assert!(state.take_branch_checkout(true).is_none());
        state.sync_busy = false;
        state.checkout_branch_prompt = Some(BranchCheckoutPrompt { selected: 99, ..pending });
        assert!(state.take_branch_checkout(true).is_none());
    }
    #[test]
    fn git_confirmations_cancel_consume_once_reject_stale_and_apply_selected_options() {
        use crate::git_action_dialog::ActionOptions;
        let mut state = checkout_state();
        let push = state.git_prompt(PendingGitAction::Toolbar(workspace_backend::ToolbarAction::Push)).unwrap();
        state.git_action_prompt = Some(push.clone()); assert!(state.repository_busy());
        assert!(state.take_git_confirmation(None).is_none());
        state.git_action_prompt = Some(push.clone());
        assert!(matches!(state.take_git_confirmation(Some(ActionOptions::Push(false))), Some(PendingGitAction::Toolbar(workspace_backend::ToolbarAction::Push))));
        assert!(state.take_git_confirmation(Some(ActionOptions::Push(false))).is_none());
        state.git_action_prompt = Some(push.clone());
        assert!(matches!(state.take_git_confirmation(Some(ActionOptions::Push(true))), Some(PendingGitAction::Toolbar(workspace_backend::ToolbarAction::PushWithLease))));
        state.git_action_prompt = Some(push.clone());
        assert!(state.take_git_confirmation(Some(ActionOptions::Confirm)).is_none());
        state.git_action_prompt = Some(push.clone()); state.projects[0].head = Some("changed".into());
        assert!(state.take_git_confirmation(Some(ActionOptions::Push(false))).is_none());
        state.projects[0].head = push.head.clone();
        let request = repository::CommitHistoryRequest { target: "selected".into(), expected_head: "head".into(), expected_reference: None };
        for reset in [false, true] {
            let action = if reset { workspace_backend::ToolbarAction::ResetCommit { request: request.clone(), mode: repository::ResetMode::Mixed } }
                else { workspace_backend::ToolbarAction::RevertCommit { request: request.clone(), no_commit: false } };
            let pending = state.git_prompt(PendingGitAction::Toolbar(action)).unwrap();
            state.git_action_prompt = Some(pending.clone());
            let options = if reset { ActionOptions::Reset(repository::ResetMode::Hard) } else { ActionOptions::Revert(true) };
            let confirmed = state.take_git_confirmation(Some(options)).unwrap();
            if reset { assert!(matches!(confirmed, PendingGitAction::Toolbar(workspace_backend::ToolbarAction::ResetCommit { mode: repository::ResetMode::Hard, .. }))); }
            else { assert!(matches!(confirmed, PendingGitAction::Toolbar(workspace_backend::ToolbarAction::RevertCommit { no_commit: true, .. }))); }
            state.git_action_prompt = Some(pending); state.commit = 1;
            assert!(state.take_git_confirmation(Some(options)).is_none()); state.commit = 0;
        }
    }

    #[test]
    fn history_rebase_requests_filter_local_targets_and_reject_stale_rows() {
        let mut state = workspace();
        let project = &mut state.projects[0];
        project.branch = "main".into(); project.head = Some("selected".into());
        project.branches = vec!["main".into(), "target".into(), "other".into()];
        let reference = |name: &str| repository::CommitReference { full_name: name.into(), name: name.into(),
            label: name.into(), is_remote: name.starts_with("refs/remotes/") };
        project.commits = vec![repository::Commit { id: "selected".into(), references: vec![
            reference("refs/heads/target"), reference("refs/heads/main"), reference("refs/tags/v1"),
            reference("refs/remotes/origin/main"), reference("refs/heads/other"), reference("refs/heads/target")], ..Default::default() }];
        state.commit_loading.reset();
        let row = (0, None, 0);
        assert!(state.history_rebase_request(row, None).is_some());
        assert_eq!(state.history_rebase_branches(row), ["refs/heads/other", "refs/heads/target"]);
        assert!(state.history_rebase_request(row, Some("refs/heads/target")).is_some());
        assert!(state.history_rebase_request(row, Some("refs/heads/main")).is_none());
        assert!(state.history_rebase_request(row, Some("refs/tags/v1")).is_none());
        assert!(state.history_rebase_request((0, Some(RepositoryTab::Local), 0), None).is_none());
        state.sync_busy = true;
        assert!(state.history_rebase_request(row, None).is_none());
        state.sync_busy = false; state.commit = 1;
        assert!(state.history_rebase_request(row, None).is_none());
    }
    #[test]
    fn revert_buttons_and_abort_confirmation_guard_conflicts_head_and_busy_state() {
        let mut state = workspace(); state.projects[0].head = Some("head".into());
        for operation in [git2::RepositoryState::Revert, git2::RepositoryState::RevertSequence] {
            state.projects[0].work_state.operation = Some(operation);
            state.projects[0].work_state.has_conflicts = true;
            assert!(!state.revert_action_available(true)); assert!(state.revert_action_available(false));
            state.projects[0].work_state.has_conflicts = false;
            assert!(state.revert_action_available(true));
            let pending = (0, state.projects[0].path.clone().unwrap(), "head".to_owned());
            state.abort_operation = AbortOperation::Revert;
            state.merge_abort_confirmation = Some(pending.clone()); assert!(state.take_confirmed_merge_abort(false).is_none());
            state.merge_abort_confirmation = Some(pending.clone()); assert_eq!(state.take_confirmed_merge_abort(true).as_deref(), Some("head"));
            assert!(state.take_confirmed_merge_abort(true).is_none());
            state.merge_abort_confirmation = Some(pending); state.projects[0].head = Some("changed".into());
            assert!(state.take_confirmed_merge_abort(true).is_none()); state.projects[0].head = Some("head".into());
            state.sync_busy = true; assert!(!state.revert_action_available(true)); state.sync_busy = false;
        }
        state.projects[0].work_state.operation = Some(git2::RepositoryState::Merge);
        assert!(!state.revert_action_available(false));
    }

    #[test]
    fn history_rebase_controls_and_abort_confirmation_follow_pending_state() {
        let mut state = workspace(); state.projects[0].head = Some("head".into());
        state.projects[0].work_state.operation = Some(git2::RepositoryState::RebaseMerge);
        state.projects[0].work_state.has_conflicts = true;
        assert!(!state.integration_action_available(true, true));
        assert!(state.integration_action_available(true, false));
        assert!(!state.merge_action_available(false));
        state.projects[0].work_state.has_conflicts = false;
        assert!(state.integration_action_available(true, true));
        state.abort_operation = AbortOperation::Rebase;
        let pending = (0, state.projects[0].path.clone().unwrap(), "head".to_owned());
        state.merge_abort_confirmation = Some(pending.clone());
        assert!(state.take_confirmed_merge_abort(false).is_none());
        state.merge_abort_confirmation = Some(pending.clone());
        assert_eq!(state.take_confirmed_merge_abort(true).as_deref(), Some("head"));
        assert!(state.take_confirmed_merge_abort(true).is_none());
        state.merge_abort_confirmation = Some(pending);
        state.projects[0].work_state.operation = None;
        assert!(state.take_confirmed_merge_abort(true).is_none());
    }
    #[test]
    fn merge_failure_paths_reject_stale_pull_and_merge_results() {
        let mut state = workspace();
        let mut result = workspace_backend::SyncResult {
            request: workspace_backend::SyncRequest { revision: state.sync_revision, project: 0,
                path: state.projects[0].path.clone().unwrap(), name: "fixture".into(), mode: repository::SyncMode::Merge },
            conflict_paths: vec![std::path::PathBuf::from("한글 file.txt")],
            outcome: Err("blocked".into()), refreshed: Err("unused".into()),
        };
        assert_eq!(state.sync_failure_paths(&result), ["한글 file.txt"]);
        result.request.revision += 1;
        assert!(state.sync_failure_paths(&result).is_empty());
        result.request.revision = state.sync_revision;
        result.request.mode = repository::SyncMode::Fetch;
        assert!(state.sync_failure_paths(&result).is_empty());
        let mut toolbar = workspace_backend::ToolbarResult {
            request: workspace_backend::ToolbarRequest { revision: state.sync_revision, project: 0,
                path: state.projects[0].path.clone().unwrap(), name: "fixture".into(),
                action: workspace_backend::ToolbarAction::MergeCommit { id: "target".into() } },
            conflict_paths: vec![std::path::PathBuf::from("한글 file.txt")],
            outcome: Err("blocked".into()), refreshed: Err("unused".into()),
        };
        assert_eq!(state.toolbar_failure_paths(&toolbar), ["한글 file.txt"]);
        toolbar.request.path = "other".into();
        assert!(state.toolbar_failure_paths(&toolbar).is_empty());
        state.start_tab_active = true;
        assert!(state.sync_failure_paths(&result).is_empty());
    }
    #[test]
    fn merge_toolbar_buttons_require_idle_merge_and_commit_requires_resolved_conflicts() {
        let mut state = workspace();
        state.projects[0].head = Some("head".into());
        assert!(!state.merge_action_available(false));
        state.projects[0].work_state.operation = Some(git2::RepositoryState::Merge);
        assert!(state.merge_action_available(true));
        assert!(state.merge_action_available(false));
        state.projects[0].work_state.has_conflicts = true;
        assert!(!state.merge_action_available(true));
        assert!(state.merge_action_available(false));
        state.sync_busy = true;
        assert!(!state.merge_action_available(false));
        state.sync_busy = false;
        state.panel_waiting = true;
        assert!(!state.merge_action_available(false));
        state.panel_waiting = false;
        state.projects[0].work_state.error = Some("read failed".into());
        assert!(!state.merge_action_available(false));
    }
    #[test]
    fn merge_abort_confirmation_is_consumed_once_and_rejects_stale_contexts() {
        let mut state = workspace();
        state.projects[0].head = Some("head".into());
        state.projects[0].work_state.operation = Some(git2::RepositoryState::Merge);
        let pending = (0, state.projects[0].path.clone().unwrap(), "head".to_owned());
        state.merge_abort_confirmation = Some(pending.clone());
        assert!(state.repository_busy());
        assert!(state.take_confirmed_merge_abort(false).is_none());
        state.merge_abort_confirmation = Some(pending.clone());
        assert_eq!(state.take_confirmed_merge_abort(true).as_deref(), Some("head"));
        assert!(state.take_confirmed_merge_abort(true).is_none());
        state.merge_abort_confirmation = Some(pending.clone());
        state.projects[0].head = Some("different".into());
        assert!(state.take_confirmed_merge_abort(true).is_none());
        state.projects[0].head = Some("head".into());
        state.merge_abort_confirmation = Some((1, pending.1.clone(), pending.2.clone()));
        assert!(state.take_confirmed_merge_abort(true).is_none());
        state.merge_abort_confirmation = Some(pending.clone());
        state.sync_busy = true;
        assert!(state.take_confirmed_merge_abort(true).is_none());
        state.sync_busy = false;
        state.merge_abort_confirmation = Some(pending);
        state.projects[0].work_state.operation = None;
        assert!(state.take_confirmed_merge_abort(true).is_none());
    }

    #[test]
    fn history_operation_names_cover_all_pending_git_states() {
        use git2::RepositoryState::*;
        let mut state = workspace();
        for (operation, name) in [(Merge, "병합"), (Rebase, "리베이스"),
            (RebaseInteractive, "리베이스"), (RebaseMerge, "리베이스"),
            (CherryPick, "체리픽"), (CherryPickSequence, "체리픽"),
            (Revert, "되돌리기"), (RevertSequence, "되돌리기"), (Bisect, "이분 탐색"),
            (ApplyMailbox, "패치 적용"), (ApplyMailboxOrRebase, "패치 적용 또는 리베이스")] {
            state.projects[0].work_state.operation = Some(operation);
            assert_eq!(state.history_operation_status().unwrap().0, format!("{name} 진행 중 · 완료 대기"));
        }
        state.projects[0].work_state.operation = None;
        state.projects[0].work_state.has_conflicts = true;
        assert_eq!(state.history_operation_status().unwrap(), ("충돌 해결 필요".into(), true));
    }

    #[test]
    fn history_operation_status_tracks_execution_conflicts_completion_and_errors() {
        let mut state = workspace();
        assert!(state.history_operation_status().is_none());
        state.projects[0].work_state.operation = Some(git2::RepositoryState::Merge);
        state.projects[0].work_state.has_conflicts = true;
        assert_eq!(state.history_operation_status().unwrap().0, "병합 진행 중 · 충돌 해결 필요");
        state.sync_busy = true;
        state.sync_operation = Some("병합 실행 중…");
        assert_eq!(state.history_operation_status().unwrap().0, "병합 실행 중…");
        state.commit_busy = true;
        assert_eq!(state.history_operation_status().unwrap().0, "커밋 실행 중…");
        state.commit_busy = false;
        state.sync_busy = false;
        state.projects[0].work_state.has_conflicts = false;
        assert_eq!(state.history_operation_status().unwrap().0, "병합 진행 중 · 완료 대기");
        state.projects[0].work_state = Default::default();
        assert!(state.history_operation_status().is_none());
        state.projects[0].work_state.error = Some("locked".into());
        assert!(state.history_operation_status().unwrap().0.contains("확인 실패"));
        state.start_tab_active = true;
        assert!(state.history_operation_status().is_none());
        state.start_tab_active = false;
        state.projects.push(Project::default());
        state.project = 1;
        state.open_projects.push(1);
        assert!(state.history_operation_status().is_none());
    }
    #[test]
    fn history_operation_panel_updates_only_current_results_and_reports_failure() {
        let mut state = workspace();
        let request = PanelRequest { revision: state.panel_revision, project: 0,
            path: state.projects[0].path.clone(), historical: false, commit: None,
            graph: None, selected_files: Vec::new() };
        let result = |request: PanelRequest, work_state| PanelResult {
            request, work_state, files: Ok(Vec::new()), displayed_files: Ok(Vec::new()), history: None,
        };
        let pending = repository::RepositoryWorkState { operation: Some(git2::RepositoryState::RebaseMerge),
            has_conflicts: true, error: None };
        let mut stale = request.clone(); stale.revision += 1;
        assert!(!state.finish_panel(result(stale, Ok(pending.clone()))));
        assert!(state.history_operation_status().is_none());
        assert!(state.finish_panel(result(request.clone(), Ok(pending))));
        assert_eq!(state.history_operation_status().unwrap().0, "리베이스 진행 중 · 충돌 해결 필요");
        assert!(state.finish_panel(result(request, Err("read failed".into()))));
        assert!(state.history_operation_status().unwrap().0.contains("확인 실패"));
    }
}
