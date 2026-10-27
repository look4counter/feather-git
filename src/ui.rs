use crate::commit_editor::CommitEditor;
use crate::history_view::{self, ColumnLayout, RowGraph};
use crate::layout::{SplitAxis, SplitLayout};
use crate::settings::SettingsPanel;
use crate::{
    session::{self, Session},
    text_selection::{self, Position, Selection},
};
use feather_git::{git, model::*};
use gpui::{prelude::*, *};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Default)]
struct StartPage {
    url: String,
    clone_form: bool,
    busy: bool,
    error: Option<String>,
    select_all: bool,
    recent_scroll: UniformListScrollHandle,
}

struct CommitDialog {
    project_id: u64,
    editor: Entity<CommitEditor>,
    busy: bool,
    push_after: bool,
    error: Option<String>,
}

struct DiscardDialog {
    project_id: u64,
    paths: Vec<Vec<u8>>,
    busy: bool,
    error: Option<String>,
}

#[derive(Clone)]
enum BranchAction {
    Create { commit: String },
    Track(crate::branches::CheckoutTarget),
}

struct BranchDialog {
    project_id: u64,
    path: PathBuf,
    action: BranchAction,
    editor: Option<Entity<CommitEditor>>,
    error: Option<String>,
}

const BG: u32 = 0x10151d;
const PANEL: u32 = 0x151c26;
const LINE: u32 = 0x293343;
const TEXT: u32 = 0xd9e2ed;
const MUTED: u32 = 0x8290a6;
const ACCENT: u32 = 0x5edac1;
const SELECT: u32 = 0x203d42;
const SELECT_INACTIVE: u32 = 0x303844;
const TAB_BAR_HEIGHT: f32 = 44.;
const SCROLLBAR: f32 = 12.;
const RECENT_ROW_HEIGHT: f32 = 40.;

fn branch_click_interval() -> std::time::Duration {
    #[cfg(windows)]
    let millis =
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() } as u64;
    #[cfg(not(windows))]
    let millis = 500;
    std::time::Duration::from_millis(millis + 30)
}
type ListKey = (u64, u8);

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListFocus {
    Branches(u64),
    Files(u64),
    History(u64),
    Text(ListKey),
}

#[derive(Clone)]
struct ScrollDrag {
    key: ListKey,
    handle: UniformListScrollHandle,
    horizontal: bool,
    grab: f32,
}

enum WorkspaceTab {
    Empty(u64),
    Repository(Box<Project>),
}

impl WorkspaceTab {
    fn open_in_place(tabs: &mut Vec<Self>, source: u64, path: PathBuf) -> Option<u64> {
        let index = tabs
            .iter()
            .position(|tab| matches!(tab, Self::Empty(id) if *id == source))?;
        if let Some(id) = tabs
            .iter()
            .filter_map(Self::project)
            .find(|project| project.path == path)
            .map(|project| project.id)
        {
            tabs.remove(index);
            Some(id)
        } else {
            tabs[index] = Self::Repository(Box::new(Project::new(source, path)));
            Some(source)
        }
    }

    fn id(&self) -> u64 {
        match self {
            Self::Empty(id) => *id,
            Self::Repository(project) => project.id,
        }
    }

    fn project(&self) -> Option<&Project> {
        match self {
            Self::Empty(_) => None,
            Self::Repository(project) => Some(project),
        }
    }

    fn project_mut(&mut self) -> Option<&mut Project> {
        match self {
            Self::Empty(_) => None,
            Self::Repository(project) => Some(project),
        }
    }
}

pub struct Workspace {
    state_path: Option<PathBuf>,
    recent: Vec<PathBuf>,
    pending: usize,
    staging: HashSet<u64>,
    remote_status: Option<(u64, &'static str)>,
    help: bool,
    settings: Option<Entity<SettingsPanel>>,
    branch_dialog: Option<BranchDialog>,
    branch_busy: bool,
    commit_dialog: Option<CommitDialog>,
    discard_dialog: Option<DiscardDialog>,
    delete_target: Option<PathBuf>,
    delete_confirm: bool,
    focus: FocusHandle,
    list_focus: Option<ListFocus>,
    text_selection: Option<(ListKey, Selection)>,
    text_drag: bool,
    tabs: Vec<WorkspaceTab>,
    tabs_scroll: ScrollHandle,
    active: Option<u64>,
    next_id: u64,
    error: Option<String>,
    layout: SplitLayout,
    observed_status_error: (Option<String>, Option<(u64, String)>),
    status_error: Option<String>,
    status_error_generation: u64,
    body_bounds: Bounds<Pixels>,
    resize_drag: Option<(SplitAxis, Pixels)>,
    start_pages: HashMap<u64, StartPage>,
    url_focus: FocusHandle,
    graphs: HashMap<u64, HashMap<usize, RowGraph>>,
    history_scroll: HashMap<u64, UniformListScrollHandle>,
    grid_columns: ColumnLayout,
    grid_width: f32,
    column_drag: Option<(usize, f32, ColumnLayout, f32)>,
    list_scroll: HashMap<ListKey, UniformListScrollHandle>,
    tracks: HashMap<(ListKey, bool), Bounds<Pixels>>,
    viewports: HashMap<ListKey, Bounds<Pixels>>,
    diff_measured: HashSet<u64>,
    scroll_drag: Option<ScrollDrag>,
    file_items: HashMap<ListKey, Vec<FileRow>>,
    detail_items: HashMap<u64, TextRows>,
    diff_width_row: HashMap<u64, usize>,
    diff_content_width: HashMap<u64, f32>,
}

fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .bg(rgb(if active { SELECT } else { PANEL }))
        .text_color(rgb(if active { ACCENT } else { MUTED }))
        .hover(|s| s.bg(rgb(0x263443)).text_color(rgb(TEXT)))
        .child(label.into())
}

fn panel() -> Div {
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_w_0()
        .min_h_0()
        .bg(rgb(PANEL))
        .overflow_hidden()
}

fn empty(message: &str) -> Div {
    div()
        .p_5()
        .text_sm()
        .text_color(rgb(MUTED))
        .child(message.to_owned())
}

fn trash_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let origin = bounds.origin;
            let mut path = PathBuilder::stroke(px(1.5));
            for points in [
                &[(3., 5.), (15., 5.)][..],
                &[(6., 5.), (6., 2.), (12., 2.), (12., 5.)][..],
                &[(4., 5.), (5., 16.), (13., 16.), (14., 5.)][..],
                &[(7., 8.), (7., 13.)][..],
                &[(11., 8.), (11., 13.)][..],
            ] {
                path.move_to(origin + point(px(points[0].0), px(points[0].1)));
                for &(x, y) in &points[1..] {
                    path.line_to(origin + point(px(x), px(y)));
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(MUTED));
            }
        },
    )
    .size(px(18.))
}

fn shortcut_help(window: &Window) -> Stateful<Div> {
    let sections: &[(&str, &[&str])] = &[
        (
            "공통",
            &[
                "f - 패치",
                "f5 - 새로고침",
                "ctrl + r - 새로고침",
                "c - 커밋",
                "p - 풀",
                "shift + p - 푸시",
                "? - 도움말",
                "f12 - 세팅",
                "esc - 도움말 닫기",
            ],
        ),
        ("브랜치", &[]),
        ("커밋히스토리", &["b - 선택 커밋에 브랜치 만들기"]),
        (
            "파일목록",
            &[
                "shift + ↑ - 위로 범위 선택",
                "shift + ↓ - 아래로 범위 선택",
                "ctrl + a - 전체 선택",
                "space - 선택 파일 stage / unstage",
                "d - 선택 파일 변경사항 폐기 확인",
            ],
        ),
        (
            "텍스트뷰어",
            &[
                "방향키 - 커서 이동",
                "home - 줄 처음으로 이동",
                "end - 줄 끝으로 이동",
                "shift + 방향키 - 선택 확장",
                "shift + home - 줄 처음까지 선택",
                "shift + end - 줄 끝까지 선택",
                "ctrl + a - 전체 선택",
                "ctrl + c - 복사",
            ],
        ),
    ];
    let mut help = div()
        .id("shortcut-help")
        .flex()
        .flex_col()
        .gap_3()
        .max_h(window.viewport_size().height - px(120.))
        .overflow_y_scroll();
    for &(title, shortcuts) in sections {
        let mut section = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .child(div().text_color(rgb(ACCENT)).child(title));
        if title == "공통" {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("입력창과 확인창에서는 제외"),
            );
        } else if title == "파일목록" {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("stage 파일목록 포커스 필요"),
            );
        } else if title == "텍스트뷰어" {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("텍스트뷰어 포커스 필요"),
            );
        }
        if shortcuts.is_empty() {
            section = section.child(div().text_color(rgb(MUTED)).child("표시할 단축키 없음"));
        }
        for &shortcut in shortcuts {
            section = section.child(shortcut);
        }
        help = help.child(section);
    }
    help
}

impl Workspace {
    pub fn new(paths: Vec<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            state_path: session::state_path().ok(),
            recent: Vec::new(),
            pending: 0,
            staging: HashSet::new(),
            remote_status: None,
            help: false,
            settings: None,
            branch_dialog: None,
            branch_busy: false,
            commit_dialog: None,
            discard_dialog: None,
            delete_target: None,
            delete_confirm: false,
            focus: cx.focus_handle(),
            list_focus: None,
            text_selection: None,
            text_drag: false,
            tabs: Vec::new(),
            tabs_scroll: ScrollHandle::new(),
            active: None,
            next_id: 0,
            error: None,
            layout: SplitLayout::default(),
            observed_status_error: (None, None),
            status_error: None,
            status_error_generation: 0,
            body_bounds: Bounds::default(),
            resize_drag: None,
            start_pages: HashMap::new(),
            url_focus: cx.focus_handle(),
            graphs: HashMap::new(),
            history_scroll: HashMap::new(),
            grid_columns: ColumnLayout::default(),
            grid_width: 0.,
            column_drag: None,
            list_scroll: HashMap::new(),
            tracks: HashMap::new(),
            viewports: HashMap::new(),
            diff_measured: HashSet::new(),
            scroll_drag: None,
            file_items: HashMap::new(),
            detail_items: HashMap::new(),
            diff_width_row: HashMap::new(),
            diff_content_width: HashMap::new(),
        };
        this.restore(paths, cx);
        this
    }

    fn restore(&mut self, arguments: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.pending += 1;
        let state_path = self.state_path.clone();
        let task = cx.background_executor().spawn(async move {
            let loaded = state_path
                .as_deref()
                .map(session::load)
                .unwrap_or_else(|| Err(anyhow::anyhow!("작업 공간 저장 경로가 없습니다.")));
            let can_save = loaded.is_ok();
            let mut errors = Vec::new();
            let mut state = loaded.unwrap_or_else(|error| {
                errors.push(format!("{error:#}"));
                Session::default()
            });
            errors.extend(session::restore_paths(&mut state, arguments));
            (state, errors, can_save)
        });
        cx.spawn(async move |this, cx| {
            let (state, errors, can_save) = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                if !can_save {
                    this.state_path = None;
                }
                this.recent = state.recent;
                for path in state.tabs {
                    let id = this.next_id;
                    this.next_id += 1;
                    if let Some(path) = path {
                        this.tabs
                            .push(WorkspaceTab::Repository(Box::new(Project::new(id, path))));
                    } else {
                        this.tabs.push(WorkspaceTab::Empty(id));
                        this.start_pages.insert(id, StartPage::default());
                    }
                }
                this.active = this.tabs.get(state.active).map(WorkspaceTab::id);
                let active = this.active;
                this.ensure_active_loaded(cx);
                this.persist();
                if !errors.is_empty() {
                    this.error = Some(errors.join("\n"));
                }
                if let Some(index) = this.tabs.iter().position(|tab| Some(tab.id()) == active) {
                    this.tabs_scroll.scroll_to_item(index);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn persist(&mut self) {
        let Some(path) = &self.state_path else {
            return;
        };
        let state = Session {
            version: 1,
            tabs: self
                .tabs
                .iter()
                .map(|tab| tab.project().map(|p| p.path.clone()))
                .collect(),
            active: self
                .tabs
                .iter()
                .position(|tab| Some(tab.id()) == self.active)
                .unwrap_or(0),
            recent: self.recent.clone(),
        };
        if let Err(error) = session::save(path, &state) {
            self.error = Some(format!("{error:#}"));
        }
    }

    fn busy(&self) -> bool {
        self.pending > 0
            || self.start_pages.values().any(|p| p.busy)
            || self
                .tabs
                .iter()
                .filter_map(WorkspaceTab::project)
                .any(|p| p.loading || p.diff_loading || p.page_loading)
    }

    fn history_in_progress(&self) -> bool {
        self.tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .any(Project::history_in_progress)
    }

    fn update_status_error(&mut self, cx: &mut Context<Self>) {
        let current = (
            self.error.clone(),
            self.project()
                .and_then(|project| project.error.clone().map(|error| (project.id, error))),
        );
        if current == self.observed_status_error {
            return;
        }
        let new_error = if current.0 != self.observed_status_error.0 {
            current.0.clone()
        } else {
            None
        }
        .or_else(|| {
            if current.1 != self.observed_status_error.1 {
                current.1.as_ref().map(|(_, error)| error.clone())
            } else {
                None
            }
        });
        self.observed_status_error = current;
        let Some(error) = new_error else {
            return;
        };
        self.status_error = Some(error);
        self.status_error_generation += 1;
        let generation = self.status_error_generation;
        let timer = cx
            .background_executor()
            .timer(std::time::Duration::from_secs(3));
        cx.spawn(async move |this, cx| {
            timer.await;
            this.update(cx, |this, cx| {
                if this.status_error_generation == generation {
                    this.status_error = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn text_lines(&self, key: ListKey) -> Vec<String> {
        if key.1 == 31 {
            self.detail_items
                .get(&key.0)
                .map(|r| r.lines.clone())
                .unwrap_or_default()
        } else {
            self.tabs
                .iter()
                .filter_map(WorkspaceTab::project)
                .find(|p| p.id == key.0)
                .map(|p| p.diff.lines.iter().map(|line| line.text.clone()).collect())
                .unwrap_or_default()
        }
    }

    fn selection_color(&self, section: ListFocus, window: &Window) -> u32 {
        if self.list_focus == Some(section)
            && self.focus.is_focused(window)
            && !self.history_in_progress()
            && !self.help
            && self.settings.is_none()
            && self.branch_dialog.is_none()
            && !self.branch_busy
            && self.delete_target.is_none()
            && self.commit_dialog.is_none()
            && self.discard_dialog.is_none()
            && self.active
                == Some(match section {
                    ListFocus::Branches(id) | ListFocus::Files(id) | ListFocus::History(id) => id,
                    ListFocus::Text(key) => key.0,
                })
        {
            SELECT
        } else {
            SELECT_INACTIVE
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_in_progress() || self.branch_busy {
            cx.stop_propagation();
            return;
        }
        let key = &event.keystroke;
        if let Some(dialog) = &self.discard_dialog {
            if !dialog.busy && key.modifiers.control && key.key == "s" {
                self.discard_selected(cx);
                cx.stop_propagation();
                return;
            }
            if !dialog.busy && key.key == "escape" {
                self.discard_dialog = None;
                self.focus.focus(window);
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if let Some(dialog) = &self.commit_dialog {
            if dialog.busy {
                cx.stop_propagation();
                return;
            }
            if key.modifiers.control && (key.key == "s" || key.key == "p") {
                self.submit_commit(key.key == "p", cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "escape" {
                if dialog.editor.read(cx).is_composing() {
                    return;
                }
                self.commit_dialog = None;
                self.focus.focus(window);
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }
        if let Some(settings) = self.settings.clone() {
            if key.key == "escape" && !settings.read(cx).is_composing(cx) {
                self.close_settings(window, cx);
                cx.stop_propagation();
            } else if key.modifiers.control && key.key == "s" {
                settings.update(cx, |settings, cx| settings.save(cx));
                cx.stop_propagation();
            } else if key.key == "tab" && !key.modifiers.control && !key.modifiers.alt {
                settings
                    .read(cx)
                    .focus_next(key.modifiers.shift, window, cx);
                cx.stop_propagation();
            }
            return;
        }
        if let Some(dialog) = &self.branch_dialog {
            let composing = dialog
                .editor
                .as_ref()
                .is_some_and(|editor| editor.read(cx).is_composing());
            if key.key == "escape" && !composing {
                self.close_branch_dialog(window, cx);
                cx.stop_propagation();
            } else if !composing
                && (key.key == "enter" || (key.modifiers.control && key.key == "s"))
            {
                self.submit_branch(cx);
                cx.stop_propagation();
            }
            return;
        }
        if self.help || self.delete_target.is_some() {
            if key.key == "escape" {
                self.help = false;
                self.delete_target = None;
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if key.key == "f12"
            && !key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.platform
        {
            self.open_settings(window, cx);
            cx.stop_propagation();
            return;
        }
        if self.url_focus.is_focused(window) {
            return;
        }
        if !key.modifiers.alt
            && !key.modifiers.platform
            && !key.modifiers.shift
            && ((!key.modifiers.control && key.key == "f5")
                || (key.modifiers.control && key.key == "r"))
        {
            if self.pending == 0 {
                self.reload(cx);
            }
            cx.stop_propagation();
            return;
        }
        if !key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform {
            if key.key == "b" && !key.modifiers.shift {
                self.open_branch_create(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "?" || key.key_char.as_deref() == Some("?") {
                self.help = true;
                cx.stop_propagation();
                cx.notify();
                return;
            }
            if key.key == "f" {
                self.remote_operation(crate::setup::RemoteOperation::Fetch, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "c" {
                self.open_commit(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "d" {
                self.open_discard(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key.eq_ignore_ascii_case("p") {
                let operation = if key.modifiers.shift {
                    crate::setup::RemoteOperation::Push
                } else {
                    crate::setup::RemoteOperation::Pull
                };
                self.remote_operation(operation, cx);
                cx.stop_propagation();
                return;
            }
        }
        if let Some(ListFocus::Text(text_key)) = self.list_focus {
            if Some(text_key.0) != self.active {
                return;
            }
            let lines = self.text_lines(text_key);
            if let Some((selection_key, selection)) = &mut self.text_selection
                && *selection_key == text_key
            {
                if key.modifiers.control && key.key == "c" {
                    cx.write_to_clipboard(ClipboardItem::new_string(selection.text(&lines)));
                } else if key.modifiers.control && key.key == "a" {
                    selection.anchor = Position::default();
                    selection.cursor = Position {
                        row: lines.len().saturating_sub(1),
                        column: lines.last().map_or(0, |line| line.chars().count()),
                    };
                } else if ["up", "down", "left", "right", "home", "end"].contains(&key.key.as_str())
                {
                    selection.move_cursor(&lines, &key.key, key.modifiers.shift);
                    if let Some(scroll) = self.list_scroll.get(&text_key) {
                        scroll.scroll_to_item(selection.cursor.row, ScrollStrategy::Center);
                    }
                }
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }
        if key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform && key.key == "a"
        {
            if let Some(ListFocus::Files(id)) = self.list_focus
                && self.active == Some(id)
                && let Some(project) = self.project_mut()
                && project.file_tab == FileTab::Stage
            {
                project.stage_selection.select_all(&project.snapshot.stage);
                if project.selected_file.is_none() && !project.snapshot.stage.is_empty() {
                    self.select_file(0, cx);
                }
                cx.notify();
                cx.stop_propagation();
            }
            return;
        }
        if !key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.platform
            && !key.modifiers.shift
            && (key.key == "space" || key.key_char.as_deref() == Some(" "))
        {
            self.stage_selected(cx);
            cx.stop_propagation();
            return;
        }
        if !key.modifiers.control
            && !key.modifiers.alt
            && ["up", "down"].contains(&key.key.as_str())
        {
            self.navigate(key.key == "down", key.modifiers.shift, cx);
            cx.stop_propagation();
        }
    }

    fn navigate(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) {
        let Some(focus) = self.list_focus else {
            return;
        };
        let Some(p) = self.project() else {
            return;
        };
        let id = p.id;
        match focus {
            ListFocus::Branches(project) if project == id => {
                let kind = p.ref_tab.index();
                let rows = &p.snapshot.reference_rows[kind];
                let current = rows.iter().position(|index| {
                    let revision = &p.snapshot.references[*index].revision;
                    p.revision.as_ref().or(p.snapshot.current_branch.as_ref()) == Some(revision)
                });
                if let Some(index) = text_selection::adjacent(current, rows.len(), down) {
                    let reference = p.snapshot.references[rows[index]].clone();
                    if current == Some(index) {
                        return;
                    }
                    if let Some(scroll) = self.list_scroll.get(&(id, 10 + kind as u8)) {
                        scroll.scroll_to_item(index, ScrollStrategy::Center);
                    }
                    self.select_reference(reference.revision, reference.target, None, cx);
                }
            }
            ListFocus::History(project) if project == id => {
                let current = p
                    .snapshot
                    .commits
                    .iter()
                    .position(|c| Some(&c.id) == p.commit.as_ref());
                if let Some(index) =
                    text_selection::adjacent(current, p.snapshot.commits.len(), down)
                {
                    if current == Some(index) {
                        return;
                    }
                    let commit = p.snapshot.commits[index].id.clone();
                    if let Some(scroll) = self.history_scroll.get(&id) {
                        scroll.scroll_to_item(index, ScrollStrategy::Center);
                    }
                    self.select_commit(commit, cx);
                }
            }
            ListFocus::Files(project) if project == id => {
                let history = p.file_tab == FileTab::History;
                let offset = usize::from(history);
                let current = if history && p.show_commit_detail {
                    Some(0)
                } else {
                    p.selected_file.map(|index| index + offset)
                };
                let key = (
                    id,
                    if p.file_tab == FileTab::History {
                        21
                    } else {
                        20
                    },
                );
                if let Some(index) =
                    text_selection::adjacent(current, p.files().len() + offset, down)
                {
                    if current == Some(index) {
                        return;
                    }
                    if history && index == 0 {
                        self.show_detail();
                        cx.notify();
                        return;
                    }
                    let index = index - offset;
                    if let Some(row) = self
                        .file_items
                        .get(&key)
                        .and_then(|rows| rows.iter().position(|row| *row == FileRow::File(index)))
                        && let Some(scroll) = self.list_scroll.get(&key)
                    {
                        scroll.scroll_to_item(row, ScrollStrategy::Center);
                    }
                    if !history && let Some(project) = self.project_mut() {
                        project.stage_selection.click(
                            &project.snapshot.stage,
                            index,
                            extend,
                            false,
                        );
                    }
                    self.select_file(index, cx);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn remote_operation(
        &mut self,
        operation: crate::setup::RemoteOperation,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        let Some(p) = self.project() else {
            return;
        };
        let (id, path) = (p.id, p.path.clone());
        self.error = None;
        self.pending += 1;
        self.remote_status = Some((id, operation.label()));
        let task = cx
            .background_executor()
            .spawn(async move { operation.run(&path) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                this.remote_status = None;
                match result {
                    Ok(()) if this.tabs.iter().any(|tab| tab.id() == id) => {
                        let active = this.active;
                        this.active = Some(id);
                        this.reload(cx);
                        this.active = active;
                    }
                    Ok(()) => {}
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn open_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending > 0 {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        if project.loading || !project.snapshot.stage.iter().any(|file| file.staged) {
            return;
        }
        let project_id = project.id;
        let editor = cx.new(CommitEditor::new);
        editor.read(cx).focus.focus(window);
        self.commit_dialog = Some(CommitDialog {
            project_id,
            editor,
            busy: false,
            push_after: false,
            error: None,
        });
        cx.notify();
    }

    fn submit_commit(&mut self, push_after: bool, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.commit_dialog else {
            return;
        };
        if dialog.busy {
            return;
        }
        let message = dialog.editor.read(cx).message().to_owned();
        if message.trim().is_empty() {
            dialog.error = Some("커밋 메시지를 입력하세요.".into());
            cx.notify();
            return;
        }
        let id = dialog.project_id;
        if !self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|project| project.id == id)
            .is_some_and(|project| project.snapshot.stage.iter().any(|file| file.staged))
        {
            dialog.error = Some("staged 변경사항이 없습니다.".into());
            cx.notify();
            return;
        }
        let Some(path) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|project| project.id == id)
            .map(|project| project.path.clone())
        else {
            return;
        };
        dialog.busy = true;
        dialog.push_after = push_after;
        dialog.error = None;
        dialog.editor.update(cx, |editor, cx| {
            editor.enabled = false;
            cx.notify();
        });
        self.pending += 1;
        let task = cx
            .background_executor()
            .spawn(async move { crate::setup::commit_then_push(&path, &message, push_after) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                match result {
                    Ok(push_error) => {
                        this.commit_dialog = None;
                        this.error = push_error;
                        let active = this.active;
                        this.active = Some(id);
                        if let Some(project) = this.project_mut() {
                            project.commit = None;
                        }
                        this.reload(cx);
                        this.active = active;
                    }
                    Err(error) => {
                        if let Some(dialog) = &mut this.commit_dialog
                            && dialog.project_id == id
                        {
                            dialog.busy = false;
                            dialog.error = Some(format!("{error:#}"));
                            dialog.editor.update(cx, |editor, cx| {
                                editor.enabled = true;
                                cx.notify();
                            });
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn stage_selected(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.project() else {
            return;
        };
        if p.file_tab != FileTab::Stage
            || p.loading
            || self.pending > 0
            || !matches!(self.list_focus, Some(ListFocus::Files(id)) if id == p.id)
            || self.staging.contains(&p.id)
        {
            return;
        }
        let changes = p.stage_selection.changes(&p.snapshot.stage);
        if changes.is_empty() {
            return;
        }
        let (id, root) = (p.id, p.path.clone());
        self.staging.insert(id);
        self.pending += 1;
        self.error = None;
        let targets = changes.clone();
        let task = cx.background_executor().spawn(async move {
            let operation = crate::setup::toggle_files(&root, &changes);
            let refreshed = git::load_stage(&root);
            (operation, refreshed)
        });
        cx.spawn(async move |this, cx| {
            let (operation, refreshed) = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                this.staging.remove(&id);
                if !this.tabs.iter().any(|tab| tab.id() == id) {
                    return;
                }
                match refreshed {
                    Ok(files) => this.apply_stage_files(id, files, &targets, cx),
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                if let Err(error) = operation {
                    this.error = Some(format!("{error:#}"));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn apply_stage_files(
        &mut self,
        id: u64,
        files: Vec<FileChange>,
        targets: &std::collections::BTreeMap<Vec<u8>, bool>,
        cx: &mut Context<Self>,
    ) {
        let Some(p) = self
            .tabs
            .iter_mut()
            .filter_map(WorkspaceTab::project_mut)
            .find(|p| p.id == id)
        else {
            return;
        };
        let primary = if p.file_tab == FileTab::Stage {
            p.selected_file
                .and_then(|index| p.snapshot.stage.get(index))
                .map(|file| (file.path.clone(), file.staged))
        } else {
            None
        };
        p.snapshot.stage = files;
        p.error = None;
        p.stage_selection.restore(&p.snapshot.stage, targets);
        let selected = primary
            .and_then(|(path, staged)| {
                let staged = targets.get(&path).copied().unwrap_or(staged);
                p.snapshot.stage.iter().position(|file| {
                    file.path == path && file.staged == staged && p.stage_selection.contains(file)
                })
            })
            .or_else(|| {
                p.snapshot
                    .stage
                    .iter()
                    .position(|file| p.stage_selection.contains(file))
            });
        let show_stage = p.file_tab == FileTab::Stage;
        if show_stage {
            p.selected_file = selected;
        }
        self.file_items
            .insert((id, 20), file_rows(&p.snapshot.stage, false));
        if show_stage {
            if let Some(index) = selected {
                if let Some(scroll) = self.list_scroll.get(&(id, 20)) {
                    scroll.scroll_to_item(index, ScrollStrategy::Center);
                }
                self.select_project_file(id, index, cx);
            } else if let Some(p) = self
                .tabs
                .iter_mut()
                .filter_map(WorkspaceTab::project_mut)
                .find(|p| p.id == id)
            {
                p.diff_generation += 1;
                p.diff_loading = false;
                p.diff = Diff::default();
                if self.active == Some(id) {
                    self.text_selection = None;
                }
            }
        }
    }

    fn open_discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.project() else {
            return;
        };
        if self.pending > 0
            || p.loading
            || p.file_tab != FileTab::Stage
            || !matches!(self.list_focus, Some(ListFocus::Files(id)) if id == p.id)
        {
            return;
        }
        let paths: Vec<_> = p
            .stage_selection
            .changes(&p.snapshot.stage)
            .into_keys()
            .collect();
        if paths.is_empty() {
            return;
        }
        self.discard_dialog = Some(DiscardDialog {
            project_id: p.id,
            paths,
            busy: false,
            error: None,
        });
        self.focus.focus(window);
        cx.notify();
    }

    fn discard_selected(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.discard_dialog else {
            return;
        };
        if dialog.busy || self.pending > 0 {
            return;
        }
        let id = dialog.project_id;
        let Some(root) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|p| p.id == id)
            .map(|p| p.path.clone())
        else {
            return;
        };
        let paths = dialog.paths.clone();
        dialog.busy = true;
        dialog.error = None;
        self.pending += 1;
        let task = cx.background_executor().spawn(async move {
            let operation = crate::setup::discard_files(&root, &paths);
            let refreshed = git::load_stage(&root);
            (operation, refreshed)
        });
        cx.spawn(async move |this, cx| {
            let (operation, refreshed) = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                if !this.tabs.iter().any(|tab| tab.id() == id) {
                    this.discard_dialog = None;
                    return;
                }
                let mut error = operation.err().map(|error| format!("{error:#}"));
                match refreshed {
                    Ok(files) => this.apply_stage_files(id, files, &Default::default(), cx),
                    Err(failure) => {
                        error.get_or_insert_with(|| format!("{failure:#}"));
                    }
                }
                if let Some(error) = error {
                    if let Some(dialog) = &mut this.discard_dialog {
                        dialog.busy = false;
                        dialog.error = Some(error);
                    }
                } else {
                    this.discard_dialog = None;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn delete_repository(&mut self, cx: &mut Context<Self>) {
        if self.busy() || !self.delete_confirm {
            return;
        }
        let Some(path) = self.delete_target.take() else {
            return;
        };
        self.pending += 1;
        let mut sessions = Vec::new();
        for project in self.tabs.iter_mut().filter_map(WorkspaceTab::project_mut) {
            if project.path == path
                && let Some(session) = project.snapshot.session.take()
            {
                sessions.push(session);
            }
        }
        let target = path.clone();
        let task = cx.background_executor().spawn(async move {
            for session in sessions {
                let _ = session.close();
            }
            crate::setup::recycle_repository(&target)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                match result {
                    Ok(()) => {
                        let ids: Vec<_> = this
                            .tabs
                            .iter()
                            .filter_map(WorkspaceTab::project)
                            .filter(|p| p.path == path)
                            .map(|p| p.id)
                            .collect();
                        for id in ids {
                            this.close_tab(id, cx);
                        }
                        this.recent.retain(|item| *item != path);
                        this.error = None;
                        this.persist();
                    }
                    Err(error) => {
                        let ids: Vec<_> = this
                            .tabs
                            .iter()
                            .filter_map(WorkspaceTab::project)
                            .filter(|p| p.path == path)
                            .map(|p| p.id)
                            .collect();
                        let active = this.active;
                        for id in ids {
                            this.active = Some(id);
                            this.reload(cx);
                        }
                        this.active = active;
                        this.error = Some(format!("{error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn scroll_handle(&mut self, key: ListKey) -> UniformListScrollHandle {
        self.list_scroll.entry(key).or_default().clone()
    }

    fn drag_scroll(&self, drag: &ScrollDrag, position: Point<Pixels>) {
        let Some(bounds) = self.tracks.get(&(drag.key, drag.horizontal)) else {
            return;
        };
        let base = drag.handle.0.borrow().base_handle.clone();
        let max = base.max_offset();
        let mut offset = base.offset();
        let (track, maximum, local) = if drag.horizontal {
            (
                f32::from(bounds.size.width),
                f32::from(max.width),
                f32::from(position.x - bounds.origin.x),
            )
        } else {
            (
                f32::from(bounds.size.height),
                f32::from(max.height),
                f32::from(position.y - bounds.origin.y),
            )
        };
        let value = crate::scrolling::Thumb::new(track, maximum, 0.).offset(local - drag.grab);
        if drag.horizontal {
            offset.x = px(-value);
        } else {
            offset.y = px(-value);
        }
        base.set_offset(offset);
    }

    fn scroll_bar(
        &self,
        key: ListKey,
        handle: UniformListScrollHandle,
        horizontal: bool,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let view = cx.weak_entity();
        let paint_handle = handle.clone();
        div()
            .id((
                if horizontal {
                    "horizontal-scroll"
                } else {
                    "vertical-scroll"
                },
                key.1 as usize,
            ))
            .relative()
            .flex_shrink_0()
            .bg(rgb(BG))
            .cursor_pointer()
            .occlude()
            .when(horizontal, |bar| bar.h(px(SCROLLBAR)).w_full())
            .when(!horizontal, |bar| bar.w(px(SCROLLBAR)).h_full())
            .child(
                canvas(
                    move |bounds, _, cx| {
                        view.update(cx, |this, _| {
                            this.tracks.insert((key, horizontal), bounds);
                        })
                        .ok();
                    },
                    move |bounds, _, window, _| {
                        let base = paint_handle.0.borrow().base_handle.clone();
                        let maximum = base.max_offset();
                        let offset = base.offset();
                        let (track, max, pos) = if horizontal {
                            (bounds.size.width, maximum.width, -offset.x)
                        } else {
                            (bounds.size.height, maximum.height, -offset.y)
                        };
                        let thumb =
                            crate::scrolling::Thumb::new(track.into(), max.into(), pos.into());
                        let rect = if horizontal {
                            Bounds::new(
                                bounds.origin + point(px(thumb.start), px(2.)),
                                size(px(thumb.length), px(8.)),
                            )
                        } else {
                            Bounds::new(
                                bounds.origin + point(px(2.), px(thumb.start)),
                                size(px(8.), px(thumb.length)),
                            )
                        };
                        window.paint_quad(quad(
                            rect,
                            px(4.),
                            rgb(if thumb.maximum > 0. { MUTED } else { LINE }),
                            px(0.),
                            rgb(LINE),
                            Default::default(),
                        ));
                    },
                )
                .absolute()
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    let Some(bounds) = this.tracks.get(&(key, horizontal)) else {
                        return;
                    };
                    let base = handle.0.borrow().base_handle.clone();
                    let max = base.max_offset();
                    let offset = base.offset();
                    let (track, maximum, pos, local) = if horizontal {
                        (
                            bounds.size.width,
                            max.width,
                            -offset.x,
                            event.position.x - bounds.origin.x,
                        )
                    } else {
                        (
                            bounds.size.height,
                            max.height,
                            -offset.y,
                            event.position.y - bounds.origin.y,
                        )
                    };
                    let thumb =
                        crate::scrolling::Thumb::new(track.into(), maximum.into(), pos.into());
                    let local = f32::from(local);
                    let grab = if local >= thumb.start && local <= thumb.start + thumb.length {
                        local - thumb.start
                    } else {
                        thumb.length / 2.
                    };
                    let drag = ScrollDrag {
                        key,
                        handle: handle.clone(),
                        horizontal,
                        grab,
                    };
                    this.drag_scroll(&drag, event.position);
                    this.scroll_drag = Some(drag);
                    this.column_drag = None;
                    this.resize_drag = None;
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
    }

    fn scroll_visibility(
        &self,
        key: ListKey,
        content: Size<f32>,
        horizontal: bool,
    ) -> crate::scrolling::Visibility {
        let bounds = self.viewports.get(&key).copied().unwrap_or_default();
        crate::scrolling::Visibility::new(
            bounds.size.width.into(),
            bounds.size.height.into(),
            content.width,
            content.height,
            horizontal,
            SCROLLBAR,
        )
    }

    fn scroll_frame(
        &self,
        key: ListKey,
        handle: UniformListScrollHandle,
        list: impl IntoElement,
        content: Size<f32>,
        horizontal: bool,
        cx: &Context<Self>,
    ) -> Div {
        let visible = self.scroll_visibility(key, content, horizontal);
        let view = cx.weak_entity();
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .w_full()
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        view.update(cx, |this, cx| {
                            if this.viewports.get(&key) != Some(&bounds) {
                                this.viewports.insert(key, bounds);
                                cx.notify();
                            }
                            for axis in [false, true] {
                                if !(if axis {
                                    visible.horizontal
                                } else {
                                    visible.vertical
                                }) {
                                    this.tracks.remove(&(key, axis));
                                    if this.scroll_drag.as_ref().is_some_and(|drag| {
                                        drag.key == key && drag.horizontal == axis
                                    }) {
                                        this.scroll_drag = None;
                                    }
                                }
                            }
                        })
                        .ok();
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(list)
                    .when(visible.vertical, |row| {
                        row.child(self.scroll_bar(key, handle.clone(), false, cx))
                    }),
            )
            .when(visible.horizontal, |frame| {
                frame.child(
                    div()
                        .pr(px(if visible.vertical { SCROLLBAR } else { 0. }))
                        .flex_shrink_0()
                        .child(self.scroll_bar(key, handle, true, cx)),
                )
            })
    }

    fn load_more(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(p) = self
            .tabs
            .iter_mut()
            .filter_map(WorkspaceTab::project_mut)
            .find(|p| p.id == id)
        else {
            return;
        };
        if p.page_loading || !p.snapshot.has_more || p.page_error.is_some() {
            return;
        }
        let Some(session) = p.snapshot.session.clone() else {
            if p.history_target.take().is_some() {
                p.page_error =
                    Some("히스토리 조회 세션이 없습니다. 저장소를 다시 조회해 주세요.".into());
                cx.notify();
            }
            return;
        };
        p.page_loading = true;
        let generation = p.page_generation;
        let task = cx
            .background_executor()
            .spawn(async move { session.next_page() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                let Some(p) = this
                    .tabs
                    .iter_mut()
                    .filter_map(WorkspaceTab::project_mut)
                    .find(|p| p.id == id && p.page_generation == generation)
                else {
                    return;
                };
                p.page_loading = false;
                let mut target_index = None;
                match result {
                    Ok(page) => {
                        p.snapshot.commits.extend(page.commits);
                        p.snapshot.has_more = page.has_more;
                        target_index = p.resolve_history_target();
                    }
                    Err(error) => {
                        p.page_error = Some(format!("{error:#}"));
                    }
                }
                let continue_loading = p.history_target.is_some() && p.page_error.is_none();
                if let Some(index) = target_index
                    && this.active == Some(id)
                {
                    this.history_scroll
                        .entry(id)
                        .or_default()
                        .scroll_to_item(index, ScrollStrategy::Center);
                }
                if continue_loading {
                    this.load_more(id, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn project(&self) -> Option<&Project> {
        self.active
            .and_then(|id| self.tabs.iter().find(|tab| tab.id() == id))
            .and_then(WorkspaceTab::project)
    }

    fn open_branch_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project() else {
            return;
        };
        if self.pending > 0
            || self.branch_dialog.is_some()
            || self.list_focus != Some(ListFocus::History(project.id))
        {
            return;
        }
        let Some(commit) = project.commit.clone() else {
            return;
        };
        let (project_id, path) = (project.id, project.path.clone());
        let editor = cx.new(|cx| CommitEditor::single_line("", cx));
        editor.read(cx).focus.focus(window);
        self.branch_dialog = Some(BranchDialog {
            project_id,
            path,
            action: BranchAction::Create { commit },
            editor: Some(editor),
            error: None,
        });
        cx.notify();
    }

    fn close_branch_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.branch_busy {
            return;
        }
        self.branch_dialog = None;
        self.focus.focus(window);
        cx.notify();
    }

    fn refresh_after_branch(
        &mut self,
        id: u64,
        switched: bool,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !self.tabs.iter().any(|tab| tab.id() == id) {
            return;
        }
        let active = self.active;
        self.active = Some(id);
        if switched && let Some(project) = self.project_mut() {
            project.revision = None;
            project.commit = None;
            project.ref_tab = RefTab::Local;
            project.file_tab = FileTab::Stage;
            project.show_commit_detail = false;
            self.list_focus = Some(ListFocus::Branches(id));
        }
        self.reload(cx);
        self.active = active;
        self.error = error;
    }

    fn submit_branch(&mut self, cx: &mut Context<Self>) {
        if self.pending > 0 || self.branch_busy {
            return;
        }
        let Some(dialog) = &mut self.branch_dialog else {
            return;
        };
        if dialog
            .editor
            .as_ref()
            .is_some_and(|editor| editor.read(cx).is_composing())
        {
            return;
        }
        let name = dialog
            .editor
            .as_ref()
            .map(|editor| editor.read(cx).message().trim().to_owned());
        let (id, path, action) = (
            dialog.project_id,
            dialog.path.clone(),
            dialog.action.clone(),
        );
        if let Some(editor) = &dialog.editor {
            editor.update(cx, |editor, cx| {
                editor.enabled = false;
                cx.notify();
            });
        }
        dialog.error = None;
        self.pending += 1;
        self.branch_busy = true;
        self.remote_status = Some((id, "브랜치 작업 중…"));
        let switched = matches!(action, BranchAction::Track(_));
        let task = cx.background_executor().spawn(async move {
            match action {
                BranchAction::Create { commit } => {
                    crate::branches::create(&path, name.as_deref().unwrap_or_default(), &commit)
                }
                BranchAction::Track(target) => crate::branches::checkout(&path, &target, true),
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                this.branch_busy = false;
                this.remote_status = None;
                match result {
                    Ok(()) => {
                        this.branch_dialog = None;
                        this.refresh_after_branch(id, switched, None, cx);
                    }
                    Err(error) => {
                        this.refresh_after_branch(id, false, Some(error.to_string()), cx);
                        if let Some(dialog) = &mut this.branch_dialog {
                            dialog.error = Some(error.to_string());
                            if let Some(editor) = &dialog.editor {
                                editor.update(cx, |editor, cx| {
                                    editor.enabled = true;
                                    cx.notify();
                                });
                            }
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn checkout_reference(&mut self, id: u64, reference: String, cx: &mut Context<Self>) {
        if self.pending > 0 || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project().filter(|project| project.id == id) else {
            return;
        };
        let path = project.path.clone();
        if let Some(project) = self.project_mut() {
            project.history_target = None;
        }
        let worker_path = path.clone();
        self.pending += 1;
        self.branch_busy = true;
        self.remote_status = Some((id, "브랜치 전환 중…"));
        let task = cx.background_executor().spawn(async move {
            let target = crate::branches::prepare_checkout(&worker_path, &reference)?;
            if target.needs_creation {
                Ok(Some(target))
            } else {
                crate::branches::checkout(&worker_path, &target, false)?;
                Ok::<_, anyhow::Error>(None)
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                this.branch_busy = false;
                this.remote_status = None;
                match result {
                    Ok(Some(target)) => {
                        this.branch_dialog = Some(BranchDialog {
                            project_id: id,
                            path,
                            action: BranchAction::Track(target),
                            editor: None,
                            error: None,
                        })
                    }
                    Ok(None) => this.refresh_after_branch(id, true, None, cx),
                    Err(error) => this.refresh_after_branch(id, false, Some(error.to_string()), cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_in_progress()
            || self.commit_dialog.is_some()
            || self.discard_dialog.is_some()
            || self.delete_target.is_some()
            || self.settings.is_some()
            || self.branch_dialog.is_some()
            || self.branch_busy
        {
            return;
        }
        let path = self.project().map(|project| project.path.clone());
        self.help = false;
        self.settings = Some(cx.new(|cx| SettingsPanel::new(path, cx)));
        self.focus.focus(window);
        cx.notify();
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(settings) = &self.settings {
            if !settings.read(cx).can_close() {
                return;
            }
            settings.read(cx).cancel();
        }
        self.settings = None;
        self.focus.focus(window);
        cx.notify();
    }

    fn settings_link(&self, cx: &Context<Self>) -> Stateful<Div> {
        div()
            .id("show-settings")
            .ml_3()
            .text_xs()
            .text_color(rgb(ACCENT))
            .cursor_pointer()
            .child("f12 - 세팅")
            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
    }

    fn add_empty_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(WorkspaceTab::Empty(id));
        self.start_pages.insert(id, StartPage::default());
        self.active = Some(id);
        self.error = None;
        self.tabs_scroll.scroll_to_item(self.tabs.len() - 1);
        self.persist();
    }

    fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        self.list_scroll.retain(|(project, _), _| *project != id);
        self.tracks.retain(|((project, _), _), _| *project != id);
        self.file_items.retain(|(project, _), _| *project != id);
        self.detail_items.remove(&id);
        self.diff_width_row.remove(&id);
        self.diff_content_width.remove(&id);
        self.diff_measured.remove(&id);
        self.viewports.retain(|(project, _), _| *project != id);
        self.scroll_drag = None;
        self.graphs.remove(&id);
        self.history_scroll.remove(&id);
        self.start_pages.remove(&id);
        self.tabs.retain(|tab| tab.id() != id);
        if self.active == Some(id) {
            self.active = self.tabs.last().map(WorkspaceTab::id);
        }
        if self.tabs.is_empty() {
            self.add_empty_tab();
        }
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| Some(tab.id()) == self.active)
        {
            self.tabs_scroll.scroll_to_item(index);
        }
        self.list_focus = None;
        self.text_selection = None;
        self.persist();
        self.ensure_active_loaded(cx);
    }

    fn ensure_active_loaded(&mut self, cx: &mut Context<Self>) {
        if self.project().is_some_and(|project| !project.initialized) {
            self.reload(cx);
        }
    }

    fn start_resize(&mut self, axis: SplitAxis, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let bounds = self.body_bounds;
        let (position, origin, extent) = match axis {
            SplitAxis::Columns => (event.position.x, bounds.origin.x, bounds.size.width),
            SplitAxis::Rows => (event.position.y, bounds.origin.y, bounds.size.height),
        };
        let offset = position - origin - extent * self.layout.ratio(axis, extent.into());
        self.resize_drag = Some((axis, offset));
        cx.stop_propagation();
        cx.notify();
    }

    fn resize_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.history_in_progress()
            || self.help
            || self.settings.is_some()
            || self.branch_dialog.is_some()
            || self.branch_busy
            || self.delete_target.is_some()
            || self.commit_dialog.is_some()
            || self.discard_dialog.is_some()
        {
            self.resize_drag = None;
            self.column_drag = None;
            self.scroll_drag = None;
            return;
        }
        if let Some(drag) = self.scroll_drag.clone() {
            if event.dragging() {
                self.drag_scroll(&drag, event.position);
                cx.stop_propagation();
            } else {
                self.scroll_drag = None;
            }
            cx.notify();
            return;
        }
        if let Some((boundary, start, mut columns, width)) = self.column_drag {
            if event.dragging() {
                columns.resize(boundary, f32::from(event.position.x) - start, width);
                self.grid_columns = columns;
                cx.stop_propagation();
            } else {
                self.column_drag = None;
            }
            cx.notify();
            return;
        }
        let Some((axis, offset)) = self.resize_drag else {
            return;
        };
        if !event.dragging() {
            self.resize_drag = None;
        } else {
            let bounds = self.body_bounds;
            let (position, origin, extent) = match axis {
                SplitAxis::Columns => (event.position.x, bounds.origin.x, bounds.size.width),
                SplitAxis::Rows => (event.position.y, bounds.origin.y, bounds.size.height),
            };
            self.layout
                .resize(axis, (position - origin - offset).into(), extent.into());
            cx.stop_propagation();
        }
        cx.notify();
    }

    fn resize_end(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.button == MouseButton::Left && self.scroll_drag.take().is_some() {
            cx.stop_propagation();
            cx.notify();
        }
        if event.button == MouseButton::Left && self.column_drag.take().is_some() {
            cx.stop_propagation();
            cx.notify();
        }
        if event.button == MouseButton::Left && self.resize_drag.take().is_some() {
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn splitter(&self, axis: SplitAxis, ratio: f32, cx: &Context<Self>) -> Stateful<Div> {
        let (id, cursor) = match axis {
            SplitAxis::Columns => ("column-splitter", CursorStyle::ResizeLeftRight),
            SplitAxis::Rows => ("row-splitter", CursorStyle::ResizeUpDown),
        };
        let handle = div().id(id).absolute().cursor(cursor).occlude();
        let handle = match axis {
            SplitAxis::Columns => handle
                .left(relative(ratio))
                .top_0()
                .h_full()
                .w(px(7.))
                .ml(px(-3.))
                .child(
                    div()
                        .absolute()
                        .left(px(3.))
                        .top_0()
                        .w(px(1.))
                        .h_full()
                        .bg(rgb(LINE)),
                ),
            SplitAxis::Rows => handle
                .top(relative(ratio))
                .left_0()
                .w_full()
                .h(px(7.))
                .mt(px(-3.))
                .child(
                    div()
                        .absolute()
                        .top(px(3.))
                        .left_0()
                        .h(px(1.))
                        .w_full()
                        .bg(rgb(LINE)),
                ),
        };
        handle.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event, _, cx| {
                this.start_resize(axis, event, cx);
            }),
        )
    }

    fn project_mut(&mut self) -> Option<&mut Project> {
        let id = self.active?;
        self.tabs
            .iter_mut()
            .find(|tab| tab.id() == id)
            .and_then(WorkspaceTab::project_mut)
    }

    fn open(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(source) = self.active else { return };
        let Some(page) = self.start_pages.get_mut(&source) else {
            return;
        };
        if page.busy {
            return;
        }
        page.busy = true;
        page.error = None;
        self.pending += 1;
        // 정규화도 저장소 탐색과 함께 백그라운드에서 수행한다.
        let task = cx
            .background_executor()
            .spawn(async move { git::repository_root(&path) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                let Some(page) = this.start_pages.get_mut(&source) else {
                    cx.notify();
                    return;
                };
                page.busy = false;
                match result {
                    Ok(path) => {
                        this.finish_open(source, path, cx);
                    }
                    Err(err) => page.error = Some(format!("{err:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn finish_open(&mut self, source: u64, path: PathBuf, cx: &mut Context<Self>) {
        let Some(target) = WorkspaceTab::open_in_place(&mut self.tabs, source, path.clone()) else {
            return;
        };
        self.start_pages.remove(&source);
        self.viewports.remove(&(source, 40));
        self.tracks
            .retain(|((id, kind), _), _| *id != source || *kind != 40);
        if self
            .scroll_drag
            .as_ref()
            .is_some_and(|drag| drag.key == (source, 40))
        {
            self.scroll_drag = None;
        }
        let active = if self.active == Some(source) {
            self.list_focus = None;
            self.text_selection = None;
            Some(target)
        } else {
            self.active
        };
        session::remember(&mut self.recent, path);
        self.error = None;
        // 대상 탭의 조회를 시작하되 작업 중 바뀐 사용자 선택은 보존한다.
        self.active = Some(target);
        self.ensure_active_loaded(cx);
        self.active = active;
        self.persist();
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| Some(tab.id()) == self.active)
        {
            self.tabs_scroll.scroll_to_item(index);
        }
    }

    fn start_repository(&mut self, id: u64, mode: u8, cx: &mut Context<Self>) {
        let Some(page) = self.start_pages.get_mut(&id) else {
            return;
        };
        if page.busy {
            return;
        }
        let source = (mode == 2).then(|| page.url.trim().to_owned());
        if source.as_ref().is_some_and(|s| s.is_empty()) {
            page.error = Some("저장소 URL을 입력하세요.".into());
            cx.notify();
            return;
        }
        page.busy = true;
        page.error = None;
        let existing = (mode == 1).then(|| {
            cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some("저장소 열기".into()),
            })
        });
        let destination = (mode != 1).then(|| {
            cx.prompt_for_new_path(
                &std::env::current_dir().unwrap_or_default(),
                Some("repository"),
            )
        });
        cx.spawn(async move |this, cx| {
            let selected = if let Some(prompt) = existing {
                prompt
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|r| r)
                    .map(|paths| paths.and_then(|p| p.into_iter().next()))
            } else {
                destination
                    .unwrap()
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|r| r)
            };
            let result = match selected {
                Ok(Some(path)) => {
                    let alive = this
                        .update(cx, |this, _| this.start_pages.contains_key(&id))
                        .unwrap_or(false);
                    if !alive {
                        return;
                    }
                    Some(
                        cx.background_executor()
                            .spawn(async move {
                                if mode != 1 {
                                    crate::setup::create(&path, source.as_deref())?;
                                }
                                git::repository_root(&path)
                            })
                            .await,
                    )
                }
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            };
            this.update(cx, |this, cx| {
                let Some(page) = this.start_pages.get_mut(&id) else {
                    return;
                };
                page.busy = false;
                match result {
                    Some(Ok(path)) => {
                        this.finish_open(id, path, cx);
                    }
                    Some(Err(error)) => page.error = Some(format!("{error:#}")),
                    None => {}
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn start_page(&self, cx: &Context<Self>) -> Stateful<Div> {
        let Some(id) = self.active else {
            return div().id("start-page");
        };
        let Some(page) = self.start_pages.get(&id) else {
            return div().id("start-page");
        };
        let mut content = div()
            .id("start-actions")
            .flex()
            .flex_col()
            .gap_3()
            .w(px(300.))
            .max_w(relative(0.4))
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .flex_shrink_0()
            .child(div().text_lg().child("신규 저장소"));
        for (mode, label) in [(0, "신규 저장소"), (1, "저장소 열기"), (2, "Clone")] {
            content = content.child(button(("start", mode as usize), label, false).on_click(
                cx.listener(move |this, _, window, cx| {
                    if this.start_pages.get(&id).is_none_or(|p| p.busy) {
                        return;
                    }
                    if mode == 2 {
                        this.start_pages.get_mut(&id).unwrap().clone_form = true;
                        this.url_focus.focus(window);
                        cx.notify();
                    } else {
                        this.start_repository(id, mode, cx);
                    }
                }),
            ));
        }
        if page.clone_form {
            content = content
                .child(div().text_sm().child("저장소 URL"))
                .child(
                    div()
                        .id("clone-url")
                        .track_focus(&self.url_focus)
                        .px_3()
                        .py_2()
                        .border_1()
                        .border_color(rgb(if page.select_all { ACCENT } else { LINE }))
                        .overflow_hidden()
                        .text_sm()
                        .child(if page.url.is_empty() {
                            "https://… 또는 SSH URL".to_owned()
                        } else {
                            page.url.clone()
                        })
                        .on_click(cx.listener(|this, _, window, _| this.url_focus.focus(window)))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                            let Some(page) = this.start_pages.get_mut(&id) else {
                                return;
                            };
                            if page.busy {
                                return;
                            }
                            let key = &event.keystroke;
                            if key.modifiers.control && key.key == "a" {
                                page.select_all = true;
                            } else if key.modifiers.control && key.key == "c" {
                                if page.select_all {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        page.url.clone(),
                                    ));
                                }
                            } else if key.key == "enter" {
                                this.start_repository(id, 2, cx);
                                return;
                            } else {
                                let text = if key.modifiers.control && key.key == "v" {
                                    cx.read_from_clipboard().and_then(|item| item.text())
                                } else if !key.modifiers.control && !key.modifiers.alt {
                                    key.key_char.clone()
                                } else {
                                    None
                                };
                                if key.key == "backspace" || key.key == "delete" || text.is_some() {
                                    if page.select_all {
                                        page.url.clear();
                                        page.select_all = false;
                                    } else if key.key == "backspace" {
                                        page.url.pop();
                                    }
                                    if let Some(text) = text {
                                        page.url.push_str(text.trim_matches(['\r', '\n']));
                                    }
                                }
                            }
                            cx.stop_propagation();
                            cx.notify();
                        })),
                )
                .child(
                    button("execute-clone", "Clone 실행 · 저장 위치 선택", true).on_click(
                        cx.listener(move |this, _, _, cx| this.start_repository(id, 2, cx)),
                    ),
                );
        }
        if page.busy {
            content = content.child(empty("처리 중입니다…"));
        }
        if let Some(error) = &page.error {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(rgb(0xf4a0a8))
                    .child(error.clone()),
            );
        }
        let key = (id, 40);
        let scroll = page.recent_scroll.clone();
        let count = self.recent.len();
        let list = uniform_list(
            ("recent-list", id as usize),
            count,
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|index| this.recent_row(index, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(scroll.clone())
        .flex_1()
        .min_w_0()
        .w_full();
        let recent = div()
            .id("recent-repositories")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .pl_5()
            .border_l_1()
            .border_color(rgb(LINE))
            .child(div().text_lg().child("최근 저장소"))
            .when(count == 0, |panel| {
                panel.child(empty("아직 열었던 저장소가 없습니다."))
            })
            .child(self.scroll_frame(
                key,
                scroll,
                list,
                size(0., count as f32 * RECENT_ROW_HEIGHT),
                false,
                cx,
            ));
        div()
            .id("start-page")
            .flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .gap_5()
            .p_5()
            .child(content)
            .child(recent)
    }

    fn recent_row(&self, index: usize, cx: &Context<Self>) -> Stateful<Div> {
        let row = div()
            .id(("recent-row", index))
            .h(px(RECENT_ROW_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .min_w_0()
            .overflow_hidden()
            .border_b_1()
            .border_color(rgb(LINE));
        let Some(path) = self.recent.get(index) else {
            return row;
        };
        let open_path = path.clone();
        let delete_path = path.clone();
        row.cursor_pointer()
            .hover(|row| row.bg(rgb(0x263443)))
            .child(
                div()
                    .text_sm()
                    .max_w(relative(0.4))
                    .min_w_0()
                    .text_ellipsis()
                    .child(
                        path.file_name()
                            .unwrap_or(path.as_os_str())
                            .to_string_lossy()
                            .into_owned(),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .text_ellipsis()
                    .child(session::display_path(path)),
            )
            .child(
                div()
                    .id(("recent-delete", index))
                    .size(px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .hover(|icon| icon.bg(rgb(0x54313a)))
                    .child(trash_icon())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.delete_target = Some(delete_path.clone());
                        this.delete_confirm = false;
                        this.focus.focus(window);
                        cx.notify();
                    })),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.open(open_path.clone(), cx)))
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.text_selection = None;
        let Some(p) = self.project_mut() else { return };
        p.initialized = true;
        p.generation += 1;
        p.page_generation += 1;
        p.page_loading = false;
        p.page_error = None;
        p.history_target = None;
        p.history_loading = true;
        p.diff_generation += 1;
        p.diff_loading = false;
        let (id, generation) = (p.id, p.generation);
        let (path, commit) = (p.path.clone(), p.commit.clone());
        p.loading = true;
        p.error = None;
        p.selected_file = None;
        p.stage_selection.clear();
        p.diff = Diff::default();
        let task = cx
            .background_executor()
            .spawn(async move { git::load_initial(&path, commit.as_deref()) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                let Some(p) = this
                    .tabs
                    .iter_mut()
                    .filter_map(WorkspaceTab::project_mut)
                    .find(|p| p.id == id && p.generation == generation)
                else {
                    return;
                };
                p.loading = false;
                p.history_loading = false;
                match result {
                    Ok(snapshot) => {
                        if !snapshot
                            .commit_detail
                            .as_ref()
                            .is_some_and(|detail| Some(&detail.id) == p.commit.as_ref())
                            && !snapshot
                                .commits
                                .iter()
                                .any(|c| Some(&c.id) == p.commit.as_ref())
                        {
                            p.commit = snapshot.commits.first().map(|c| c.id.clone());
                        }
                        if !snapshot
                            .references
                            .iter()
                            .any(|r| Some(&r.revision) == p.revision.as_ref())
                        {
                            p.revision = None;
                        }
                        let stage = file_rows(&snapshot.stage, false);
                        let history = file_rows(&snapshot.history, false);
                        let details = snapshot
                            .commit_detail
                            .as_ref()
                            .map(detail_rows)
                            .unwrap_or_default();
                        p.snapshot = snapshot;
                        this.file_items.insert((id, 20), stage);
                        this.file_items.insert((id, 21), history);
                        this.detail_items.insert(id, TextRows::new(details));
                        this.graphs.remove(&id);
                        this.history_scroll.entry(id).or_default();
                    }
                    Err(err) => {
                        p.error = Some(format!("{err:#}"));
                        p.snapshot = Snapshot::default();
                        p.commit = None;
                        this.file_items.remove(&(id, 20));
                        this.file_items.remove(&(id, 21));
                        this.detail_items.remove(&id);
                        this.graphs.remove(&id);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn select_commit(&mut self, commit: String, cx: &mut Context<Self>) {
        self.text_selection = None;
        let Some(p) = self.project_mut() else { return };
        p.history_target = None;
        p.commit = Some(commit.clone());
        p.file_tab = FileTab::History;
        p.show_commit_detail = true;
        p.selected_file = None;
        p.diff_generation += 1;
        p.diff_loading = false;
        p.diff = Diff::default();
        p.generation += 1;
        p.loading = true;
        p.error = None;
        p.snapshot.history.clear();
        p.snapshot.commit_detail = None;
        let (id, generation, path) = (p.id, p.generation, p.path.clone());
        for kind in [21, 30, 31] {
            if let Some(scroll) = self.list_scroll.get(&(id, kind)) {
                scroll
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.), px(0.)));
            }
        }
        let task = cx
            .background_executor()
            .spawn(async move { git::load_commit(&path, &commit) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                let Some(p) = this
                    .tabs
                    .iter_mut()
                    .filter_map(WorkspaceTab::project_mut)
                    .find(|p| p.id == id && p.generation == generation)
                else {
                    return;
                };
                p.loading = false;
                match result {
                    Ok(selection) => {
                        this.file_items
                            .insert((id, 21), file_rows(&selection.history, false));
                        this.detail_items
                            .insert(id, TextRows::new(detail_rows(&selection.detail)));
                        p.snapshot.history = selection.history;
                        p.snapshot.commit_detail = Some(selection.detail);
                    }
                    Err(error) => p.error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn select_reference(
        &mut self,
        revision: String,
        target: String,
        paging_delay: Option<std::time::Duration>,
        cx: &mut Context<Self>,
    ) {
        let Some(p) = self.project_mut() else {
            return;
        };
        let id = p.id;
        p.revision = Some(revision);
        self.select_commit(target.clone(), cx);
        let Some(p) = self.project_mut() else {
            return;
        };
        p.history_target = Some(target.clone());
        p.page_error = None;
        let index = p.resolve_history_target();
        let needs_page = p.history_target.is_some();
        if let Some(index) = index {
            self.history_scroll
                .entry(id)
                .or_default()
                .scroll_to_item(index, ScrollStrategy::Center);
        } else if needs_page {
            if let Some(delay) = paging_delay {
                let generation = p.page_generation;
                // 추가 페이지 모달이 두 번째 클릭을 가로막지 않게 시스템 더블클릭 시간만 기다린다.
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(delay).await;
                    this.update(cx, |this, cx| {
                        let still_selected = this.project().is_some_and(|project| {
                            project.id == id
                                && project.page_generation == generation
                                && project.history_target.as_ref() == Some(&target)
                        });
                        if still_selected
                            && this.list_focus == Some(ListFocus::Branches(id))
                            && !this.branch_busy
                            && this.branch_dialog.is_none()
                        {
                            this.load_more(id, cx);
                        }
                    })
                    .ok();
                })
                .detach();
            } else {
                self.load_more(id, cx);
            }
        }
        cx.notify();
    }

    fn select_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(id) = self.active else {
            return;
        };
        self.select_project_file(id, index, cx);
    }

    fn select_project_file(&mut self, project_id: u64, index: usize, cx: &mut Context<Self>) {
        if self.active == Some(project_id) {
            self.text_selection = None;
        }
        let Some(p) = self
            .tabs
            .iter_mut()
            .filter_map(WorkspaceTab::project_mut)
            .find(|p| p.id == project_id)
        else {
            return;
        };
        if p.loading {
            return;
        }
        let Some(file) = p.files().get(index).cloned() else {
            return;
        };
        p.selected_file = Some(index);
        p.show_commit_detail = false;
        p.diff_generation += 1;
        let (id, generation, path) = (p.id, p.diff_generation, p.path.clone());
        p.diff = Diff::default();
        p.diff_loading = true;
        p.error = None;
        if let Some(scroll) = self.list_scroll.get(&(id, 30)) {
            scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(0.)));
        }
        let task = cx
            .background_executor()
            .spawn(async move { git::diff(&path, &file) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                let Some(p) = this
                    .tabs
                    .iter_mut()
                    .filter_map(WorkspaceTab::project_mut)
                    .find(|p| p.id == id && p.diff_generation == generation)
                else {
                    return;
                };
                p.diff_loading = false;
                match result {
                    Ok(diff) => {
                        this.diff_measured.remove(&id);
                        this.diff_width_row.insert(
                            id,
                            diff.lines
                                .iter()
                                .enumerate()
                                .max_by_key(|(_, l)| l.text.len())
                                .map(|(i, _)| i)
                                .unwrap_or(0),
                        );
                        let columns = diff
                            .lines
                            .iter()
                            .map(|line| text_columns(&line.text))
                            .max()
                            .unwrap_or(0);
                        this.diff_content_width
                            .insert(id, columns as f32 * 8. + 132.);
                        p.diff = diff;
                    }
                    Err(err) => p.error = Some(format!("{err:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn show_detail(&mut self) {
        self.text_selection = None;
        if let Some(p) = self.project_mut() {
            p.show_commit_detail = true;
            if p.snapshot.commit_detail.is_some() {
                p.error = None;
            }
            p.selected_file = None;
            p.diff_generation += 1;
            p.diff_loading = false;
            p.diff = Diff::default();
        }
    }

    fn branch_panel(&mut self, cx: &Context<Self>) -> Div {
        let mut tabs = div()
            .id("reference-tabs")
            .flex()
            .gap_1()
            .px_3()
            .py_2()
            .flex_shrink_0()
            .min_w_0()
            .overflow_x_scroll();
        for (i, tab) in RefTab::ALL.into_iter().enumerate() {
            tabs = tabs.child(
                button(
                    ("ref-tab", i),
                    tab.label(),
                    self.project().is_some_and(|p| p.ref_tab == tab),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(p) = this.project_mut() {
                        p.ref_tab = tab;
                    }
                    cx.notify();
                })),
            );
        }
        let (id, kind, count) = self
            .project()
            .map(|p| {
                (
                    p.id,
                    p.ref_tab.index(),
                    p.snapshot.reference_rows[p.ref_tab.index()].len(),
                )
            })
            .unwrap_or_default();
        let key = (id, 10 + kind as u8);
        let scroll = self.scroll_handle(key);
        let list = uniform_list(
            ("branches", kind),
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                range
                    .map(|index| this.branch_row(id, kind, index, window, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(scroll.clone())
        .flex_1()
        .min_h_0()
        .min_w_0()
        .w_full();
        panel()
            .child(tabs)
            .when(count == 0, |p| p.child(empty("표시할 항목이 없습니다.")))
            .child(self.scroll_frame(
                key,
                scroll,
                list,
                size(0., count as f32 * history_view::ROW_HEIGHT),
                false,
                cx,
            ))
    }

    fn branch_row(
        &self,
        id: u64,
        kind: usize,
        index: usize,
        window: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let mut row = div()
            .id(("reference", index))
            .h(px(history_view::ROW_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .px_3()
            .min_w_0()
            .overflow_hidden()
            .text_sm();
        let Some(p) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|p| p.id == id)
        else {
            return row;
        };
        let Some(reference) = p.snapshot.reference_rows[kind]
            .get(index)
            .and_then(|i| p.snapshot.references.get(*i))
        else {
            return row;
        };
        let current = p.snapshot.current_branch.as_deref() == Some(reference.revision.as_str());
        let active = p.revision.as_deref() == Some(reference.revision.as_str())
            || (p.revision.is_none() && current);
        let revision = reference.revision.clone();
        let target = reference.target.clone();
        row = row
            .bg(rgb(if active {
                self.selection_color(ListFocus::Branches(id), window)
            } else {
                PANEL
            }))
            .text_color(rgb(if current { ACCENT } else { TEXT }))
            .cursor_pointer()
            .when(!active, |row| row.hover(|s| s.bg(rgb(0x263443))))
            .child(div().min_w_0().text_ellipsis().child(format!(
                "{}{}",
                if current { "● " } else { "⑂ " },
                reference.name
            )));
        row.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            if this.active != Some(id) {
                return;
            }
            this.list_focus = Some(ListFocus::Branches(id));
            this.focus.focus(window);
            if event.click_count() == 2 && kind < 2 {
                this.checkout_reference(id, revision.clone(), cx);
            } else if event.click_count() == 1 {
                this.select_reference(
                    revision.clone(),
                    target.clone(),
                    Some(branch_click_interval()),
                    cx,
                );
            }
        }))
    }

    fn history_row(
        &mut self,
        project_id: u64,
        index: usize,
        widths: [f32; 5],
        window: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let mut row = div()
            .id(("commit", index))
            .flex()
            .w_full()
            .h(px(history_view::ROW_HEIGHT))
            .flex_shrink_0()
            .cursor_pointer();
        let Some(p) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|p| p.id == project_id)
        else {
            return row;
        };
        let Some(commit) = p.snapshot.commits.get(index) else {
            return row;
        };
        let id = commit.id.clone();
        let selected = p.commit.as_deref() == Some(&id);
        let selection_color = self.selection_color(ListFocus::History(project_id), window);
        let graph = self
            .graphs
            .entry(project_id)
            .or_default()
            .entry(index)
            .or_insert_with(|| RowGraph::new(commit));
        row = row
            .bg(rgb(if selected { selection_color } else { PANEL }))
            .when(!selected, |row| row.hover(|s| s.bg(rgb(0x202e3c))))
            .child(history_view::cell(0, widths[0]).child(graph.render()));
        let mut description = div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .overflow_hidden();
        for branch in &commit.branches {
            description = description.child(
                div()
                    .flex_shrink_0()
                    .px_1()
                    .rounded_sm()
                    .bg(rgb(SELECT))
                    .text_color(rgb(ACCENT))
                    .child(format!(
                        "{}{}",
                        if p.snapshot.current_branch.as_deref()
                            == Some(format!("refs/heads/{branch}").as_str())
                        {
                            "● "
                        } else {
                            ""
                        },
                        branch
                    )),
            );
        }
        for tag in &commit.tags {
            description = description.child(
                div()
                    .flex_shrink_0()
                    .px_1()
                    .rounded_sm()
                    .bg(rgb(0x362b49))
                    .text_color(rgb(0xd6b4ff))
                    .child(format!("◇ {tag}")),
            );
        }
        description =
            description.child(div().min_w_0().text_ellipsis().child(commit.title.clone()));
        row = row.child(history_view::cell(1, widths[1]).child(description));
        for (column, text) in [
            (2, commit.author_label()),
            (
                3,
                history_view::format_time(commit.time, commit.time_offset),
            ),
            (4, commit.id.chars().take(8).collect()),
        ] {
            row = row.child(
                history_view::cell(column, widths[column])
                    .when(column >= 3, |cell| cell.font_family("Cascadia Mono"))
                    .child(div().min_w_0().text_ellipsis().child(text)),
            );
        }
        row.on_click(cx.listener(move |this, _, window, cx| {
            if this.active == Some(project_id) {
                this.list_focus = Some(ListFocus::History(project_id));
                this.focus.focus(window);
                this.select_commit(id.clone(), cx);
            }
        }))
    }

    fn history_panel(&mut self, cx: &Context<Self>) -> Div {
        let (project_id, count) = self
            .project()
            .map(|p| (p.id, p.snapshot.commits.len()))
            .unwrap_or_default();
        let bar_visible = self
            .scroll_visibility(
                (project_id, 0),
                size(0., count as f32 * history_view::ROW_HEIGHT),
                false,
            )
            .vertical;
        let widths = self
            .grid_columns
            .widths((self.grid_width - if bar_visible { SCROLLBAR } else { 0. }).max(0.));
        let mut headings = div()
            .pr(px(if bar_visible { SCROLLBAR } else { 0. }))
            .flex()
            .w_full()
            .h(px(history_view::ROW_HEIGHT))
            .flex_shrink_0()
            .border_b_1()
            .border_color(rgb(LINE))
            .bg(rgb(BG));
        for (i, label) in history_view::COLUMNS.iter().enumerate() {
            headings = headings.child(
                history_view::cell(i, widths[i])
                    .relative()
                    .text_color(rgb(MUTED))
                    .child(*label)
                    .when(i < 4, |cell| {
                        cell.child(
                            div()
                                .id(("column-resize", i))
                                .absolute()
                                .right_0()
                                .top_0()
                                .w(px(7.))
                                .h_full()
                                .cursor(CursorStyle::ResizeLeftRight)
                                .occlude()
                                .hover(|style| style.bg(rgb(ACCENT)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                        this.resize_drag = None;
                                        this.column_drag = Some((
                                            i,
                                            event.position.x.into(),
                                            this.grid_columns,
                                            (this.grid_width
                                                - if bar_visible { SCROLLBAR } else { 0. })
                                            .max(0.),
                                        ));
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                ),
                        )
                    }),
            );
        }
        let scroll = self.history_scroll.entry(project_id).or_default().clone();
        let view = cx.weak_entity();
        let bottom_handle = scroll.clone();
        let list = uniform_list(
            ("commits", project_id as usize),
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                this.graphs
                    .entry(project_id)
                    .or_default()
                    .retain(|index, _| range.contains(index));
                let base = bottom_handle.0.borrow().base_handle.clone();
                if range.end >= count
                    && count > 0
                    && f32::from(base.max_offset().height + base.offset().y) <= 1.
                {
                    let view = cx.weak_entity();
                    cx.defer(move |cx| {
                        view.update(cx, |this, cx| this.load_more(project_id, cx))
                            .ok();
                    });
                }
                range
                    .map(|index| this.history_row(project_id, index, widths, window, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(scroll.clone())
        .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::FitList)
        .w_full()
        .flex_1()
        .min_h_0();
        let page_loading = self.project().is_some_and(|p| p.page_loading);
        let page_error = self.project().and_then(|p| p.page_error.clone());
        panel()
            .when_some(page_error, |p, error| {
                p.child(
                    button("retry-page", format!("{error} · 다시 시도"), false).on_click(
                        cx.listener(move |this, _, _, cx| {
                            if let Some(p) = this.project_mut() {
                                p.page_error = None;
                            }
                            this.load_more(project_id, cx);
                        }),
                    ),
                )
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                view.update(cx, |this, cx| {
                                    let width = f32::from(bounds.size.width);
                                    if (this.grid_width - width).abs() > 0.1 {
                                        this.grid_width = width;
                                        this.column_drag = None;
                                        cx.notify();
                                    }
                                })
                                .ok();
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(headings)
                    .when(count == 0, |grid| {
                        grid.child(empty("커밋이 없습니다. 초기 저장소일 수 있습니다."))
                    })
                    .child(self.scroll_frame(
                        (project_id, 0),
                        scroll,
                        list,
                        size(0., count as f32 * history_view::ROW_HEIGHT),
                        false,
                        cx,
                    ))
                    .when(page_loading, |grid| grid.child(page_progress())),
            )
    }

    fn files_panel(&mut self, window: &Window, cx: &Context<Self>) -> Div {
        let mut tabs = div().flex().gap_1().px_3().py_2().flex_shrink_0();
        for (i, tab) in [FileTab::Stage, FileTab::History].into_iter().enumerate() {
            tabs = tabs.child(
                button(
                    ("file-tab", i),
                    if tab == FileTab::Stage {
                        "Stage"
                    } else {
                        "History"
                    },
                    self.project().is_some_and(|p| p.file_tab == tab),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.text_selection = None;
                    if let Some(p) = this.project_mut() {
                        p.file_tab = tab;
                        p.stage_selection.clear();
                        p.show_commit_detail = tab == FileTab::History;
                        if tab == FileTab::History && p.snapshot.commit_detail.is_some() {
                            p.error = None;
                        }
                        p.selected_file = None;
                        p.diff = Diff::default();
                        p.diff_generation += 1;
                        p.diff_loading = false;
                    }
                    cx.notify();
                })),
            );
        }
        let (id, history, detail) = self
            .project()
            .map(|p| (p.id, p.file_tab == FileTab::History, p.show_commit_detail))
            .unwrap_or_default();
        let key = (id, if history { 21 } else { 20 });
        let count = if history
            && self
                .project()
                .is_some_and(|p| p.loading || p.snapshot.history.is_empty())
        {
            0
        } else {
            self.file_items.get(&key).map_or(0, Vec::len)
        };
        let loading = self.project().is_some_and(|p| p.loading);
        let failed = self.project().is_some_and(|p| p.error.is_some());
        let scroll = self.scroll_handle(key);
        let list = uniform_list(
            ("files", key.1 as usize),
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                range
                    .map(|index| this.file_row(key, index, window, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(scroll.clone())
        .w_full()
        .min_w_0()
        .flex_1()
        .min_h_0();
        panel()
            .child(tabs)
            .when(history, |panel| {
                panel.child(
                    div()
                        .id("commit-detail")
                        .bg(rgb(if detail {
                            self.selection_color(ListFocus::Files(id), window)
                        } else {
                            PANEL
                        }))
                        .text_color(rgb(TEXT))
                        .cursor_pointer()
                        .when(!detail, |row| row.hover(|s| s.bg(rgb(0x263443))))
                        .text_sm()
                        .h(px(history_view::ROW_HEIGHT))
                        .w_full()
                        .px_3()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .rounded_none()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .child("커밋상세")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.list_focus = Some(ListFocus::Files(id));
                            this.focus.focus(window);
                            this.show_detail();
                            cx.notify();
                        })),
                )
            })
            .when(count == 0, |panel| {
                panel.child(empty(if loading {
                    "조회 중입니다…"
                } else if failed {
                    "파일 목록을 조회하지 못했습니다."
                } else if history {
                    "선택한 커밋에 변경 파일이 없습니다."
                } else {
                    "작업 디렉터리가 깨끗합니다."
                }))
            })
            .child(self.scroll_frame(
                key,
                scroll,
                list,
                size(0., count as f32 * history_view::ROW_HEIGHT),
                false,
                cx,
            ))
    }

    fn file_row(
        &self,
        key: ListKey,
        index: usize,
        window: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let mut row = div()
            .id(("file-row", index))
            .h(px(history_view::ROW_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .min_w_0()
            .overflow_hidden()
            .text_sm();
        let Some(item) = self.file_items.get(&key).and_then(|rows| rows.get(index)) else {
            return row;
        };
        if let FileRow::Group(_) = item {
            return row
                .text_xs()
                .text_color(rgb(MUTED))
                .child("STAGED · 스테이징");
        }
        let FileRow::File(file_index) = *item else {
            unreachable!()
        };
        let Some(p) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|p| p.id == key.0)
        else {
            return row;
        };
        let files = if key.1 == 20 {
            &p.snapshot.stage
        } else {
            &p.snapshot.history
        };
        let Some(file) = files.get(file_index) else {
            return row;
        };
        let selected = (key.1 == 20 && p.stage_selection.contains(file))
            || (key.1 != 20 && p.selected_file == Some(file_index));
        row = row
            .bg(rgb(if selected {
                self.selection_color(ListFocus::Files(key.0), window)
            } else {
                PANEL
            }))
            .cursor_pointer()
            .when(!selected, |row| row.hover(|s| s.bg(rgb(0x263443))))
            .child(
                div()
                    .text_xs()
                    .flex_shrink_0()
                    .when(key.1 == 20, |status| {
                        status
                            .w(px(22.))
                            .font_family("Cascadia Mono")
                            .whitespace_nowrap()
                    })
                    .text_color(rgb(if key.1 == 20 {
                        if file.staged { 0x80d9a4 } else { 0xf28d95 }
                    } else {
                        match file.status.as_str() {
                            "A" => 0x80d9a4,
                            "D" => 0xf28d95,
                            _ => 0xe8c783,
                        }
                    }))
                    .child(if key.1 == 20 {
                        file.stage_status()
                    } else {
                        file.status.clone()
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .text_ellipsis()
                    .when(key.1 == 20 && file.staged, |name| {
                        name.text_color(rgb(0x80d9a4))
                    })
                    .child(file.label.clone()),
            );
        row.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            if this.active == Some(key.0) {
                this.list_focus = Some(ListFocus::Files(key.0));
                this.focus.focus(window);
                if key.1 == 20 {
                    let modifiers = event.modifiers();
                    let selected = this.project_mut().and_then(|project| {
                        project.stage_selection.click(
                            &project.snapshot.stage,
                            file_index,
                            modifiers.shift,
                            modifiers.control,
                        )
                    });
                    if let Some(index) = selected {
                        this.select_file(index, cx);
                    } else if let Some(project) = this.project_mut() {
                        project.selected_file = None;
                        project.diff_generation += 1;
                        project.diff_loading = false;
                        project.diff = Diff::default();
                    }
                } else {
                    this.select_file(file_index, cx);
                }
                cx.notify();
            }
        }))
    }

    fn diff_panel(&mut self, window: &mut Window, cx: &Context<Self>) -> Div {
        let Some(p) = self.project() else {
            return panel().child(empty("파일을 선택하세요."));
        };
        let id = p.id;
        let details = p.file_tab == FileTab::History && p.show_commit_detail;
        let key = (id, if details { 31 } else { 30 });
        let loading = p.loading || p.diff_loading;
        let error = p.error.is_some();
        let count = if loading || error {
            0
        } else if details {
            self.detail_items.get(&id).map_or(0, |r| r.lines.len())
        } else {
            p.diff.lines.len()
        };
        let widest = if details {
            self.detail_items.get(&id).map_or(0, |r| r.widest)
        } else {
            self.diff_width_row.get(&id).copied().unwrap_or(0)
        };
        let title = if details {
            "커밋상세".to_owned()
        } else {
            p.diff.title.clone()
        };
        let note = if details {
            String::new()
        } else {
            p.diff.note.clone()
        };
        if details {
            if let Some(rows) = self.detail_items.get_mut(&id)
                && !rows.measured
            {
                rows.width = measure_text_width(window, rows.lines.iter().map(String::as_str), 24.);
                rows.measured = true;
            }
        } else if !self.diff_measured.contains(&id) && !loading && !error {
            let width = measure_text_width(
                window,
                self.project()
                    .unwrap()
                    .diff
                    .lines
                    .iter()
                    .map(|line| line.text.as_str()),
                132.,
            );
            self.diff_content_width.insert(id, width);
            self.diff_measured.insert(id);
        }
        let content_width = if details {
            self.detail_items.get(&id).map_or(24., |r| r.width)
        } else {
            self.diff_content_width.get(&id).copied().unwrap_or(132.)
        };
        let scroll = self.scroll_handle(key);
        let list = uniform_list(
            ("diff-lines", key.1 as usize),
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|index| this.diff_row(id, index, details, content_width, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(scroll.clone())
        .with_width_from_item(Some(widest))
        .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
        .flex_1()
        .min_h_0()
        .min_w_0()
        .font_family("Cascadia Mono")
        .text_xs();
        panel()
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .flex_shrink_0()
                    .child(title),
            )
            .when(count == 0, |p| {
                p.child(empty(if loading {
                    "조회 중입니다…"
                } else if error {
                    "조회하지 못했습니다. 오류 내용을 확인하세요."
                } else if details {
                    "표시할 커밋이 없습니다."
                } else {
                    "파일을 선택해 변경 내용을 확인하세요."
                }))
            })
            .child(self.scroll_frame(
                key,
                scroll,
                list,
                size(
                    if count > 0 { content_width } else { 0. },
                    count as f32 * 22.,
                ),
                true,
                cx,
            ))
            .when(!note.is_empty(), |p| {
                p.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .flex_shrink_0()
                        .child(note),
                )
            })
    }

    fn diff_row(
        &self,
        id: u64,
        index: usize,
        details: bool,
        width: f32,
        cx: &Context<Self>,
    ) -> Div {
        let row = div()
            .flex()
            .w(px(width))
            .h(px(22.))
            .min_w_full()
            .whitespace_nowrap();
        if details {
            let text = self
                .detail_items
                .get(&id)
                .and_then(|r| r.lines.get(index))
                .cloned()
                .unwrap_or_default();
            return row
                .px_3()
                .child(self.selectable_line((id, 31), index, text, TEXT, cx));
        }
        let Some(line) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|p| p.id == id)
            .and_then(|p| p.diff.lines.get(index))
        else {
            return row;
        };
        let (bg, fg) = match line.kind {
            '+' => (0x16352d, 0x98e7b4),
            '-' => (0x38232b, 0xf4a0a8),
            '@' => (0x1b2c3d, 0x82bfe4),
            _ => (PANEL, TEXT),
        };
        row.bg(rgb(bg))
            .text_color(rgb(fg))
            .child(
                div()
                    .w(px(48.))
                    .flex_shrink_0()
                    .px_2()
                    .text_color(rgb(MUTED))
                    .child(line.old.map(|v| v.to_string()).unwrap_or_default()),
            )
            .child(
                div()
                    .w(px(48.))
                    .flex_shrink_0()
                    .px_2()
                    .text_color(rgb(MUTED))
                    .child(line.new.map(|v| v.to_string()).unwrap_or_default()),
            )
            .child(div().w_5().flex_shrink_0().child(line.kind.to_string()))
            .child(self.selectable_line((id, 30), index, line.text.clone(), fg, cx))
    }

    fn selectable_line(
        &self,
        key: ListKey,
        row: usize,
        raw: String,
        color: u32,
        cx: &Context<Self>,
    ) -> Div {
        let selection = self
            .text_selection
            .as_ref()
            .filter(|(selected, _)| *selected == key)
            .map(|(_, selection)| selection.clone());
        let focused = matches!(self.list_focus, Some(ListFocus::Text(selected)) if selected == key);
        let view = cx.weak_entity();
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .cursor(CursorStyle::IBeam)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let displayed = raw.replace('\t', "    ");
                        let run = TextRun {
                            len: displayed.len(),
                            font: font("Cascadia Mono"),
                            color: rgb(color).into(),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        let shaped = window.text_system().shape_line(
                            displayed.into(),
                            window.rem_size() * 0.75,
                            &[run],
                            None,
                        );
                        let x_for_column =
                            |column: usize| shaped.x_for_index(displayed_byte(&raw, column));
                        if let Some(selection) = &selection {
                            let (start, end) = selection.range();
                            if row >= start.row && row <= end.row && start != end {
                                let from = if row == start.row { start.column } else { 0 };
                                let to = if row == end.row {
                                    end.column
                                } else {
                                    raw.chars().count()
                                };
                                let left = x_for_column(from);
                                let right =
                                    x_for_column(to) + if row < end.row { px(7.) } else { px(0.) };
                                window.paint_quad(fill(
                                    Bounds::new(
                                        bounds.origin + point(left, px(0.)),
                                        size((right - left).max(px(0.)), px(22.)),
                                    ),
                                    rgb(0x345b78),
                                ));
                            }
                            if focused && selection.cursor.row == row {
                                window.paint_quad(fill(
                                    Bounds::new(
                                        bounds.origin
                                            + point(x_for_column(selection.cursor.column), px(1.)),
                                        size(px(1.), px(19.)),
                                    ),
                                    rgb(ACCENT),
                                ));
                            }
                        }
                        shaped.paint(bounds.origin, px(22.), window, cx).ok();
                        let down_view = view.clone();
                        let down_line = shaped.clone();
                        let down_raw = raw.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture
                                || event.button != MouseButton::Left
                                || !bounds.contains(&event.position)
                            {
                                return;
                            }
                            down_view
                                .update(cx, |this, cx| {
                                    if this.history_in_progress()
                                        || this.help
                                        || this.commit_dialog.is_some()
                                        || this.discard_dialog.is_some()
                                        || this.delete_target.is_some()
                                        || this.active != Some(key.0)
                                        || this.tracks.iter().any(|((track_key, _), track)| {
                                            *track_key == key && track.contains(&event.position)
                                        })
                                        || this.viewports.get(&key).is_none_or(|viewport| {
                                            !viewport.contains(&event.position)
                                        })
                                    {
                                        return;
                                    }
                                    let byte = down_line
                                        .closest_index_for_x(event.position.x - bounds.origin.x);
                                    let position = Position {
                                        row,
                                        column: original_column(&down_raw, byte),
                                    };
                                    if event.modifiers.shift
                                        && let Some((selected_key, selection)) =
                                            &mut this.text_selection
                                        && *selected_key == key
                                    {
                                        selection.cursor = position;
                                    } else {
                                        this.text_selection = Some((
                                            key,
                                            Selection {
                                                anchor: position,
                                                cursor: position,
                                            },
                                        ));
                                    }
                                    this.list_focus = Some(ListFocus::Text(key));
                                    this.text_drag = true;
                                    this.focus.focus(window);
                                    cx.notify();
                                })
                                .ok();
                        });
                        let move_view = view.clone();
                        let move_line = shaped.clone();
                        let move_raw = raw.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase != DispatchPhase::Capture || !bounds.contains(&event.position)
                            {
                                return;
                            }
                            move_view
                                .update(cx, |this, cx| {
                                    if !this.text_drag
                                        || this.history_in_progress()
                                        || this.help
                                        || this.commit_dialog.is_some()
                                        || this.discard_dialog.is_some()
                                        || this.delete_target.is_some()
                                        || this.viewports.get(&key).is_none_or(|viewport| {
                                            !viewport.contains(&event.position)
                                        })
                                    {
                                        return;
                                    }
                                    if event.pressed_button != Some(MouseButton::Left) {
                                        this.text_drag = false;
                                        return;
                                    }
                                    if let Some((selected_key, selection)) =
                                        &mut this.text_selection
                                        && *selected_key == key
                                    {
                                        selection.cursor = Position {
                                            row,
                                            column: original_column(
                                                &move_raw,
                                                move_line.closest_index_for_x(
                                                    event.position.x - bounds.origin.x,
                                                ),
                                            ),
                                        };
                                        cx.notify();
                                    }
                                })
                                .ok();
                        });
                        let up_view = view.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                up_view.update(cx, |this, _| this.text_drag = false).ok();
                            }
                        });
                    },
                )
                .size_full(),
            )
    }
}

fn displayed_byte(raw: &str, column: usize) -> usize {
    raw.chars()
        .take(column)
        .map(|character| {
            if character == '\t' {
                4
            } else {
                character.len_utf8()
            }
        })
        .sum()
}

fn original_column(raw: &str, byte: usize) -> usize {
    let mut offset = 0;
    for (column, character) in raw.chars().enumerate() {
        let next = offset
            + if character == '\t' {
                4
            } else {
                character.len_utf8()
            };
        if byte < next {
            return column + usize::from(byte - offset > (next - offset) / 2);
        }
        offset = next;
    }
    raw.chars().count()
}

fn page_progress() -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded_md()
                .bg(rgb(BG))
                .border_1()
                .border_color(rgb(LINE))
                .child(div().text_sm().child("커밋을 더 읽는 중입니다…"))
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(3.))
                        .overflow_hidden()
                        .bg(rgb(LINE))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .h_full()
                                .w(relative(0.3))
                                .bg(rgb(ACCENT))
                                .with_animation(
                                    "commit-page-progress",
                                    Animation::new(std::time::Duration::from_millis(900)).repeat(),
                                    |bar, progress| bar.left(relative(progress * 1.3 - 0.3)),
                                ),
                        ),
                ),
        )
}

fn measure_text_width<'a>(
    window: &Window,
    lines: impl Iterator<Item = &'a str>,
    padding: f32,
) -> f32 {
    lines
        .map(|line| {
            let text = line.replace('\t', "    ");
            let len = text.len();
            let run = TextRun {
                len,
                font: font("Cascadia Mono"),
                color: rgb(TEXT).into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            f32::from(
                window
                    .text_system()
                    .shape_line(text.into(), window.rem_size() * 0.75, &[run], None)
                    .x_for_index(len),
            )
        })
        .fold(0., f32::max)
        + padding
}

fn text_columns(text: &str) -> usize {
    text.chars()
        .map(|c| {
            if c == '\t' {
                4
            } else if c.is_ascii() {
                1
            } else {
                2
            }
        })
        .sum()
}

struct TextRows {
    measured: bool,
    lines: Vec<String>,
    widest: usize,
    width: f32,
}
impl TextRows {
    fn new(lines: Vec<String>) -> Self {
        let widest = lines
            .iter()
            .enumerate()
            .max_by_key(|(_, s)| s.len())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let width = lines
            .iter()
            .map(|line| text_columns(line))
            .max()
            .unwrap_or(0) as f32
            * 8.
            + 24.;
        Self {
            measured: false,
            lines,
            widest,
            width,
        }
    }
}
fn detail_rows(detail: &CommitDetail) -> Vec<String> {
    let mut rows = vec![format!("커밋: {}", detail.id)];
    if detail.parent_ids.is_empty() {
        rows.push("상위 항목: 없음 · 초기 커밋".into());
    } else {
        rows.extend(
            detail
                .parent_ids
                .iter()
                .map(|id| format!("상위 항목: {id}")),
        );
    }
    rows.extend([
        format!("작성자: {} <{}>", detail.author, detail.author_email),
        format!(
            "작성 시간: {}",
            format_detail_time(detail.author_time, detail.author_offset)
        ),
        format!("커미터: {} <{}>", detail.committer, detail.committer_email),
        format!(
            "커밋 시간: {}",
            format_detail_time(detail.commit_time, detail.commit_offset)
        ),
        String::new(),
    ]);
    rows.extend(
        detail
            .message
            .split('\n')
            .map(|line| line.trim_end_matches('\r').to_owned()),
    );
    rows
}

fn format_detail_time(seconds: i64, offset: i32) -> String {
    gix::date::Time::new(seconds, offset)
        .format(gix::date::time::format::ISO8601)
        .map(|time| time.chars().take(19).collect::<String>().replace('T', " "))
        .unwrap_or_else(|_| seconds.to_string())
}

impl Workspace {
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut projects = div()
            .id("project-tabs")
            .flex()
            .h_full()
            .flex_1()
            .min_w_0()
            .overflow_x_scroll()
            .track_scroll(&self.tabs_scroll);
        for tab in &self.tabs {
            let id = tab.id();
            let active = self.active == Some(id);
            let name = tab
                .project()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "새 탭".into());
            projects = projects.child(
                div()
                    .id(("project", id as usize))
                    .relative()
                    .flex()
                    .items_center()
                    .h_full()
                    .w(px(200.))
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(rgb(LINE))
                    .cursor_pointer()
                    .text_sm()
                    .bg(rgb(if active { PANEL } else { BG }))
                    .text_color(rgb(if active { TEXT } else { MUTED }))
                    .hover(|style| style.bg(rgb(0x202b38)))
                    .when(active, |tab| {
                        tab.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .bg(rgb(ACCENT)),
                        )
                    })
                    .child(div().flex_1().min_w_0().px_3().text_ellipsis().child(name))
                    .child(
                        div()
                            .id(("close", id as usize))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .mr_2()
                            .rounded_sm()
                            .flex_shrink_0()
                            .text_color(rgb(MUTED))
                            .hover(|style| style.bg(rgb(0x344153)).text_color(rgb(TEXT)))
                            .child("×")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_tab(id, cx);
                                cx.notify();
                            })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.active = Some(id);
                        this.ensure_active_loaded(cx);
                        this.list_focus = None;
                        this.text_selection = None;
                        this.persist();
                        cx.notify();
                    })),
            );
        }
        let top = div()
            .flex()
            .h(px(TAB_BAR_HEIGHT))
            .flex_shrink_0()
            .bg(rgb(BG))
            .border_b_1()
            .border_color(rgb(LINE))
            .child(projects)
            .child(
                div()
                    .id("add-tab")
                    .flex()
                    .items_center()
                    .justify_center()
                    .h_full()
                    .w(px(TAB_BAR_HEIGHT))
                    .flex_shrink_0()
                    .border_l_1()
                    .border_color(rgb(LINE))
                    .cursor_pointer()
                    .text_lg()
                    .text_color(rgb(MUTED))
                    .hover(|style| style.bg(rgb(PANEL)).text_color(rgb(ACCENT)))
                    .child("+")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.add_empty_tab();
                        cx.notify();
                    })),
            );
        if self.project().is_none() {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(PANEL))
                .text_color(rgb(TEXT))
                .font_family("Segoe UI")
                .child(top)
                .child(self.start_page(cx))
                .when_some(self.status_error.clone(), |page, error| {
                    page.child(
                        div()
                            .px_4()
                            .py_2()
                            .text_sm()
                            .text_color(rgb(0xf4a0a8))
                            .child(error),
                    )
                })
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .h_8()
                        .flex_shrink_0()
                        .px_4()
                        .child(
                            div()
                                .id("start-help")
                                .text_xs()
                                .text_color(rgb(ACCENT))
                                .cursor_pointer()
                                .child("? - 도움말")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.help = true;
                                    this.focus.focus(window);
                                    cx.notify();
                                })),
                        )
                        .child(self.settings_link(cx)),
                );
        }
        let branch = self.branch_panel(cx);
        let history = self.history_panel(cx);
        let files = self.files_panel(window, cx);
        let diff = self.diff_panel(window, cx);
        let columns = self
            .layout
            .ratio(SplitAxis::Columns, self.body_bounds.size.width.into());
        let rows = self
            .layout
            .ratio(SplitAxis::Rows, self.body_bounds.size.height.into());
        let measure_view = cx.weak_entity();
        let events_view = cx.weak_entity();
        let body = div()
            .id("workspace-body")
            .relative()
            .flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        measure_view
                            .update(cx, |this, cx| {
                                if this.body_bounds != bounds {
                                    this.body_bounds = bounds;
                                    cx.notify();
                                }
                            })
                            .ok();
                    },
                    move |_, _, window, _| {
                        // 전역 캡처로 경계선 밖에서도 이동과 마우스 해제를 처리한다.
                        let move_view = events_view.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase == DispatchPhase::Capture {
                                move_view
                                    .update(cx, |this, cx| this.resize_move(event, window, cx))
                                    .ok();
                            }
                        });
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                            if phase == DispatchPhase::Capture {
                                events_view
                                    .update(cx, |this, cx| this.resize_end(event, window, cx))
                                    .ok();
                            }
                        });
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(relative(columns))
                    .h_full()
                    .flex_shrink_0()
                    .min_w_0()
                    .min_h_0()
                    .child(
                        div()
                            .h(relative(rows))
                            .flex_shrink_0()
                            .min_h_0()
                            .overflow_hidden()
                            .child(branch),
                    )
                    .child(div().flex_1().min_h_0().overflow_hidden().child(files)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .min_h_0()
                    .child(
                        div()
                            .h(relative(rows))
                            .flex_shrink_0()
                            .min_h_0()
                            .overflow_hidden()
                            .child(history),
                    )
                    .child(div().flex_1().min_h_0().overflow_hidden().child(diff)),
            )
            .child(self.splitter(SplitAxis::Columns, columns, cx))
            .child(self.splitter(SplitAxis::Rows, rows, cx))
            .when_some(self.resize_drag, |body, (axis, _)| {
                body.child(
                    div()
                        .id("resize-overlay")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .occlude()
                        .cursor(match axis {
                            SplitAxis::Columns => CursorStyle::ResizeLeftRight,
                            SplitAxis::Rows => CursorStyle::ResizeUpDown,
                        }),
                )
            });
        let status = if let Some(error) = &self.status_error {
            error.clone()
        } else if let Some(project) = self.project() {
            let loading =
                if let Some((_, label)) = self.remote_status.filter(|(id, _)| *id == project.id) {
                    format!("  ·  {label}")
                } else if project.loading || project.diff_loading {
                    "  ·  조회 중…".into()
                } else {
                    String::new()
                };
            format!(
                "{}  ·  {}{}",
                session::display_path(&project.path),
                project.snapshot.head,
                loading
            )
        } else {
            String::new()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .font_family("Segoe UI")
            .child(top)
            .child(body)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .h_8()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_ellipsis()
                            .text_color(rgb(if self.status_error.is_some() {
                                0xf4a0a8
                            } else {
                                MUTED
                            }))
                            .child(status),
                    )
                    .child(
                        div()
                            .ml_3()
                            .text_xs()
                            .text_color(rgb(ACCENT))
                            .cursor_pointer()
                            .child("? - 도움말")
                            .id("show-help")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.help = true;
                                this.focus.focus(window);
                                cx.notify();
                            })),
                    )
                    .child(self.settings_link(cx)),
            )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_status_error(cx);
        if window.focused(cx).is_none() {
            self.focus.focus(window);
        }
        let content = self.render_content(window, cx);
        let busy = self.history_in_progress() || self.branch_busy;
        let mut root = div()
            .id("workspace")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .child(content)
            .capture_key_down(
                cx.listener(|this, event, window, cx| this.key_down(event, window, cx)),
            )
            .capture_any_mouse_down(cx.listener(|this, _, _, cx| {
                if this.history_in_progress() || this.branch_busy {
                    cx.stop_propagation();
                }
            }));
        if busy
            || self.help
            || self.settings.is_some()
            || self.branch_dialog.is_some()
            || self.delete_target.is_some()
            || self.commit_dialog.is_some()
            || self.discard_dialog.is_some()
        {
            let mut card = div()
                .id("modal-card")
                .w(px(500.))
                .max_w_full()
                .p_5()
                .flex()
                .flex_col()
                .gap_3()
                .bg(rgb(PANEL))
                .border_1()
                .border_color(rgb(LINE))
                .rounded_lg()
                .on_click(|_, _, cx| cx.stop_propagation());
            if busy {
                let message = if self.branch_busy {
                    "브랜치 작업 중입니다…"
                } else if self
                    .tabs
                    .iter()
                    .filter_map(WorkspaceTab::project)
                    .any(|p| p.history_target.is_some())
                {
                    "선택한 브랜치의 커밋을 찾는 중입니다…"
                } else {
                    "커밋 히스토리를 읽는 중입니다…"
                };
                card = card
                    .child(div().text_lg().child("처리 중"))
                    .child(message)
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(4.))
                            .overflow_hidden()
                            .bg(rgb(LINE))
                            .child(
                                div()
                                    .absolute()
                                    .h_full()
                                    .w(relative(0.3))
                                    .bg(rgb(ACCENT))
                                    .with_animation(
                                        "workspace-progress",
                                        Animation::new(std::time::Duration::from_millis(900))
                                            .repeat(),
                                        |bar, progress| bar.left(relative(progress * 1.3 - 0.3)),
                                    ),
                            ),
                    );
            } else if let Some(dialog) = &self.branch_dialog {
                card = card.child(div().text_lg().child(match &dialog.action {
                    BranchAction::Create { .. } => "브랜치 만들기",
                    BranchAction::Track(_) => "로컬 브랜치 만들기",
                }));
                match &dialog.action {
                    BranchAction::Create { commit } => {
                        card = card
                            .child(format!("선택 커밋: {}", &commit[..commit.len().min(12)]))
                            .child("브랜치명")
                            .when_some(dialog.editor.clone(), |card, editor| {
                                card.child(
                                    div()
                                        .h(px(36.))
                                        .border_1()
                                        .border_color(rgb(LINE))
                                        .p_1()
                                        .child(editor),
                                )
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child("브랜치만 생성하고 현재 체크아웃은 유지합니다."),
                            );
                    }
                    BranchAction::Track(target) => {
                        card = card
                            .child(format!(
                                "같은 이름의 Local 브랜치 '{}'가 없습니다.",
                                target.local
                            ))
                            .child(format!(
                                "{} 브랜치를 추적하는 Local 브랜치를 만들고 체크아웃할까요?",
                                target.remote.as_deref().unwrap_or_default()
                            ));
                    }
                }
                card = card
                    .when_some(dialog.error.clone(), |card, error| {
                        card.child(div().text_color(rgb(0xf4a0a8)).child(error))
                    })
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("confirm-branch", "확인 · enter", true)
                                    .on_click(cx.listener(|this, _, _, cx| this.submit_branch(cx))),
                            )
                            .child(button("cancel-branch", "취소 · esc", false).on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.close_branch_dialog(window, cx)
                                }),
                            )),
                    );
            } else if let Some(settings) = &self.settings {
                card = card
                    .w(px(760.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_lg().child("세팅"))
                            .child(
                                button("close-settings", "닫기 · esc", false)
                                    .when(!settings.read(cx).can_close(), |button| {
                                        button.opacity(0.45)
                                    })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_settings(window, cx)
                                    })),
                            ),
                    )
                    .child(settings.clone());
            } else if let Some(dialog) = &self.discard_dialog {
                card = card.w(px(640.)).child(div().text_sm().child("선택한 변경사항 폐기"))
                    .child(format!("{}개 파일의 staged·unstaged 변경을 HEAD로 복원합니다. 신규 파일은 삭제됩니다.",dialog.paths.len()))
                    .when_some(dialog.error.clone(),|card,error| card.child(div().text_color(rgb(0xf4a0a8)).child(error)))
                    .when(dialog.busy,|card| card.child("변경사항을 폐기하는 중입니다…"))
                    .child(div().text_xs().text_color(rgb(MUTED)).child("폐기 : ctrl + s, 취소 : esc"));
            } else if let Some(dialog) = &self.commit_dialog {
                card = card
                    .w(px(640.))
                    .child(div().text_sm().child("커밋"))
                    .child(dialog.editor.clone())
                    .when_some(dialog.error.clone(), |card, error| {
                        card.child(div().text_color(rgb(0xf4a0a8)).child(error))
                    })
                    .when(dialog.busy, |card| {
                        card.child(if dialog.push_after {
                            "커밋 후 푸시하는 중입니다…"
                        } else {
                            "커밋하는 중입니다…"
                        })
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("커밋 : ctrl + s, 커밋 후 푸시 : ctrl + p, 취소 : esc"),
                    );
            } else if let Some(path) = &self.delete_target
                && !self.delete_confirm
            {
                card = card
                    .child(div().text_lg().child("최근 저장소 관리"))
                    .child(session::display_path(path))
                    .child("목록에서만 제거하거나 저장소 폴더를 삭제할 수 있습니다.")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(button("forget-recent", "목록에서 제거", false).on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(path) = this.delete_target.take() {
                                        this.recent.retain(|item| *item != path);
                                        this.persist();
                                    }
                                    cx.notify();
                                }),
                            ))
                            .child(button("choose-delete", "저장소 삭제…", false).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.delete_confirm = true;
                                    cx.notify();
                                }),
                            ))
                            .child(button("cancel-recent-action", "취소", false).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.delete_target = None;
                                    cx.notify();
                                }),
                            )),
                    );
            } else if let Some(path) = &self.delete_target {
                card = card
                    .child(div().text_lg().child("저장소 삭제"))
                    .child(session::display_path(path))
                    .child("미커밋 파일을 포함한 저장소 폴더 전체를 휴지통으로 이동합니다.")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(button("cancel-delete", "취소", false).on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.delete_target = None;
                                    cx.notify();
                                },
                            )))
                            .child(button("confirm-delete", "휴지통으로 이동", true).on_click(
                                cx.listener(|this, _, _, cx| this.delete_repository(cx)),
                            )),
                    );
            } else {
                card = card.child(shortcut_help(window));
            }
            root = root.child(
                div()
                    .id("modal-backdrop")
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .occlude()
                    .bg(rgba(0x000000a0))
                    .text_color(rgb(TEXT))
                    .font_family("Segoe UI")
                    .text_sm()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_mouse_move(|_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.branch_busy {
                            cx.stop_propagation();
                            return;
                        }
                        if this.branch_dialog.is_some() {
                            this.close_branch_dialog(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if this.settings.is_some() {
                            this.close_settings(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                        if !this.history_in_progress()
                            && !this
                                .commit_dialog
                                .as_ref()
                                .is_some_and(|dialog| dialog.busy)
                            && !this
                                .discard_dialog
                                .as_ref()
                                .is_some_and(|dialog| dialog.busy)
                        {
                            this.help = false;
                            this.delete_target = None;
                            this.commit_dialog = None;
                            this.discard_dialog = None;
                            this.focus.focus(window);
                            cx.notify();
                        }
                        cx.stop_propagation();
                    }))
                    .child(card),
            );
        }
        root
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn opening_repository_replaces_only_the_origin_empty_tab_in_place() {
        let path = PathBuf::from("repository");
        let mut tabs = vec![
            WorkspaceTab::Empty(1),
            WorkspaceTab::Empty(2),
            WorkspaceTab::Empty(3),
        ];
        assert_eq!(
            WorkspaceTab::open_in_place(&mut tabs, 2, path.clone()),
            Some(2)
        );
        assert_eq!(
            tabs.iter().map(WorkspaceTab::id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(matches!(tabs[0], WorkspaceTab::Empty(1)));
        assert!(matches!(tabs[2], WorkspaceTab::Empty(3)));
        assert_eq!(tabs[1].project().unwrap().path, path);
    }

    #[::core::prelude::v1::test]
    fn reopening_repository_consumes_only_the_origin_and_reuses_the_existing_tab() {
        let path = PathBuf::from("repository");
        let mut tabs = vec![
            WorkspaceTab::Empty(1),
            WorkspaceTab::Repository(Box::new(Project::new(2, path.clone()))),
            WorkspaceTab::Empty(3),
        ];
        assert_eq!(WorkspaceTab::open_in_place(&mut tabs, 1, path), Some(2));
        assert_eq!(
            tabs.iter().map(WorkspaceTab::id).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(matches!(tabs[1], WorkspaceTab::Empty(3)));
    }

    #[::core::prelude::v1::test]
    fn opening_from_a_closed_or_already_replaced_tab_does_not_add_a_tab() {
        let path = PathBuf::from("repository");
        let mut tabs = vec![
            WorkspaceTab::Empty(1),
            WorkspaceTab::Repository(Box::new(Project::new(2, path.clone()))),
        ];
        assert_eq!(
            WorkspaceTab::open_in_place(&mut tabs, 3, path.clone()),
            None
        );
        assert_eq!(
            WorkspaceTab::open_in_place(&mut tabs, 2, PathBuf::from("other")),
            None
        );
        assert_eq!(tabs.len(), 2);
        assert!(matches!(tabs[0], WorkspaceTab::Empty(1)));
        assert_eq!(tabs[1].project().unwrap().path, path);
    }

    fn detail(parents: &[&str]) -> CommitDetail {
        CommitDetail {
            id: "commit-id".into(),
            parent_ids: parents.iter().map(|id| (*id).into()).collect(),
            author: "작성자".into(),
            author_email: "author@example.invalid".into(),
            author_time: 0,
            author_offset: 9 * 3600,
            committer: "커미터".into(),
            committer_email: "committer@example.invalid".into(),
            commit_time: 0,
            commit_offset: -4 * 3600,
            message: "제목\n\n본문\n".into(),
        }
    }

    #[::core::prelude::v1::test]
    fn merge_parent_labels_stay_with_hashes_and_message_keeps_blank_lines() {
        let rows = detail_rows(&detail(&["first-parent", "second-parent"]));
        assert_eq!(rows[1], "상위 항목: first-parent");
        assert_eq!(rows[2], "상위 항목: second-parent");
        assert!(
            !rows
                .iter()
                .any(|line| line == "부모:" || line == "상위 항목:" || line == "커밋 메시지")
        );
        assert!(rows.contains(&"작성 시간: 1970-01-01 09:00:00".into()));
        assert!(rows.contains(&"커밋 시간: 1969-12-31 20:00:00".into()));
        assert_eq!(&rows[rows.len() - 4..], ["제목", "", "본문", ""]);
    }

    #[::core::prelude::v1::test]
    fn root_parent_explanation_is_inline_and_time_keeps_local_value_without_zone() {
        let rows = detail_rows(&detail(&[]));
        assert_eq!(rows[1], "상위 항목: 없음 · 초기 커밋");
        assert_eq!(format_detail_time(3661, 0), "1970-01-01 01:01:01");
        assert_eq!(format_detail_time(0, 9 * 3600), "1970-01-01 09:00:00");
        assert_eq!(format_detail_time(0, -4 * 3600), "1969-12-31 20:00:00");
    }
}
