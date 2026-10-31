use crate::commit_editor::CommitEditor;
use crate::history_view::{self, ColumnLayout, RowGraph};
use crate::layout::{SplitAxis, SplitLayout};
use crate::settings::SettingsPanel;
use crate::theme::*;
use crate::{
    session::{self, Session},
    text_selection::{self, Position, Selection},
};
use feather_git::{git, model::*};
use gpui::{prelude::*, *};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const SEARCH_VIRTUAL_ROW: usize = usize::MAX;

#[derive(Debug, PartialEq, Eq)]
enum SearchArrowTarget {
    Input,
    Row(usize),
}

fn search_arrow_target(selected: usize, count: usize, down: bool) -> SearchArrowTarget {
    if selected == SEARCH_VIRTUAL_ROW {
        return if down {
            SearchArrowTarget::Input
        } else {
            SearchArrowTarget::Row(SEARCH_VIRTUAL_ROW)
        };
    }
    if down {
        SearchArrowTarget::Row(if count == 0 {
            SEARCH_VIRTUAL_ROW
        } else {
            (selected + 1).min(count - 1)
        })
    } else if selected == 0 {
        SearchArrowTarget::Input
    } else {
        SearchArrowTarget::Row(selected - 1)
    }
}

#[derive(Default)]
struct StartPage {
    url_editor: Option<Entity<CommitEditor>>,
    clone_form: bool,
    busy: bool,
    error: Option<String>,
    recent_scroll: UniformListScrollHandle,
}

struct CommitDialog {
    project_id: u64,
    editor: Entity<CommitEditor>,
    busy: bool,
    push_after: bool,
    error: Option<String>,
}

struct StashDialog {
    project_id: u64,
    editor: Option<Entity<CommitEditor>>,
    deletion: Option<(String, String, String)>,
    busy: bool,
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
    Create {
        commit: String,
        remote: bool,
        checkout: bool,
    },
    Tag {
        commit: String,
    },
    Delete {
        plan: crate::branches::DeletePlan,
        delete_related: bool,
    },
    Connect {
        plan: crate::branches::UpstreamPlan,
        selected: usize,
    },
    ChooseCheckout {
        commit: String,
        candidates: Vec<crate::branches::CheckoutCandidate>,
        selected: usize,
    },
    Reset {
        commit: String,
        mode: crate::branches::ResetMode,
    },
    Revert {
        commit: String,
        no_commit: bool,
    },
    Integrate {
        kind: crate::branches::IntegrateKind,
        commit: String,
    },
    MergeControl {
        state: MergeState,
        action: crate::branches::MergeControl,
    },
    RebaseControl {
        state: RebaseState,
        action: crate::branches::RebaseControl,
    },
    RevertControl {
        state: RevertState,
        action: crate::branches::RevertControl,
        no_commit: bool,
    },
    StashControl {
        state: StashRestoreState,
        cancel: bool,
    },
    Track(crate::branches::CheckoutTarget),
}

struct BranchDialog {
    project_id: u64,
    path: PathBuf,
    action: BranchAction,
    editor: Option<Entity<CommitEditor>>,
    error: Option<String>,
}

const TAB_BAR_HEIGHT: f32 = 44.;
const SCROLLBAR: f32 = 12.;
const RECENT_ROW_HEIGHT: f32 = 40.;

fn navigation_key(key: &str) -> &str {
    match key {
        "i" | "I" => "up",
        "j" | "J" => "left",
        "k" | "K" => "down",
        "l" | "L" => "right",
        _ => key,
    }
}

fn adjacent_tab(current: usize, count: usize, next: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let current = current % count;
    Some(if next {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    })
}

fn adjacent_reference_tab(tab: RefTab, search: bool, next: bool) -> usize {
    let current = if search {
        RefTab::ALL.len()
    } else {
        tab.index()
    };
    adjacent_tab(current, RefTab::ALL.len() + 1, next).unwrap()
}

fn candidate_shortcut(key: &str) -> Option<usize> {
    match key {
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
            key.parse::<usize>().ok().map(|n| n - 1)
        }
        "0" => Some(9),
        _ => None,
    }
}

type ListKey = (u64, u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListFocus {
    Branches(u64),
    Files(u64),
    History(u64),
    Text(ListKey),
}

fn section_focus(key: &str, id: u64, details: bool) -> Option<ListFocus> {
    match key {
        "1" => Some(ListFocus::Branches(id)),
        "2" => Some(ListFocus::History(id)),
        "3" => Some(ListFocus::Files(id)),
        "4" => Some(ListFocus::Text((id, if details { 31 } else { 30 }))),
        _ => None,
    }
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
    file_window: Option<AnyWindowHandle>,
    file_window_activation: Option<Subscription>,
    search_tab: bool,
    search_editor: Entity<CommitEditor>,
    search_results: Vec<feather_git::git_search::Match>,
    search_selected: usize,
    search_loading: bool,
    search_generation: u64,
    state_path: Option<PathBuf>,
    popup_options: session::PopupOptions,
    recent: Vec<PathBuf>,
    pending: usize,
    staging: HashSet<u64>,
    remote_status: Option<(u64, &'static str)>,
    help: bool,
    shortcut_settings: bool,
    shortcut_tab: usize,
    shortcut_draft: Option<crate::shortcut::Bindings>,
    shortcuts: crate::shortcut::Bindings,
    shortcut_capture: Option<(String, usize)>,
    shortcut_message: Option<String>,
    settings: Option<Entity<SettingsPanel>>,
    branch_dialog: Option<BranchDialog>,
    branch_busy: bool,
    focus_after_popup: bool,
    commit_dialog: Option<CommitDialog>,
    discard_dialog: Option<DiscardDialog>,
    stash_dialog: Option<StashDialog>,
    delete_target: Option<PathBuf>,
    delete_confirm: bool,
    focus: FocusHandle,
    branch_checks_focus: [FocusHandle; 2],
    commit_push_focus: FocusHandle,
    reset_mode_focus: FocusHandle,
    upstream_scroll: ScrollHandle,
    upstream_focus: FocusHandle,
    list_focus: Option<ListFocus>,
    text_selection: Option<(ListKey, Selection)>,
    text_drag: bool,
    tabs: Vec<WorkspaceTab>,
    tabs_scroll: ScrollHandle,
    active: Option<u64>,
    next_id: u64,
    error: Option<String>,
    layout: SplitLayout,
    error_history: crate::error_history::ErrorHistory,
    error_history_open: bool,
    error_editor: Option<(usize, String, Entity<CommitEditor>)>,
    body_bounds: Bounds<Pixels>,
    resize_drag: Option<(SplitAxis, Pixels)>,
    start_pages: HashMap<u64, StartPage>,
    start_action_focus: [FocusHandle; 4],
    recent_action_focus: HashMap<PathBuf, [FocusHandle; 2]>,
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
    file_tree: bool,
    selected_folder: Option<(ListKey, Vec<u8>)>,
    detail_items: HashMap<u64, TextRows>,
    diff_width_row: HashMap<u64, usize>,
    diff_content_width: HashMap<u64, f32>,
    partial_selection: HashMap<u64, (u64, std::collections::BTreeSet<usize>)>,
}

fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, active: bool) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .bg(rgb(if active { SELECT } else { PANEL }))
        .text_color(rgb(if active { ACCENT } else { MUTED }))
        .hover(|s| s.bg(rgb(HOVER)).text_color(rgb(TEXT)))
        .when(!label.is_empty(), |button| {
            button.child(crate::shortcut::label(label.as_ref()))
        })
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

fn diff_column_guides(partial: bool, horizontal_offset: Pixels) -> Div {
    div()
        .absolute()
        .left(horizontal_offset)
        .top_0()
        .h_full()
        .flex()
        .when(partial, |guide| {
            guide.child(
                div()
                    .w(px(24.))
                    .h_full()
                    .flex_shrink_0()
                    .bg(rgb(HISTORY_HOVER)),
            )
        })
        .child(
            div()
                .w(px(96.))
                .h_full()
                .flex_shrink_0()
                .bg(rgb(HISTORY_HOVER))
                .border_r_1()
                .border_color(rgb(LINE)),
        )
        .child(
            div()
                .w(px(20.))
                .h_full()
                .flex_shrink_0()
                .border_r_1()
                .border_color(rgb(LINE)),
        )
}

fn empty(message: &str) -> Div {
    div()
        .p_5()
        .text_sm()
        .text_color(rgb(MUTED))
        .child(message.to_owned())
}

fn current_branch_dot(slot: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let center = bounds.origin + point(bounds.size.width / 2., bounds.size.height / 2.);
            window.paint_quad(quad(
                Bounds::new(center - point(px(5.), px(5.)), size(px(10.), px(10.))),
                px(5.),
                rgb(ACCENT),
                px(0.),
                rgb(ACCENT),
                Default::default(),
            ));
        },
    )
    .size(px(slot))
    .flex_shrink_0()
}

fn commit_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let center = bounds.origin + point(px(6.), px(6.));
            let mut path = PathBuilder::stroke(px(1.2));
            path.move_to(bounds.origin + point(px(0.), px(6.)));
            path.line_to(bounds.origin + point(px(3.), px(6.)));
            path.move_to(bounds.origin + point(px(9.), px(6.)));
            path.line_to(bounds.origin + point(px(12.), px(6.)));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(MUTED));
            }
            window.paint_quad(quad(
                Bounds::new(center - point(px(3.), px(3.)), size(px(6.), px(6.))),
                px(3.),
                rgb(PANEL),
                px(1.2),
                rgb(MUTED),
                Default::default(),
            ));
        },
    )
    .w(px(12.))
    .h(px(12.))
    .flex_shrink_0()
}

fn reference_badge_icon(tag: bool, tint: u32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.2));
            let origin = bounds.origin;
            if tag {
                let points = [(1., 1.), (5., 1.), (10., 6.), (6., 10.), (1., 5.), (1., 1.)];
                path.move_to(origin + point(px(points[0].0), px(points[0].1)));
                for (x, y) in &points[1..] {
                    path.line_to(origin + point(px(*x), px(*y)));
                }
            } else {
                path.move_to(origin + point(px(3.), px(2.)));
                path.line_to(origin + point(px(3.), px(9.)));
                path.move_to(origin + point(px(3.), px(7.)));
                path.cubic_bezier_to(
                    origin + point(px(9.), px(3.)),
                    origin + point(px(8.), px(7.)),
                    origin + point(px(9.), px(6.)),
                );
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(tint));
            }
            let dots: &[(f32, f32)] = if tag {
                &[(3., 3.)]
            } else {
                &[(3., 2.), (3., 9.), (9., 3.)]
            };
            for (x, y) in dots {
                window.paint_quad(quad(
                    Bounds::new(
                        origin + point(px(*x - 1.), px(*y - 1.)),
                        size(px(2.), px(2.)),
                    ),
                    px(1.),
                    rgb(tint),
                    px(0.),
                    rgb(tint),
                    Default::default(),
                ));
            }
        },
    )
    .w(px(11.))
    .h(px(11.))
    .flex_shrink_0()
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

impl Workspace {
    fn error_indicator(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let keys = self.shortcuts.label("errors");
        div()
            .id("open-error-history")
            .h_8()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .px_2()
            .text_xs()
            .whitespace_nowrap()
            .bg(rgba(0x00000000))
            .text_color(rgb(ERROR))
            .cursor_pointer()
            .occlude()
            .hover(|style| style.bg(rgb(DANGER_HOVER)))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_error_history(cx);
                cx.stop_propagation();
            }))
            .child(
                div()
                    .size(px(14.))
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(ERROR))
                    .bg(rgba(0x00000000))
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(10.))
                    .child("!"),
            )
            .child(self.error_history.len().to_string())
            .children(
                keys.split(" / ")
                    .filter(|keys| !keys.is_empty())
                    .map(crate::shortcut::keys),
            )
    }

    fn error_overlay(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let count = self.error_history.len();
        let mut overlay = div()
            .id("error-history-overlay")
            .absolute()
            .left(px(16.))
            .bottom_0()
            .h_8()
            .w(px(0.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation());
        if self.error_history_open
            && let Some(entry) = self.error_history.current()
        {
            let page = self.error_history.page();
            let mut controls = div().flex().flex_wrap().items_center().gap_2();
            for (id, label, previous, enabled) in [
                ("previous-error", "← 이전", true, page > 0),
                ("next-error", "다음 →", false, page + 1 < count),
            ] {
                controls = controls.child(
                    button(id, label, false)
                        .when(!enabled, |button| button.opacity(0.4).cursor_default())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if enabled {
                                if previous {
                                    this.error_history.previous();
                                } else {
                                    this.error_history.next();
                                }
                                this.error_editor = None;
                                cx.notify();
                            }
                        })),
                );
            }
            controls = controls
                .child(
                    button("copy-error", "", false)
                        .child(crate::shortcut::hint("전체 복사", "ctrl + c"))
                        .on_click(cx.listener(|this, _, _, cx| this.copy_error(cx))),
                )
                .child(div().text_sm().child(format!("{} / {count}", page + 1)))
                .child(
                    self.shortcut_button("acknowledge-error", "확인완료", "confirm", false)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.acknowledge_error(cx);
                        })),
                );
            let available = (window.viewport_size().height - px(180.)).max(px(80.));
            overlay = overlay.child(
                div()
                    .id("error-history-card")
                    .absolute()
                    .bottom(px(40.))
                    .left_0()
                    .w(px(640.))
                    .max_w((window.viewport_size().width - px(24.)).max(px(0.)))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(rgb(PANEL))
                    .border_1()
                    .border_color(rgb(ERROR))
                    .rounded_lg()
                    .occlude()
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.error_history_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .child(div().text_sm().text_color(rgb(ERROR)).child("오류 기록"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(entry.source.clone()),
                    )
                    .child(
                        div()
                            .id("error-history-message")
                            .h(available.min(px(220.)))
                            .min_h_0()
                            .overflow_hidden()
                            .when_some(
                                self.error_editor
                                    .as_ref()
                                    .map(|(_, _, editor)| editor.clone()),
                                |body, editor| body.child(editor),
                            ),
                    )
                    .child(controls),
            );
        }
        overlay
    }

    fn sync_error_editor(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self
            .error_history
            .current()
            .filter(|_| self.error_history_open)
        else {
            self.error_editor = None;
            return;
        };
        let page = self.error_history.page();
        if self
            .error_editor
            .as_ref()
            .is_none_or(|(previous, message, _)| *previous != page || *message != entry.message)
        {
            let message = entry.message.clone();
            let editor = cx.new(|cx| CommitEditor::read_only(&message, cx));
            self.error_editor = Some((page, message, editor));
        }
    }

    fn copy_error(&self, cx: &mut Context<Self>) {
        if let Some(entry) = self.error_history.current() {
            cx.write_to_clipboard(ClipboardItem::new_string(entry.message.clone()));
        }
    }

    fn toggle_error_history(&mut self, cx: &mut Context<Self>) {
        if self.error_history.len() == 0 {
            return;
        }
        if !self.error_history_open {
            self.error_history.open();
            self.error_editor = None;
        }
        self.error_history_open = !self.error_history_open;
        cx.notify();
    }

    fn acknowledge_error(&mut self, cx: &mut Context<Self>) {
        self.error_history.acknowledge();
        self.error_editor = None;
        if self.error_history.len() == 0 {
            self.error_history_open = false;
        }
        cx.notify();
    }

    fn shortcut_button(
        &self,
        id: impl Into<ElementId>,
        label: &str,
        command: &str,
        active: bool,
    ) -> Stateful<Div> {
        button(id, "", active).child(self.shortcuts.button_hint(label, command))
    }

    fn open_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shortcut_draft = Some(self.shortcuts.clone());
        self.shortcut_settings = true;
        self.shortcut_tab = 0;
        self.shortcut_capture = None;
        self.shortcut_message = None;
        self.focus.focus(window);
        self.notify(cx);
    }

    fn close_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shortcut_settings = false;
        self.shortcut_draft = None;
        self.shortcut_capture = None;
        self.shortcut_message = None;
        self.focus.focus(window);
        self.notify(cx);
    }

    fn reset_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.shortcut_draft = Some(crate::shortcut::Bindings::default());
        self.shortcut_capture = None;
        self.shortcut_message = None;
        self.notify(cx);
    }

    fn apply_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = &self.shortcut_draft else {
            return;
        };
        let Some(path) = &self.state_path else {
            self.shortcut_message = self.error_history.report(
                "shortcut-settings",
                "단축키 설정",
                Some("설정 저장 경로를 사용할 수 없습니다.".into()),
            );
            self.notify(cx);
            return;
        };
        let mut state = self.session_state();
        match session::save_shortcuts(path, &mut state, draft) {
            Ok(()) => {
                self.shortcuts = state.shortcuts;
                self.close_shortcuts(window, cx);
            }
            Err(error) => {
                self.shortcut_message = self.error_history.report(
                    "shortcut-settings",
                    "단축키 설정",
                    Some(format!("단축키 저장 실패: {error:#}")),
                );
                self.notify(cx);
            }
        }
    }

    fn open_terminal(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.project().map(|project| project.path.clone()) else {
            return;
        };
        let task = cx
            .background_executor()
            .spawn(async move { crate::terminal::open(&path) });
        cx.spawn(async move |this, cx| {
            if let Err(error) = task.await {
                this.update(cx, |this, cx| {
                    this.error_history.report(
                        "terminal",
                        "터미널",
                        Some(format!("터미널 열기 실패: {error:#}")),
                    );
                    this.notify(cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn terminal_link(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("open-terminal")
            .ml_3()
            .text_xs()
            .text_color(rgb(ACCENT))
            .cursor_pointer()
            .child(crate::shortcut::hint(
                "터미널",
                &self.shortcuts.label("terminal"),
            ))
            .on_click(cx.listener(|this, _, _, cx| this.open_terminal(cx)))
    }

    fn open_file_manager(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.project().map(|project| project.path.clone()) else {
            return;
        };
        let task = cx
            .background_executor()
            .spawn(async move { crate::file_manager::open(&path) });
        cx.spawn(async move |this, cx| {
            if let Err(error) = task.await {
                this.update(cx, |this, cx| {
                    this.error_history.report(
                        "file-manager",
                        "탐색기",
                        Some(format!("폴더 열기 실패: {error:#}")),
                    );
                    this.notify(cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn explorer_link(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("open-file-manager")
            .ml_3()
            .text_xs()
            .text_color(rgb(ACCENT))
            .cursor_pointer()
            .child(crate::shortcut::hint(
                "탐색기",
                &self.shortcuts.label("explorer"),
            ))
            .on_click(cx.listener(|this, _, _, cx| this.open_file_manager(cx)))
    }

    fn shortcuts_link(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id("show-shortcuts")
            .ml_3()
            .text_xs()
            .text_color(rgb(ACCENT))
            .cursor_pointer()
            .child(crate::shortcut::hint(
                "단축키",
                &self.shortcuts.label("shortcuts"),
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_shortcuts(window, cx);
            }))
    }

    fn switch_shortcuts_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= crate::shortcut::GROUPS.len() {
            return;
        }
        self.shortcut_tab = index;
        self.shortcut_capture = None;
        self.shortcut_message = None;
        cx.notify();
    }

    fn shortcut_tabs(&self, cx: &mut Context<Self>) -> Div {
        let mut tabs = div().flex().flex_wrap().gap_1();
        for (index, group) in crate::shortcut::GROUPS.into_iter().enumerate() {
            tabs = tabs.child(
                button(("shortcut-tab", index), group, self.shortcut_tab == index).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.switch_shortcuts_tab(index, cx);
                    }),
                ),
            );
        }
        tabs
    }

    fn shortcuts_table(&self, editable: bool, cx: &mut Context<Self>) -> Div {
        let mut table = div().flex().flex_col().gap_3();
        if editable && let Some(message) = &self.shortcut_message {
            table = table.child(div().text_color(rgb(ACCENT)).child(message.clone()));
        }
        let bindings = if editable {
            self.shortcut_draft.as_ref().unwrap_or(&self.shortcuts)
        } else {
            &self.shortcuts
        };
        let commands = crate::shortcut::commands();
        for (index, group) in crate::shortcut::GROUPS.into_iter().enumerate() {
            if editable && self.shortcut_tab != index {
                continue;
            }
            for subsection in crate::shortcut::subsections(group).iter().copied() {
                let mut section = div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .gap_1()
                    .when(!editable || !subsection.is_empty(), |section| {
                        let heading = if editable {
                            subsection.to_owned()
                        } else if subsection.is_empty() {
                            group.to_owned()
                        } else {
                            format!("{group} / {subsection}")
                        };
                        section.child(
                            div()
                                .text_color(rgb(ACCENT))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(heading),
                        )
                    })
                    .when(!editable, |section| {
                        section.child(
                            div()
                                .flex()
                                .border_b_1()
                                .border_color(rgb(LINE))
                                .bg(rgb(0xe5e7eb))
                                .text_color(rgb(TEXT))
                                .font_weight(FontWeight::SEMIBOLD)
                                .py_2()
                                .child(div().flex_1().child("기능"))
                                .child(div().w(px(210.)).child("단축키1"))
                                .child(div().w(px(210.)).child("단축키2")),
                        )
                    });
                let section_commands: Vec<_> = commands
                    .iter()
                    .filter(|c| {
                        c.group == group
                            && crate::shortcut::list_visible(c)
                            && crate::shortcut::in_subsection(c, subsection)
                    })
                    .collect();
                if section_commands.is_empty() {
                    section = section.child(
                        div()
                            .py_2()
                            .text_color(rgb(MUTED))
                            .child("등록된 단축키 없음"),
                    );
                }
                for command in section_commands {
                    let mut row = div()
                        .flex()
                        .items_center()
                        .py_1()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .child(div().flex_1().min_w_0().child(command.name.clone()));
                    for (slot, value) in bindings.slots(command).into_iter().enumerate() {
                        let id = command.id.clone();
                        let clearing_id = id.clone();
                        let capturing = self
                            .shortcut_capture
                            .as_ref()
                            .is_some_and(|(selected, index)| selected == &id && *index == slot);
                        let label = if capturing {
                            "키 입력…".into()
                        } else if value.is_empty() {
                            "미지정".into()
                        } else {
                            value.replace("-", " + ")
                        };
                        let mut cell = div().w(px(210.)).flex().items_center().gap_1();
                        if editable {
                            cell = cell
                                .child(
                                    button(
                                        (
                                            SharedString::from(format!("key-{subsection}-{id}")),
                                            slot,
                                        ),
                                        "",
                                        capturing,
                                    )
                                    .h(px(36.))
                                    .child(crate::shortcut::badge(&label))
                                    .on_click(cx.listener(
                                        move |this, _, window, cx| {
                                            this.shortcut_capture = Some((id.clone(), slot));
                                            this.shortcut_message = None;
                                            this.focus.focus(window);
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        (
                                            SharedString::from(format!(
                                                "clear-{subsection}-{clearing_id}"
                                            )),
                                            slot,
                                        ),
                                        "×",
                                        false,
                                    )
                                    .h(px(36.))
                                    .py_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            if let Some(draft) = &mut this.shortcut_draft
                                                && draft
                                                    .set(&clearing_id, slot, String::new())
                                                    .is_ok()
                                            {
                                                this.shortcut_capture = None;
                                                this.shortcut_message = None;
                                                cx.notify();
                                            }
                                        },
                                    )),
                                );
                        } else {
                            cell = cell.child(crate::shortcut::badge(&label));
                        }
                        row = row.child(cell);
                    }
                    section = section.child(row);
                }
                table = table.child(section);
            }
        }
        table
    }

    pub fn new(paths: Vec<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            file_window: None,
            file_window_activation: None,
            search_tab: false,
            search_editor: cx.new(|cx| CommitEditor::single_line("", cx)),
            search_results: Vec::new(),
            search_selected: SEARCH_VIRTUAL_ROW,
            search_loading: false,
            search_generation: 0,
            state_path: session::state_path().ok(),
            popup_options: session::PopupOptions::default(),
            recent: Vec::new(),
            pending: 0,
            staging: HashSet::new(),
            remote_status: None,
            help: false,
            shortcut_settings: false,
            shortcut_tab: 0,
            shortcut_draft: None,
            shortcuts: crate::shortcut::Bindings::default(),
            shortcut_capture: None,
            shortcut_message: None,
            settings: None,
            branch_dialog: None,
            branch_busy: false,
            focus_after_popup: false,
            commit_dialog: None,
            discard_dialog: None,
            stash_dialog: None,
            delete_target: None,
            delete_confirm: false,
            focus: cx.focus_handle(),
            branch_checks_focus: [cx.focus_handle(), cx.focus_handle()],
            commit_push_focus: cx.focus_handle(),
            reset_mode_focus: cx.focus_handle(),
            upstream_scroll: ScrollHandle::new(),
            upstream_focus: cx.focus_handle(),
            list_focus: None,
            text_selection: None,
            partial_selection: HashMap::new(),
            text_drag: false,
            tabs: Vec::new(),
            tabs_scroll: ScrollHandle::new(),
            active: None,
            next_id: 0,
            error: None,
            layout: SplitLayout::default(),
            error_history: crate::error_history::ErrorHistory::default(),
            error_history_open: false,
            error_editor: None,
            body_bounds: Bounds::default(),
            resize_drag: None,
            start_pages: HashMap::new(),
            start_action_focus: [
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
            ],
            recent_action_focus: HashMap::new(),
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
            file_tree: false,
            selected_folder: None,
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
                this.shortcuts = state.shortcuts;
                this.popup_options = state.popup_options;
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
                    this.error =
                        this.error_history
                            .report("workspace", "앱 작업", Some(errors.join("\n")));
                }
                if let Some(index) = this.tabs.iter().position(|tab| Some(tab.id()) == active) {
                    this.tabs_scroll.scroll_to_item(index);
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
    }

    fn session_state(&self) -> Session {
        Session {
            version: 1,
            popup_options: self.popup_options.clone(),
            shortcuts: self.shortcuts.clone(),
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
        }
    }

    fn persist(&mut self) {
        let Some(path) = &self.state_path else {
            return;
        };
        let state = self.session_state();
        if let Err(error) = session::save(path, &state) {
            self.error =
                self.error_history
                    .report("workspace", "앱 작업", Some(format!("{error:#}")));
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

    fn collect_errors(&mut self, cx: &App) {
        let mut errors = Vec::new();
        let mut add = |key: String, source: String, error: Option<&str>| {
            if let Some(error) = error {
                errors.push((key, source, error.to_owned()));
            }
        };
        add("workspace".into(), "앱 작업".into(), self.error.as_deref());
        for project in self.tabs.iter().filter_map(WorkspaceTab::project) {
            let source = format!(
                "{} · {}",
                project.name,
                session::display_path(&project.path)
            );
            add(
                format!("project-{}", project.id),
                source.clone(),
                project.error.as_deref(),
            );
            add(
                format!("page-{}", project.id),
                format!("{source} · 이력 추가 조회"),
                project.page_error.as_deref(),
            );
        }
        let mut pages: Vec<_> = self.start_pages.iter().collect();
        pages.sort_by_key(|(id, _)| **id);
        for (id, page) in pages {
            add(
                format!("start-{id}"),
                "저장소 시작".into(),
                page.error.as_deref(),
            );
        }
        if let Some(dialog) = &self.branch_dialog {
            add(
                "branch-dialog".into(),
                format!("브랜치 작업 · {}", session::display_path(&dialog.path)),
                dialog.error.as_deref(),
            );
        }
        if let Some(dialog) = &self.commit_dialog {
            add(
                "commit-dialog".into(),
                "커밋".into(),
                dialog.error.as_deref(),
            );
        }
        if let Some(dialog) = &self.stash_dialog {
            add(
                "stash-dialog".into(),
                "Stash".into(),
                dialog.error.as_deref(),
            );
        }
        if let Some(dialog) = &self.discard_dialog {
            add(
                "discard-dialog".into(),
                "변경 폐기".into(),
                dialog.error.as_deref(),
            );
        }
        if let Some(settings) = &self.settings {
            let panel = settings.read(cx);
            add(
                format!(
                    "settings-{:?}-{}",
                    settings.entity_id(),
                    panel.error_revision()
                ),
                "설정".into(),
                panel.error(),
            );
        }
        add(
            "shortcut-settings".into(),
            "단축키 설정".into(),
            self.shortcut_message.as_deref(),
        );
        self.error_history.observe(errors);
    }

    fn notify(&mut self, cx: &mut Context<Self>) {
        self.collect_errors(cx);
        cx.notify();
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

    fn cursor_rows(&self, key: ListKey) -> Vec<usize> {
        if key.1 == 31 {
            return (0..self
                .detail_items
                .get(&key.0)
                .map_or(0, |rows| rows.lines.len()))
                .collect();
        }
        (0..self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|project| project.id == key.0)
            .map_or(0, |project| project.diff.lines.len()))
            .collect()
    }

    fn initial_text_selection(&self, key: ListKey) -> Option<(ListKey, Selection)> {
        let row = self.cursor_rows(key).into_iter().next()?;
        let position = Position { row, column: 0 };
        Some((
            key,
            Selection {
                anchor: position,
                cursor: position,
            },
        ))
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
            && self.stash_dialog.is_none()
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
        if let Some(handle) = self.file_window {
            handle
                .update(cx, |_, child, _| child.activate_window())
                .ok();
            cx.stop_propagation();
            return;
        }
        if !self.history_in_progress()
            && !self.branch_busy
            && self.shortcut_capture.is_none()
            && self
                .shortcuts
                .resolve("error-history", &event.keystroke)
                .is_some_and(|key| key.key == "f1")
        {
            self.toggle_error_history(cx);
            cx.stop_propagation();
            return;
        }
        if self.error_history_open {
            let text_focused = self
                .error_editor
                .as_ref()
                .is_some_and(|(_, _, editor)| editor.read(cx).focus.is_focused(window));
            if text_focused && crate::commit_editor::read_only_key(&event.keystroke) {
                return;
            }
            if event.keystroke.modifiers.control && event.keystroke.key == "c" {
                self.copy_error(cx);
                cx.stop_propagation();
                return;
            }
            if let Some(key) = self.shortcuts.resolve("modal", &event.keystroke) {
                if key.key == "escape" {
                    self.error_history_open = false;
                    cx.notify();
                } else if key.key == "enter" {
                    self.acknowledge_error(cx);
                }
            }
            cx.stop_propagation();
            return;
        }
        if (self.history_in_progress() && !self.search_tab) || self.branch_busy {
            cx.stop_propagation();
            return;
        }
        if self.shortcut_settings {
            let key = &event.keystroke;
            if let Some((id, slot)) = self.shortcut_capture.clone() {
                if key.key == "escape" {
                    self.shortcut_capture = None;
                    self.shortcut_message = None;
                } else if !["shift", "control", "ctrl", "alt", "cmd", "fn"]
                    .contains(&key.key.as_str())
                    && let Some(draft) = &mut self.shortcut_draft
                {
                    match draft.set(&id, slot, crate::shortcut::chord(key)) {
                        Ok(()) => {
                            self.shortcut_capture = None;
                            self.shortcut_message = None;
                        }
                        Err(error) => {
                            self.shortcut_message = self.error_history.report(
                                "shortcut-settings",
                                "단축키 설정",
                                Some(error),
                            )
                        }
                    }
                }
            } else if let Some(action) = self.shortcuts.resolve("shortcuts-modal", key) {
                if crate::shortcut::chord(&action) == "ctrl-d" {
                    self.reset_shortcuts(cx);
                } else if action.key == "escape" {
                    self.close_shortcuts(window, cx);
                } else if action.key == "enter" {
                    self.apply_shortcuts(window, cx);
                } else if ["left", "right"].contains(&action.key.as_str())
                    && let Some(index) = adjacent_tab(
                        self.shortcut_tab,
                        crate::shortcut::GROUPS.len(),
                        action.key == "right",
                    )
                {
                    self.switch_shortcuts_tab(index, cx);
                }
            }
            cx.stop_propagation();
            self.notify(cx);
            return;
        }
        let scope = if self.settings.is_some() {
            "settings-modal"
        } else if self.commit_dialog.is_some() {
            "commit-modal"
        } else if self.discard_dialog.is_some() {
            "discard-modal"
        } else if self.stash_dialog.is_some() {
            "stash-modal"
        } else if let Some(dialog) = &self.branch_dialog {
            match dialog.action {
                BranchAction::ChooseCheckout { .. } => "checkout",
                BranchAction::Reset { .. } => "reset",
                BranchAction::Connect { .. } => "upstream",
                _ => "branch",
            }
        } else if self.help || self.delete_target.is_some() {
            "modal"
        } else {
            if self
                .active
                .and_then(|id| self.start_pages.get(&id))
                .and_then(|page| page.url_editor.as_ref())
                .is_some_and(|editor| editor.read(cx).focus.is_focused(window))
            {
                "clone"
            } else {
                match self.list_focus {
                    Some(ListFocus::Branches(id))
                        if !self.search_tab
                            && self.project().is_some_and(|project| {
                                project.id == id && project.ref_tab == RefTab::Local
                            }) =>
                    {
                        "local-branches"
                    }
                    Some(ListFocus::Branches(_)) => "branches",
                    Some(ListFocus::History(_)) => "history",
                    Some(ListFocus::Files(id))
                        if self.project().is_some_and(|project| {
                            project.id == id && project.file_tab == FileTab::Stage
                        }) =>
                    {
                        "stage-files"
                    }
                    Some(ListFocus::Files(_)) => "files",
                    Some(ListFocus::Text(_)) => "viewer",
                    None => "main",
                }
            }
        };
        let input = &event.keystroke;
        if self.search_tab
            && self.project().is_some()
            && !input.modifiers.control
            && !input.modifiers.alt
            && !input.modifiers.platform
            && !input.modifiers.shift
        {
            if self.search_editor.read(cx).focus.is_focused(window)
                && input.key == "enter"
                && !self.search_editor.read(cx).is_composing()
            {
                self.run_search(cx);
                cx.stop_propagation();
                return;
            }
            if self.search_editor.read(cx).focus.is_focused(window)
                && self.search_editor.read(cx).message().is_empty()
                && ["left", "right"].contains(&input.key.as_str())
            {
                self.cycle_panel_tab(input.key == "right", window, cx);
                cx.stop_propagation();
                return;
            }
            if self.focus.is_focused(window)
                && matches!(self.list_focus, Some(ListFocus::Branches(_)))
                && ["enter", "/"].contains(&input.key.as_str())
            {
                self.search_editor.read(cx).focus.focus(window);
                if input.key == "enter" {
                    self.run_search(cx);
                }
                cx.stop_propagation();
                self.notify(cx);
                return;
            }
        }
        if input.key == "enter"
            && input.modifiers.shift
            && !input.modifiers.control
            && !input.modifiers.alt
            && !input.modifiers.platform
            && let Some(dialog) = &self.commit_dialog
            && dialog.editor.read(cx).focus.is_focused(window)
        {
            if dialog.busy {
                cx.stop_propagation();
            }
            return;
        }
        let key = &event.keystroke;
        if ["main", "clone"].contains(&scope)
            && let Some(id) = self.active.filter(|id| self.start_pages.contains_key(id))
        {
            if key.key == "tab"
                && !key.modifiers.control
                && !key.modifiers.alt
                && !key.modifiers.platform
            {
                let composing = self.start_pages.get(&id).is_some_and(|page| {
                    page.url_editor.as_ref().is_some_and(|editor| {
                        editor.read(cx).focus.is_focused(window) && editor.read(cx).is_composing()
                    })
                });
                if !composing {
                    crate::popup::focus_next(
                        &self.start_focus_order(cx),
                        key.modifiers.shift,
                        window,
                    );
                    if let Some(page) = self.start_pages.get(&id) {
                        for (index, path) in self.recent.iter().enumerate() {
                            if self.recent_action_focus.get(path).is_some_and(|handles| {
                                handles.iter().any(|handle| handle.is_focused(window))
                            }) {
                                page.recent_scroll
                                    .scroll_to_item(index, ScrollStrategy::Center);
                                break;
                            }
                        }
                    }
                    cx.stop_propagation();
                    self.notify(cx);
                }
                return;
            }
            if key.key == "space"
                && !key.modifiers.control
                && !key.modifiers.alt
                && !key.modifiers.shift
                && !key.modifiers.platform
            {
                if event.is_held {
                    cx.stop_propagation();
                    return;
                }
                if self.start_pages.get(&id).is_some_and(|page| page.busy) {
                    cx.stop_propagation();
                    return;
                }
                if let Some(mode) = self.start_action_focus[..3]
                    .iter()
                    .position(|focus| focus.is_focused(window))
                {
                    self.activate_start_action(id, mode as u8, window, cx);
                } else if self.start_action_focus[3].is_focused(window) {
                    self.start_repository(id, 2, cx);
                } else if let Some((path, action)) = self.recent.iter().find_map(|path| {
                    self.recent_action_focus.get(path).and_then(|focus| {
                        focus
                            .iter()
                            .position(|focus| focus.is_focused(window))
                            .map(|action| (path.clone(), action))
                    })
                }) {
                    if action == 0 {
                        self.open(path, cx);
                    } else {
                        self.delete_target = Some(path);
                        self.delete_confirm = false;
                        self.focus.focus(window);
                        self.notify(cx);
                    }
                } else {
                    return;
                }
                cx.stop_propagation();
                return;
            }
        }
        let Some(mapped_key) = self.shortcuts.resolve(scope, &event.keystroke) else {
            return;
        };
        let key = &mapped_key;
        if key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.shift
            && !key.modifiers.platform
            && key.key == "n"
        {
            if !event.is_held {
                self.add_empty_tab();
                self.focus.focus(window);
                self.notify(cx);
            }
            cx.stop_propagation();
            return;
        }
        if key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.shift
            && !key.modifiers.platform
            && key.key == "x"
        {
            if self.search_editor.read(cx).focus.is_focused(window) {
                return;
            }
            if let Some(id) = self.project().map(|project| project.id) {
                if !event.is_held {
                    self.close_tab(id, cx);
                    self.focus.focus(window);
                    self.notify(cx);
                }
                cx.stop_propagation();
            }
            return;
        }
        if key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.shift
            && !key.modifiers.platform
            && ["[", "]"].contains(&key.key.as_str())
        {
            if let Some(current) = self
                .tabs
                .iter()
                .position(|tab| Some(tab.id()) == self.active)
                && let Some(index) = adjacent_tab(current, self.tabs.len(), key.key == "]")
            {
                let id = self.tabs[index].id();
                self.activate_project_tab(id, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        let checkbox_focused = (self.commit_dialog.is_some()
            && self.commit_push_focus.is_focused(window))
            || (self.branch_dialog.is_some()
                && self
                    .branch_checks_focus
                    .iter()
                    .any(|focus| focus.is_focused(window)));
        if crate::controls::checkbox_key_repeat(key, event.is_held, checkbox_focused) {
            cx.stop_propagation();
            return;
        }
        if event.is_held
            && self.branch_dialog.is_some()
            && (key.key == "enter" || (key.modifiers.control && key.key == "s"))
        {
            cx.stop_propagation();
            return;
        }
        if key.key == "f9" {
            if !event.is_held {
                self.open_terminal(cx);
            }
            cx.stop_propagation();
            return;
        }
        if key.key == "f10" {
            if !event.is_held {
                self.open_file_manager(cx);
            }
            cx.stop_propagation();
            return;
        }
        if key.key == "f11" && scope != "modal" {
            self.open_shortcuts(window, cx);
            cx.stop_propagation();
            self.notify(cx);
            return;
        }

        if key.key == "tab"
            && !key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.platform
            && (self.settings.is_some()
                || self.branch_dialog.is_some()
                || self.stash_dialog.is_some()
                || self.commit_dialog.is_some()
                || self.help
                || self.delete_target.is_some()
                || self.discard_dialog.is_some())
        {
            if let Some(settings) = &self.settings {
                settings
                    .read(cx)
                    .focus_next(key.modifiers.shift, window, cx);
            } else {
                let mut inputs = Vec::new();
                let mut blocked = false;
                if let Some(dialog) = &self.branch_dialog {
                    if let Some(editor) = &dialog.editor {
                        blocked = editor.read(cx).is_composing();
                        inputs.push(editor.read(cx).focus.clone());
                    }
                    if matches!(dialog.action, BranchAction::Reset { .. }) {
                        inputs.push(self.reset_mode_focus.clone());
                    }
                    if matches!(dialog.action, BranchAction::Connect { .. }) {
                        inputs.push(self.upstream_focus.clone());
                    }
                    if matches!(dialog.action, BranchAction::Revert { .. })
                        || matches!(&dialog.action, BranchAction::RevertControl { state, action: crate::branches::RevertControl::Continue, .. } if state.no_commit.is_none())
                        || matches!(&dialog.action, BranchAction::Delete { plan, .. } if !plan.related.is_empty())
                    {
                        inputs.push(self.branch_checks_focus[0].clone());
                    }
                    if matches!(dialog.action, BranchAction::Create { .. }) {
                        inputs.extend(self.branch_checks_focus.iter().cloned());
                    }
                } else if let Some(dialog) = &self.stash_dialog {
                    blocked = dialog.busy;
                    if let Some(editor) = &dialog.editor {
                        blocked |= editor.read(cx).is_composing();
                        inputs.push(editor.read(cx).focus.clone());
                    }
                } else if let Some(dialog) = &self.commit_dialog {
                    blocked = dialog.busy || dialog.editor.read(cx).is_composing();
                    inputs.push(dialog.editor.read(cx).focus.clone());
                    inputs.push(self.commit_push_focus.clone());
                }
                if !blocked {
                    crate::popup::focus_next(&inputs, key.modifiers.shift, window);
                }
            }
            cx.stop_propagation();
            self.notify(cx);
            return;
        }
        if let Some(dialog) = &self.stash_dialog {
            if dialog.busy {
                cx.stop_propagation();
                return;
            }
            if dialog
                .editor
                .as_ref()
                .is_some_and(|editor| editor.read(cx).is_composing())
            {
                return;
            }
            if key.key == "enter" {
                self.stash_changes(cx);
                cx.stop_propagation();
            } else if key.key == "escape" {
                self.stash_dialog = None;
                self.focus.focus(window);
                cx.stop_propagation();
                self.notify(cx);
            }
            return;
        }
        if let Some(dialog) = &self.discard_dialog {
            if !dialog.busy && key.modifiers.control && key.key == "s" {
                self.discard_selected(cx);
                cx.stop_propagation();
                return;
            }
            if !dialog.busy && key.key == "escape" {
                self.discard_dialog = None;
                self.focus.focus(window);
                self.notify(cx);
            }
            cx.stop_propagation();
            return;
        }
        if let Some(dialog) = &self.commit_dialog {
            if dialog.busy {
                cx.stop_propagation();
                return;
            }
            if dialog.editor.read(cx).is_composing() {
                return;
            }
            if key.key == "space" && self.commit_push_focus.is_focused(window) {
                if let Some(dialog) = &mut self.commit_dialog {
                    dialog.push_after = !dialog.push_after;
                }
                cx.stop_propagation();
                self.notify(cx);
                return;
            }
            if key.modifiers.control && key.key == "s" {
                self.submit_commit(cx);
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
                self.notify(cx);
            }
            return;
        }
        if let Some(settings) = self.settings.clone() {
            if (key.key == "space" || (key.modifiers.control && key.key == "s"))
                && !settings.read(cx).is_composing(cx)
                && settings.update(cx, |settings, cx| {
                    if event.is_held {
                        return settings.action_focused(window, cx);
                    }
                    settings.activate_focused(window, key.key == "space", cx)
                })
            {
                cx.stop_propagation();
            } else if key.key == "escape" && !settings.read(cx).is_composing(cx) {
                self.close_settings(window, cx);
                cx.stop_propagation();
            } else if key.modifiers.control && key.key == "s" {
                if !event.is_held && !settings.read(cx).is_composing(cx) {
                    settings.update(cx, |settings, cx| settings.save(cx));
                }
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
            if key.key == "space"
                && !dialog
                    .editor
                    .as_ref()
                    .is_some_and(|editor| editor.read(cx).is_composing())
                && let Some(index) = self
                    .branch_checks_focus
                    .iter()
                    .position(|focus| focus.is_focused(window))
            {
                let changed = if let Some(dialog) = &mut self.branch_dialog {
                    match &mut dialog.action {
                        BranchAction::Revert { no_commit, .. } if index == 0 => {
                            *no_commit = !*no_commit;
                            self.popup_options.revert_no_commit = *no_commit;
                            true
                        }
                        BranchAction::RevertControl {
                            state,
                            action: crate::branches::RevertControl::Continue,
                            no_commit,
                        } if index == 0 && state.no_commit.is_none() => {
                            *no_commit = !*no_commit;
                            true
                        }
                        BranchAction::Delete {
                            plan,
                            delete_related,
                        } if index == 0 && !plan.related.is_empty() => {
                            *delete_related = !*delete_related;
                            true
                        }
                        BranchAction::Create {
                            remote, checkout, ..
                        } => {
                            if index == 0 {
                                *remote = !*remote;
                            } else {
                                *checkout = !*checkout;
                            }
                            self.popup_options.branch_remote = *remote;
                            self.popup_options.branch_checkout = *checkout;
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                };
                if changed {
                    self.persist();
                    cx.stop_propagation();
                    self.notify(cx);
                    return;
                }
            }
            let Some(dialog) = &self.branch_dialog else {
                return;
            };
            if matches!(dialog.action, BranchAction::ChooseCheckout { .. })
                && !key.modifiers.control
                && !key.modifiers.alt
                && !key.modifiers.shift
                && !key.modifiers.platform
                && let Some(index) = candidate_shortcut(&key.key)
            {
                self.choose_checkout_candidate(index, cx);
                cx.stop_propagation();
                return;
            }
            if self.branch_busy {
                cx.stop_propagation();
                return;
            }
            if let BranchAction::Connect { plan, selected } = &dialog.action {
                if key.key == "space" {
                    if !plan.candidates.is_empty() {
                        self.upstream_focus.focus(window);
                    }
                    cx.stop_propagation();
                    return;
                }
                let next = match key.key.as_str() {
                    "up" if !plan.candidates.is_empty() => Some(selected.saturating_sub(1)),
                    "down" if !plan.candidates.is_empty() => {
                        Some((selected + 1).min(plan.candidates.len() - 1))
                    }
                    _ => None,
                };
                if let Some(next) = next {
                    if let Some(dialog) = &mut self.branch_dialog
                        && let BranchAction::Connect { selected, .. } = &mut dialog.action
                    {
                        *selected = next;
                    }
                    self.upstream_scroll.scroll_to_item(next);
                    self.upstream_focus.focus(window);
                    cx.stop_propagation();
                    self.notify(cx);
                    return;
                }
            }
            if let BranchAction::Reset { mode, .. } = &dialog.action {
                let digit = if !key.modifiers.control
                    && !key.modifiers.alt
                    && !key.modifiers.shift
                    && !key.modifiers.platform
                {
                    match key.key.as_str() {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        _ => None,
                    }
                } else {
                    None
                };
                let direction = navigation_key(&key.key);
                let index = crate::branches::ResetMode::ALL
                    .iter()
                    .position(|m| m == mode)
                    .unwrap_or(0);
                let next = digit.or_else(|| {
                    if self.reset_mode_focus.is_focused(window) {
                        match direction {
                            "up" | "left" => Some((index + 2) % 3),
                            "down" | "right" => Some((index + 1) % 3),
                            _ => None,
                        }
                    } else {
                        None
                    }
                });
                if let Some(next) = next {
                    if let Some(dialog) = &mut self.branch_dialog
                        && let BranchAction::Reset { mode, .. } = &mut dialog.action
                    {
                        *mode = crate::branches::ResetMode::ALL[next];
                    }
                    self.reset_mode_focus.focus(window);
                    cx.stop_propagation();
                    self.notify(cx);
                    return;
                }
                if self.reset_mode_focus.is_focused(window) && key.key == "space" {
                    cx.stop_propagation();
                    return;
                }
            }
            if matches!(dialog.action, BranchAction::Revert { .. })
                && self.branch_checks_focus[0].is_focused(window)
                && key.key == "space"
            {
                return;
            }
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
                if let BranchAction::ChooseCheckout { selected, .. } = dialog.action {
                    self.choose_checkout_candidate(selected, cx);
                } else {
                    self.submit_branch(cx);
                }
                cx.stop_propagation();
            }
            return;
        }
        if self.help || self.delete_target.is_some() {
            if key.key == "escape" {
                self.help = false;
                self.delete_target = None;
                self.notify(cx);
            }
            cx.stop_propagation();
            return;
        }
        if self.search_tab && self.project().is_some() {
            if self.search_editor.read(cx).focus.is_focused(window) {
                if key.key == "enter" && !self.search_editor.read(cx).is_composing() {
                    self.run_search(cx);
                    cx.stop_propagation();
                } else if ["up", "down"].contains(&key.key.as_str())
                    && !self.search_editor.read(cx).is_composing()
                    && !key.modifiers.control
                    && !key.modifiers.alt
                    && !key.modifiers.shift
                    && !key.modifiers.platform
                {
                    self.search_selected = if key.key == "down" && !self.search_results.is_empty() {
                        0
                    } else {
                        SEARCH_VIRTUAL_ROW
                    };
                    self.list_focus = self
                        .project()
                        .map(|project| ListFocus::Branches(project.id));
                    self.focus.focus(window);
                    cx.stop_propagation();
                    self.notify(cx);
                }
                return;
            }
            if matches!(self.list_focus, Some(ListFocus::Branches(_)))
                && !key.modifiers.control
                && !key.modifiers.alt
            {
                match key.key.as_str() {
                    "up" | "down" => match search_arrow_target(
                        self.search_selected,
                        self.search_results.len(),
                        key.key == "down",
                    ) {
                        SearchArrowTarget::Input => {
                            self.search_editor.read(cx).focus.focus(window);
                        }
                        SearchArrowTarget::Row(index) => self.search_selected = index,
                    },
                    "space" => {
                        if matches!(
                            self.search_results.get(self.search_selected),
                            Some(feather_git::git_search::Match::Commit { .. })
                        ) {
                            self.activate_search_result(cx);
                        }
                    }
                    "o" => {
                        if let Some(feather_git::git_search::Match::File { path, .. }) =
                            self.search_results.get(self.search_selected)
                        {
                            self.open_search_file(path.clone(), cx);
                        }
                    }
                    "enter" | "/" => {
                        self.search_editor.read(cx).focus.focus(window);
                        if key.key == "enter" {
                            self.run_search(cx);
                        }
                    }
                    "left" | "right" if !key.modifiers.shift && !key.modifiers.platform => {
                        self.cycle_panel_tab(key.key == "right", window, cx);
                    }
                    _ => {}
                }
                if ["up", "down", "space", "o", "enter", "/", "left", "right"]
                    .contains(&key.key.as_str())
                {
                    cx.stop_propagation();
                    self.notify(cx);
                    return;
                }
            }
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
        if let Some(editor) = self
            .active
            .and_then(|id| self.start_pages.get(&id))
            .and_then(|page| page.url_editor.as_ref())
            && editor.read(cx).focus.is_focused(window)
        {
            if key.key == "enter" && !editor.read(cx).is_composing() {
                if let Some(id) = self.active {
                    self.start_repository(id, 2, cx);
                }
                cx.stop_propagation();
            }
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
        if ["branches", "local-branches", "files", "stage-files"].contains(&scope)
            && !key.modifiers.control
            && !key.modifiers.alt
            && !key.modifiers.shift
            && !key.modifiers.platform
            && ["left", "right"].contains(&key.key.as_str())
        {
            self.cycle_panel_tab(key.key == "right", window, cx);
            cx.stop_propagation();
            return;
        }
        if !key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform {
            if self
                .project()
                .is_some_and(|project| project.snapshot.integration_active())
                && ["c", "s", "o", "d", "b", "t", "x", "v", "m", "r", "p"]
                    .contains(&key.key.as_str())
            {
                self.error = self.error_history.report("workspace", "앱 작업", Some("진행 중인 작업이나 미해결 충돌이 있습니다. 파일을 수정하고 Stage한 뒤 진행 표시줄에서 완료하거나 취소하세요.".into()));
                cx.stop_propagation();
                self.notify(cx);
                return;
            }
            if key.key == "`" && !key.modifiers.shift && ["files", "stage-files"].contains(&scope) {
                self.file_tree = !self.file_tree;
                self.selected_folder = None;
                self.rebuild_file_rows(true);
                cx.stop_propagation();
                self.notify(cx);
                return;
            }
            if !key.modifiers.shift
                && let Some(project) = self.project()
                && let Some(section) = section_focus(
                    &key.key,
                    project.id,
                    project.file_tab == FileTab::History && project.show_commit_detail,
                )
            {
                self.list_focus = Some(section);
                self.text_drag = false;
                if let ListFocus::Text(text_key) = section
                    && self
                        .text_selection
                        .as_ref()
                        .is_none_or(|(selected, _)| *selected != text_key)
                {
                    self.text_selection = self.initial_text_selection(text_key);
                }
                self.focus.focus(window);
                cx.stop_propagation();
                self.notify(cx);
                return;
            }

            if key.key == "o" && !key.modifiers.shift {
                if let Some(project) = self.project() {
                    let id = project.id;
                    if self.list_focus == Some(ListFocus::History(id)) {
                        if let Some(commit) = project.commit.clone() {
                            self.checkout_history_commit(id, commit, cx);
                        }
                    } else if self.list_focus == Some(ListFocus::Branches(id)) {
                        let selected = project
                            .revision
                            .as_ref()
                            .or(project.snapshot.current_branch.as_ref());
                        let reference = project
                            .snapshot
                            .references
                            .iter()
                            .find(|reference| {
                                reference.kind == project.ref_tab
                                    && Some(&reference.revision) == selected
                            })
                            .cloned();
                        if let Some(reference) = reference {
                            match reference.kind {
                                RefTab::Stash => {
                                    self.restore_stash_reference(id, reference.revision, cx)
                                }
                                RefTab::Tag => self.checkout_reference(id, reference.target, cx),
                                _ => self.checkout_reference(id, reference.revision, cx),
                            }
                        }
                    }
                }
                cx.stop_propagation();
                return;
            }
            if key.key == "r" && !key.modifiers.shift && scope == "local-branches" {
                self.open_branch_upstream(window, cx);
                cx.stop_propagation();
                return;
            }
            if matches!(key.key.as_str(), "v" | "m" | "r") && !key.modifiers.shift {
                self.open_history_operation(&key.key, window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "x" && !key.modifiers.shift {
                self.open_reset(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "t" && !key.modifiers.shift {
                self.open_branch_create(window, cx);
                if let Some(dialog) = &mut self.branch_dialog
                    && let BranchAction::Create { commit, .. } = &dialog.action
                {
                    dialog.action = BranchAction::Tag {
                        commit: commit.clone(),
                    };
                    if let Some(editor) = &dialog.editor {
                        editor.update(cx, |editor, _| editor.text_size = 14.);
                    }
                }
                cx.stop_propagation();
                return;
            }
            if key.key == "b" && !key.modifiers.shift {
                self.open_branch_create(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "?" || key.key_char.as_deref() == Some("?") {
                self.help = true;
                cx.stop_propagation();
                self.notify(cx);
                return;
            }
            if key.key == "s" && !key.modifiers.shift && scope == "stage-files" {
                self.open_stash(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "f" {
                self.remote_operation(crate::setup::RemoteOperation::Fetch, cx);
                cx.stop_propagation();
                return;
            }
            if key.key == "c" && scope == "stage-files" {
                self.open_commit(window, cx);
                cx.stop_propagation();
                return;
            }
            if key.key.eq_ignore_ascii_case("d") {
                if self
                    .project()
                    .is_some_and(|p| self.list_focus == Some(ListFocus::Branches(p.id)))
                {
                    if self.project().is_some_and(|p| p.ref_tab == RefTab::Stash) {
                        if !key.modifiers.shift {
                            self.open_stash_delete(window, cx);
                        }
                    } else {
                        self.open_reference_delete(window, cx);
                    }
                } else if !key.modifiers.shift {
                    self.open_discard(window, cx);
                }
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
        let nav_key = if !key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform {
            navigation_key(&key.key)
        } else {
            key.key.as_str()
        };
        if let Some(ListFocus::Text(text_key)) = self.list_focus {
            if Some(text_key.0) != self.active {
                return;
            }
            if text_key.1 == 30
                && !key.modifiers.control
                && !key.modifiers.alt
                && !key.modifiers.shift
                && !key.modifiers.platform
                && let Some(project) = self.project()
            {
                let (id, generation) = (project.id, project.diff_generation);
                if key.key == "space" {
                    if let Some((_, selection)) = self
                        .text_selection
                        .as_ref()
                        .filter(|(key, _)| *key == text_key)
                    {
                        let rows = selection.rows(project.diff.lines.len());
                        self.toggle_partial_rows(id, generation, rows, cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                if key.key == "enter" {
                    self.stage_partial(id, generation, None, cx);
                    cx.stop_propagation();
                    return;
                }
            }
            let lines = self.text_lines(text_key);
            let cursor_rows = self.cursor_rows(text_key);
            if cursor_rows.is_empty() {
                self.text_selection = None;
                cx.stop_propagation();
                return;
            }
            if let Some((selection_key, selection)) = &mut self.text_selection
                && *selection_key == text_key
            {
                if key.modifiers.control && key.key == "c" {
                    cx.write_to_clipboard(ClipboardItem::new_string(selection.text(&lines)));
                } else if key.modifiers.control && key.key == "a" {
                    selection.anchor = Position {
                        row: cursor_rows[0],
                        column: 0,
                    };
                    let last = *cursor_rows.last().unwrap();
                    selection.cursor = Position {
                        row: last,
                        column: lines[last].chars().count(),
                    };
                } else if ["up", "down", "left", "right", "home", "end"].contains(&nav_key) {
                    selection.move_cursor_on_rows(
                        &lines,
                        &cursor_rows,
                        nav_key,
                        key.modifiers.shift,
                    );
                    if let Some(scroll) = self.list_scroll.get(&text_key) {
                        scroll.scroll_to_item(selection.cursor.row, ScrollStrategy::Center);
                    }
                }
                cx.stop_propagation();
                self.notify(cx);
            }
            return;
        }
        if key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform && key.key == "a"
        {
            self.selected_folder = None;
            if let Some(ListFocus::Files(id)) = self.list_focus
                && self.active == Some(id)
                && let Some(project) = self.project_mut()
                && project.file_tab == FileTab::Stage
            {
                project.stage_selection.select_all(&project.snapshot.stage);
                if project.selected_file.is_none() && !project.snapshot.stage.is_empty() {
                    self.select_file(0, cx);
                }
                self.notify(cx);
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
            if let Some(project) = self
                .project()
                .filter(|p| self.list_focus == Some(ListFocus::Branches(p.id)))
            {
                let selected = project
                    .revision
                    .as_ref()
                    .or(project.snapshot.current_branch.as_ref());
                let reference = project
                    .snapshot
                    .references
                    .iter()
                    .find(|reference| {
                        reference.kind == project.ref_tab && Some(&reference.revision) == selected
                    })
                    .cloned();
                if let Some(reference) = reference {
                    self.select_reference(reference.revision, reference.target, None, cx);
                }
            } else {
                self.stage_selected(cx);
            }
            cx.stop_propagation();
            return;
        }
        if !key.modifiers.control && !key.modifiers.alt && ["up", "down"].contains(&nav_key) {
            self.navigate(nav_key == "down", key.modifiers.shift, cx);
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
                    if let Some(project) = self.project_mut() {
                        project.revision = Some(reference.revision);
                        project.history_target = None;
                    }
                    cx.notify();
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
                let key = (id, if history { 21 } else { 20 });
                let order: Vec<_> = self
                    .file_items
                    .get(&key)
                    .into_iter()
                    .flatten()
                    .filter_map(FileRow::file_index)
                    .collect();
                let rows: Vec<_> = self
                    .file_items
                    .get(&key)
                    .into_iter()
                    .flatten()
                    .filter(|row| !extend || row.file_index().is_some())
                    .cloned()
                    .collect();
                let current = if history && p.show_commit_detail {
                    Some(0)
                } else if let Some((folder_key, path)) = &self.selected_folder
                    && *folder_key == key
                    && !extend
                {
                    rows.iter().position(|row| matches!(row, FileRow::Folder { path: candidate } if candidate == path)).map(|position| position + offset)
                } else {
                    p.selected_file
                        .and_then(|index| {
                            rows.iter()
                                .position(|item| item.file_index() == Some(index))
                        })
                        .map(|position| position + offset)
                };
                if let Some(index) = text_selection::adjacent(current, rows.len() + offset, down) {
                    if current == Some(index) {
                        return;
                    }
                    if history && index == 0 {
                        self.show_detail();
                        cx.notify();
                        return;
                    }
                    let item = &rows[index - offset];
                    if let FileRow::Folder { path } = item {
                        self.select_file_folder(key, path.clone());
                        if let Some(row) = self
                            .file_items
                            .get(&key)
                            .and_then(|all| all.iter().position(|row| row == item))
                            && let Some(scroll) = self.list_scroll.get(&key)
                        {
                            scroll.scroll_to_item(row, ScrollStrategy::Center);
                        }
                        cx.notify();
                        return;
                    }
                    let Some(index) = item.file_index() else {
                        return;
                    };
                    if let Some(row) = self
                        .file_items
                        .get(&key)
                        .and_then(|rows| rows.iter().position(|row| *row == FileRow::File(index)))
                        && let Some(scroll) = self.list_scroll.get(&key)
                    {
                        scroll.scroll_to_item(row, ScrollStrategy::Center);
                    }
                    if !history && let Some(project) = self.project_mut() {
                        project.stage_selection.click_visible(
                            &project.snapshot.stage,
                            &order,
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
                    Err(error) => {
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        )
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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

    fn submit_commit(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.commit_dialog else {
            return;
        };
        if dialog.busy || dialog.editor.read(cx).is_composing() {
            return;
        }
        let message = dialog.editor.read(cx).message().to_owned();
        if message.trim().is_empty() {
            dialog.error = self.error_history.report(
                "commit-dialog",
                "커밋",
                Some("커밋 메시지를 입력하세요.".into()),
            );
            self.notify(cx);
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
            dialog.error = self.error_history.report(
                "commit-dialog",
                "커밋",
                Some("staged 변경사항이 없습니다.".into()),
            );
            self.notify(cx);
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
        let push_after = dialog.push_after;
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
                        this.error = this
                            .error_history
                            .report("workspace", "앱 작업", push_error);
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
                            dialog.error = this.error_history.report(
                                "commit-dialog",
                                "커밋",
                                Some(format!("{error:#}")),
                            );
                            dialog.editor.update(cx, |editor, cx| {
                                editor.enabled = true;
                                cx.notify();
                            });
                        }
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
        let changes = if let Some((key, directory)) = &self.selected_folder
            && *key == (p.id, 20)
            && self.file_tree
        {
            directory_stage_changes(&p.snapshot.stage, directory)
        } else {
            p.stage_selection.changes(&p.snapshot.stage)
        };
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
                    Err(error) => {
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        )
                    }
                }
                if let Err(error) = operation {
                    this.error = this.error_history.report(
                        "workspace",
                        "앱 작업",
                        Some(format!("{error:#}")),
                    );
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn diff_cursor_row(&self, id: u64) -> Option<usize> {
        self.text_selection
            .as_ref()
            .filter(|(key, _)| *key == (id, 30))
            .map(|(_, selection)| selection.cursor.row)
    }

    fn resolve_conflict(
        &mut self,
        id: u64,
        generation: u64,
        choice: feather_git::conflict::Choice,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(p) = self.project().filter(|p| {
            p.id == id && p.diff_generation == generation && p.file_tab == FileTab::Stage
        }) else {
            return;
        };
        let Some(file) = p
            .selected_file
            .and_then(|index| p.files().get(index))
            .cloned()
        else {
            return;
        };
        let Some(preview) = p.diff.conflict.clone() else {
            return;
        };
        let Some(region) = self
            .diff_cursor_row(id)
            .and_then(|row| p.diff.conflict_at_row(row))
        else {
            return;
        };
        if file.status != "U" || file.staged {
            return;
        }
        let root = p.path.clone();
        self.pending += 1;
        self.error = None;
        let task = cx.background_executor().spawn(async move {
            let operation =
                feather_git::conflict::apply(&root, &file.path, &preview, region, choice);
            (operation, git::load_stage(&root))
        });
        cx.spawn(async move |this, cx| {
            let (operation, refreshed) = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                match refreshed {
                    Ok(files) => this.apply_stage_files(id, files, &Default::default(), cx),
                    Err(error) => {
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        )
                    }
                }
                if let Err(error) = operation {
                    this.error = this.error_history.report(
                        "workspace",
                        "앱 작업",
                        Some(format!("{error:#}")),
                    );
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn toggle_partial_line(
        &mut self,
        id: u64,
        generation: u64,
        row: usize,
        cx: &mut Context<Self>,
    ) {
        self.toggle_partial_rows(id, generation, row..row.saturating_add(1), cx);
    }

    fn toggle_partial_rows(
        &mut self,
        id: u64,
        generation: u64,
        range: std::ops::Range<usize>,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project().filter(|p| {
            p.id == id
                && p.diff_generation == generation
                && p.file_tab == FileTab::Stage
                && p.diff.partial.is_some()
        }) else {
            return;
        };
        let rows: Vec<_> = project
            .diff
            .lines
            .iter()
            .enumerate()
            .skip(range.start)
            .take(range.end.saturating_sub(range.start))
            .filter(|(_, line)| matches!(line.kind, '+' | '-'))
            .map(|(row, _)| row)
            .collect();
        if rows.is_empty() {
            return;
        }
        let entry = self
            .partial_selection
            .entry(id)
            .or_insert_with(|| (generation, Default::default()));
        if entry.0 != generation {
            *entry = (generation, Default::default());
        }
        toggle_checked_rows(&mut entry.1, &rows);
        cx.notify();
    }

    fn stage_partial(
        &mut self,
        id: u64,
        generation: u64,
        hunk: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(p) = self.project().filter(|p| {
            p.id == id && p.diff_generation == generation && p.file_tab == FileTab::Stage
        }) else {
            return;
        };
        let Some(file) = p
            .selected_file
            .and_then(|index| p.files().get(index))
            .cloned()
        else {
            return;
        };
        let Some(preview) = p.diff.partial.clone() else {
            return;
        };
        let rows = match hunk {
            Some(header) => feather_git::partial_stage::hunk_rows(&p.diff.lines, header),
            None => self
                .partial_selection
                .get(&id)
                .filter(|(selected_generation, _)| *selected_generation == generation)
                .map(|(_, rows)| rows.iter().copied().collect())
                .unwrap_or_default(),
        };
        if rows.is_empty() {
            return;
        }
        let root = p.path.clone();
        let lines = p.diff.lines.clone();
        let raw_path = file.path.clone();
        self.pending += 1;
        self.error = None;
        self.partial_selection.remove(&id);
        let task = cx.background_executor().spawn(async move {
            let operation =
                feather_git::partial_stage::apply(&root, &file, &preview, &lines, &rows);
            (operation, git::load_stage(&root))
        });
        cx.spawn(async move |this, cx| {
            let (operation, refreshed) = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                match refreshed {
                    Ok(files) => {
                        let staged = !files
                            .iter()
                            .any(|file| file.path == raw_path && !file.staged);
                        let targets = std::collections::BTreeMap::from([(raw_path, staged)]);
                        this.apply_stage_files(id, files, &targets, cx);
                    }
                    Err(error) => {
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        )
                    }
                }
                if let Err(error) = operation {
                    this.error = this.error_history.report(
                        "workspace",
                        "앱 작업",
                        Some(format!("{error:#}")),
                    );
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn apply_stage_files(
        &mut self,
        id: u64,
        files: Vec<FileChange>,
        targets: &std::collections::BTreeMap<Vec<u8>, bool>,
        cx: &mut Context<Self>,
    ) {
        let folder_selected = self
            .selected_folder
            .as_ref()
            .is_some_and(|(key, _)| *key == (id, 20));
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
        p.snapshot.unmerged = p
            .snapshot
            .stage
            .iter()
            .filter(|file| file.status == "U")
            .map(|file| file.path.clone())
            .collect();
        if let Some(stash) = &mut p.snapshot.stash_restore {
            stash.conflicts = p.snapshot.unmerged.clone();
        }
        if let Some(merge) = &mut p.snapshot.merge {
            merge.conflicts = p
                .snapshot
                .stage
                .iter()
                .filter(|file| file.status == "U")
                .map(|file| file.path.clone())
                .collect();
        }
        if let Some(rebase) = &mut p.snapshot.rebase {
            rebase.conflicts = p
                .snapshot
                .stage
                .iter()
                .filter(|file| file.status == "U")
                .map(|file| file.path.clone())
                .collect();
        }
        if let Some(revert) = &mut p.snapshot.revert {
            revert.conflicts = p
                .snapshot
                .stage
                .iter()
                .filter(|file| file.status == "U")
                .map(|file| file.path.clone())
                .collect();
        }
        p.error = None;
        p.stage_selection.restore(&p.snapshot.stage, targets);
        if folder_selected {
            p.stage_selection.clear();
        }
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
        self.file_items.insert(
            (id, 20),
            display_file_rows(&p.snapshot.stage, self.file_tree),
        );
        if show_stage {
            if let Some(index) = selected {
                if let Some(row) = self
                    .file_items
                    .get(&(id, 20))
                    .and_then(|rows| rows.iter().position(|row| row.file_index() == Some(index)))
                    && let Some(scroll) = self.list_scroll.get(&(id, 20))
                {
                    scroll.scroll_to_item(row, ScrollStrategy::Center);
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

    fn open_stash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending > 0 || self.busy() {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        if project.loading || project.diff_loading {
            return;
        }
        let id = project.id;
        let editor = cx.new(|cx| CommitEditor::single_line("", cx));
        editor.read(cx).focus.focus(window);
        self.stash_dialog = Some(StashDialog {
            project_id: id,
            editor: Some(editor),
            deletion: None,
            busy: false,
            error: None,
        });
        cx.notify();
    }

    fn open_stash_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let Some(reference) =
            project.snapshot.references.iter().find(|r| {
                r.kind == RefTab::Stash && Some(&r.revision) == project.revision.as_ref()
            })
        else {
            return;
        };
        let selector = reference
            .name
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned();
        self.stash_dialog = Some(StashDialog {
            project_id: project.id,
            editor: None,
            deletion: Some((selector, reference.revision.clone(), reference.name.clone())),
            busy: false,
            error: None,
        });
        self.focus.focus(window);
        cx.notify();
    }

    fn stash_changes(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.stash_dialog else {
            return;
        };
        if dialog.busy || self.pending > 0 {
            return;
        }
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
        if name.as_ref().is_some_and(|name| name.is_empty()) {
            dialog.error = self.error_history.report(
                "stash-dialog",
                "Stash",
                Some("스태시 이름을 입력하세요.".into()),
            );
            self.notify(cx);
            return;
        }
        let deletion = dialog.deletion.clone();
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
        if let Some(editor) = &dialog.editor {
            editor.update(cx, |editor, cx| {
                editor.enabled = false;
                cx.notify();
            });
        }
        dialog.busy = true;
        dialog.error = None;
        self.pending += 1;
        let task = cx.background_executor().spawn(async move {
            let operation = if let Some((selector, commit, _)) = deletion {
                crate::setup::delete_stash(&root, &selector, &commit)
            } else {
                crate::setup::stash_all(&root, name.as_deref().unwrap_or_default())
            };
            let refreshed = git::load_stage(&root);
            (operation, refreshed)
        });
        cx.spawn(async move |this, cx| {
            let (operation, refreshed) = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                if !this.tabs.iter().any(|tab| tab.id() == id) {
                    this.stash_dialog = None;
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
                    if let Some(dialog) = &mut this.stash_dialog {
                        dialog.busy = false;
                        dialog.error =
                            this.error_history
                                .report("stash-dialog", "Stash", Some(error));
                        if let Some(editor) = &dialog.editor {
                            editor.update(cx, |editor, cx| {
                                editor.enabled = true;
                                cx.notify();
                            });
                        }
                    }
                } else {
                    this.stash_dialog = None;
                    this.refresh_after_branch(id, false, None, cx);
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
                        dialog.error =
                            this.error_history
                                .report("discard-dialog", "변경 폐기", Some(error));
                    }
                } else {
                    this.discard_dialog = None;
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
            crate::setup::delete_repository(&target)
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
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        );
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
        decoration: (bool, Option<Div>),
        cx: &Context<Self>,
    ) -> Div {
        let (horizontal, guide) = decoration;
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
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .when_some(guide, |row, guide| row.child(guide))
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
                p.page_error = self.error_history.report(
                    format!("page-{}", p.id),
                    format!(
                        "{} · {} · 이력 추가 조회",
                        p.name,
                        session::display_path(&p.path)
                    ),
                    Some("히스토리 조회 세션이 없습니다. 저장소를 다시 조회해 주세요.".into()),
                );
                self.notify(cx);
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
                        p.page_error = this.error_history.report(
                            format!("page-{}", p.id),
                            format!(
                                "{} · {} · 이력 추가 조회",
                                p.name,
                                session::display_path(&p.path)
                            ),
                            Some(format!("{error:#}")),
                        );
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
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn project(&self) -> Option<&Project> {
        self.active
            .and_then(|id| self.tabs.iter().find(|tab| tab.id() == id))
            .and_then(WorkspaceTab::project)
    }

    fn open_merge_control(
        &mut self,
        action: crate::branches::MergeControl,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let Some(state) = project.snapshot.merge.clone() else {
            return;
        };
        if action == crate::branches::MergeControl::Continue && !state.conflicts.is_empty() {
            return;
        }
        self.branch_dialog = Some(BranchDialog {
            project_id: project.id,
            path: project.path.clone(),
            action: BranchAction::MergeControl { state, action },
            editor: None,
            error: None,
        });
        self.focus.focus(window);
        cx.notify();
    }

    fn open_rebase_control(
        &mut self,
        action: crate::branches::RebaseControl,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let Some(state) = project.snapshot.rebase.clone() else {
            return;
        };
        if action == crate::branches::RebaseControl::Continue && !state.conflicts.is_empty() {
            return;
        }
        self.branch_dialog = Some(BranchDialog {
            project_id: project.id,
            path: project.path.clone(),
            action: BranchAction::RebaseControl { state, action },
            editor: None,
            error: None,
        });
        self.focus.focus(window);
        cx.notify();
    }

    fn open_revert_control(
        &mut self,
        action: crate::branches::RevertControl,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let Some(state) = project.snapshot.revert.clone() else {
            return;
        };
        if action == crate::branches::RevertControl::Continue && !state.conflicts.is_empty() {
            return;
        }
        let no_commit = state.no_commit.unwrap_or(true);
        self.branch_dialog = Some(BranchDialog {
            project_id: project.id,
            path: project.path.clone(),
            action: BranchAction::RevertControl {
                state,
                action,
                no_commit,
            },
            editor: None,
            error: None,
        });
        self.focus.focus(window);
        cx.notify();
    }

    fn open_stash_control(&mut self, cancel: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let Some(state) = project.snapshot.stash_restore.clone() else {
            return;
        };
        if state.applying || (!cancel && !state.conflicts.is_empty()) {
            return;
        }
        self.branch_dialog = Some(BranchDialog {
            project_id: project.id,
            path: project.path.clone(),
            action: BranchAction::StashControl { state, cancel },
            editor: None,
            error: None,
        });
        self.focus.focus(window);
        cx.notify();
    }

    fn open_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending > 0 || self.branch_busy {
            return;
        }
        let Some(project) = self
            .project()
            .filter(|p| self.list_focus == Some(ListFocus::History(p.id)))
        else {
            return;
        };
        let Some(commit) = project.commit.clone() else {
            return;
        };
        self.branch_dialog = Some(BranchDialog {
            project_id: project.id,
            path: project.path.clone(),
            action: BranchAction::Reset {
                commit,
                mode: crate::branches::ResetMode::Soft,
            },
            editor: None,
            error: None,
        });
        self.reset_mode_focus.focus(window);
        cx.notify();
    }

    fn open_history_operation(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending > 0 || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self
            .project()
            .filter(|p| self.list_focus == Some(ListFocus::History(p.id)))
        else {
            return;
        };
        let Some(commit) = project.commit.clone() else {
            return;
        };
        let id = project.id;
        let path = project.path.clone();
        let action = if key == "v" {
            BranchAction::Revert {
                commit: commit.clone(),
                no_commit: self.popup_options.revert_no_commit,
            }
        } else {
            BranchAction::Integrate {
                kind: if key == "m" {
                    crate::branches::IntegrateKind::Merge
                } else {
                    crate::branches::IntegrateKind::Rebase
                },
                commit,
            }
        };
        self.branch_dialog = Some(BranchDialog {
            project_id: id,
            path: path.clone(),
            action,
            editor: None,
            error: None,
        });
        if key == "v" {
            self.branch_checks_focus[0].focus(window);
        } else {
            self.focus.focus(window);
        }
        cx.notify();
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
        let editor = cx.new(|cx| {
            let mut editor = CommitEditor::single_line("", cx);
            editor.text_size = 13.;
            editor
        });
        editor.read(cx).focus.focus(window);
        self.branch_dialog = Some(BranchDialog {
            project_id,
            path,
            action: BranchAction::Create {
                commit,
                remote: self.popup_options.branch_remote,
                checkout: self.popup_options.branch_checkout,
            },
            editor: Some(editor),
            error: None,
        });
        cx.notify();
    }

    fn open_branch_upstream(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() || self.branch_busy {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        if project.ref_tab != RefTab::Local
            || self.list_focus != Some(ListFocus::Branches(project.id))
        {
            return;
        }
        let Some(reference) = project.snapshot.references.iter().find(|r| {
            r.kind == RefTab::Local
                && Some(&r.revision)
                    == project
                        .revision
                        .as_ref()
                        .or(project.snapshot.current_branch.as_ref())
        }) else {
            return;
        };
        let (id, path, reference, expected) = (
            project.id,
            project.path.clone(),
            reference.revision.clone(),
            reference.target.clone(),
        );
        self.branch_busy = true;
        self.pending += 1;
        self.remote_status = Some((id, "연결할 원격 브랜치 확인 중…"));
        self.focus.focus(window);
        let worker_path = path.clone();
        let task = cx.background_executor().spawn(async move {
            crate::branches::plan_upstream(&worker_path, &reference, &expected)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.branch_busy = false;
                this.pending = this.pending.saturating_sub(1);
                this.remote_status = None;
                match result {
                    Ok(plan)
                        if this.active == Some(id)
                            && this.tabs.iter().any(|tab| tab.id() == id) =>
                    {
                        let selected = plan
                            .candidates
                            .iter()
                            .position(|candidate| candidate.reference == plan.current)
                            .unwrap_or(0);
                        this.upstream_scroll = ScrollHandle::new();
                        this.upstream_scroll.scroll_to_item(selected);
                        this.branch_dialog = Some(BranchDialog {
                            project_id: id,
                            path,
                            action: BranchAction::Connect { plan, selected },
                            editor: None,
                            error: None,
                        });
                        this.focus_after_popup = true;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        )
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn open_reference_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() || self.branch_busy {
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let Some(reference) = project.snapshot.references.iter().find(|reference| {
            reference.kind == project.ref_tab
                && Some(&reference.revision) == project.revision.as_ref()
        }) else {
            return;
        };
        let (id, path, reference, expected) = (
            project.id,
            project.path.clone(),
            reference.revision.clone(),
            reference.target.clone(),
        );
        self.branch_busy = true;
        self.pending += 1;
        self.remote_status = Some((id, "삭제할 연결 브랜치 확인 중…"));
        self.focus.focus(window);
        let worker_path = path.clone();
        let task = cx.background_executor().spawn(async move {
            crate::branches::plan_reference_deletion(&worker_path, &reference, &expected)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.branch_busy = false;
                this.pending = this.pending.saturating_sub(1);
                this.remote_status = None;
                match result {
                    Ok(plan)
                        if this.active == Some(id)
                            && this.tabs.iter().any(|tab| tab.id() == id) =>
                    {
                        this.branch_dialog = Some(BranchDialog {
                            project_id: id,
                            path,
                            action: BranchAction::Delete {
                                plan,
                                delete_related: false,
                            },
                            editor: None,
                            error: None,
                        });
                        this.focus_after_popup = true;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        this.error = this.error_history.report(
                            "workspace",
                            "앱 작업",
                            Some(format!("{error:#}")),
                        )
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
        if switched && active == Some(id) {
            self.focus_after_popup = true;
        }
        self.reload(cx);
        self.active = active;
        self.error = self.error_history.report("workspace", "앱 작업", error);
    }

    fn choose_checkout_candidate(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.branch_busy {
            return;
        }
        let Some(dialog) = &self.branch_dialog else {
            return;
        };
        let BranchAction::ChooseCheckout { candidates, .. } = &dialog.action else {
            return;
        };
        let Some(candidate) = candidates.get(index) else {
            return;
        };
        if self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .any(|project| {
                project.id == dialog.project_id
                    && project.snapshot.current_branch.as_deref()
                        == Some(candidate.reference.as_str())
            })
        {
            return;
        }
        if let Some(dialog) = &mut self.branch_dialog
            && let BranchAction::ChooseCheckout { selected, .. } = &mut dialog.action
        {
            *selected = index;
        }
        self.submit_branch(cx);
    }

    fn submit_branch(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.branch_dialog.as_ref().map(|dialog| &dialog.action),
            Some(
                BranchAction::MergeControl { .. }
                    | BranchAction::RebaseControl { .. }
                    | BranchAction::RevertControl { .. }
                    | BranchAction::StashControl { .. }
            )
        ) && self.busy()
        {
            return;
        }
        if self.pending > 0 || self.branch_busy {
            return;
        }
        let Some(dialog) = &mut self.branch_dialog else {
            return;
        };
        if let BranchAction::Connect { plan, selected } = &dialog.action
            && plan.candidates.get(*selected).is_none()
        {
            return;
        }
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
        let switched = matches!(
            action,
            BranchAction::Track(_) | BranchAction::ChooseCheckout { .. }
        );
        let restore_focus = matches!(
            action,
            BranchAction::Create { .. }
                | BranchAction::Tag { .. }
                | BranchAction::Reset { .. }
                | BranchAction::Revert { .. }
                | BranchAction::Integrate { .. }
                | BranchAction::Connect { .. }
                | BranchAction::MergeControl { .. }
                | BranchAction::RebaseControl { .. }
                | BranchAction::RevertControl { .. }
                | BranchAction::StashControl { .. }
        );
        let task = cx.background_executor().spawn(async move {
            match action {
                BranchAction::Create {
                    commit,
                    remote,
                    checkout,
                } => crate::branches::create_options(
                    &path,
                    name.as_deref().unwrap_or_default(),
                    &commit,
                    remote,
                    checkout,
                ),
                BranchAction::Tag { commit } => {
                    crate::branches::create_tag(&path, name.as_deref().unwrap_or_default(), &commit)
                }
                BranchAction::Delete {
                    plan,
                    delete_related,
                } => crate::branches::delete_reference_plan(&path, &plan, delete_related),
                BranchAction::Connect { plan, selected } => {
                    crate::branches::connect_upstream(&path, &plan, selected)
                }
                BranchAction::ChooseCheckout {
                    commit,
                    candidates,
                    selected,
                } => {
                    let candidate = candidates
                        .get(selected)
                        .ok_or_else(|| anyhow::anyhow!("브랜치를 선택하세요."))?;
                    crate::branches::checkout_commit_branch(
                        &path,
                        &candidate.reference,
                        &commit,
                        true,
                    )
                }
                BranchAction::Revert { commit, no_commit } => {
                    crate::branches::revert_commit(&path, &commit, no_commit)
                }
                BranchAction::Integrate { kind, commit } => {
                    crate::branches::integrate_commit(&path, &commit, kind)
                }
                BranchAction::MergeControl { state, action } => {
                    crate::branches::control_merge(&path, &state, action)
                }
                BranchAction::RebaseControl { state, action } => {
                    crate::branches::control_rebase(&path, &state, action)
                }
                BranchAction::RevertControl {
                    state,
                    action,
                    no_commit,
                } => crate::branches::control_revert(&path, &state, action, no_commit),
                BranchAction::StashControl { state, cancel } => {
                    if cancel {
                        feather_git::stash_recovery::cancel(&path, &state)
                    } else {
                        feather_git::stash_recovery::complete(&path, &state)
                    }
                }
                BranchAction::Reset { commit, mode, .. } => {
                    crate::branches::reset_commit(&path, &commit, mode)
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
                        this.focus_after_popup = restore_focus;
                        this.refresh_after_branch(id, switched, None, cx);
                    }
                    Err(error) => {
                        this.refresh_after_branch(id, false, Some(error.to_string()), cx);
                        if let Some(dialog) = &mut this.branch_dialog {
                            dialog.error = this
                                .error_history
                                .mirror("branch-dialog", Some(error.to_string()));
                            if let Some(editor) = &dialog.editor {
                                editor.update(cx, |editor, cx| {
                                    editor.enabled = true;
                                    cx.notify();
                                });
                            }
                        }
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn restore_stash_reference(&mut self, id: u64, commit: String, cx: &mut Context<Self>) {
        if self.pending > 0
            || self.branch_busy
            || self.branch_dialog.is_some()
            || self.stash_dialog.is_some()
        {
            return;
        }
        let Some(project) = self.project().filter(|p| p.id == id) else {
            return;
        };
        let path = project.path.clone();
        if let Some(project) = self.project_mut() {
            project.history_target = None;
        }
        self.pending += 1;
        self.branch_busy = true;
        self.remote_status = Some((id, "스태시 복원 중…"));
        let task = cx
            .background_executor()
            .spawn(async move { crate::setup::restore_stash(&path, &commit) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                this.branch_busy = false;
                this.remote_status = None;
                if result.is_ok()
                    && let Some(project) = this
                        .tabs
                        .iter_mut()
                        .filter_map(WorkspaceTab::project_mut)
                        .find(|p| p.id == id)
                {
                    project.file_tab = FileTab::Stage;
                    project.show_commit_detail = false;
                }
                this.refresh_after_branch(
                    id,
                    false,
                    result.err().map(|error| format!("{error:#}")),
                    cx,
                );
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn checkout_history_commit(&mut self, id: u64, commit: String, cx: &mut Context<Self>) {
        if self.pending > 0 || self.branch_busy || self.branch_dialog.is_some() {
            return;
        }
        let Some(project) = self.project().filter(|p| p.id == id) else {
            return;
        };
        let path = project.path.clone();
        if let Some(project) = self.project_mut() {
            project.history_target = None;
        }
        self.pending += 1;
        self.branch_busy = true;
        self.remote_status = Some((id, "체크아웃 브랜치 확인 중…"));
        let worker_path = path.clone();
        let worker_commit = commit.clone();
        let task = cx.background_executor().spawn(async move {
            let candidates =
                crate::branches::commit_checkout_candidates(&worker_path, &worker_commit)?;
            if candidates.is_empty() {
                crate::branches::checkout_commit(&worker_path, &worker_commit)?;
                Ok((None, true))
            } else if candidates.len() == 1 {
                if crate::branches::is_current_branch(&worker_path, &candidates[0].reference)? {
                    return Ok((None, false));
                }
                crate::branches::checkout_commit_branch(
                    &worker_path,
                    &candidates[0].reference,
                    &worker_commit,
                    true,
                )?;
                Ok((None, true))
            } else {
                Ok::<_, anyhow::Error>((Some(candidates), false))
            }
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.pending = this.pending.saturating_sub(1);
                this.branch_busy = false;
                this.remote_status = None;
                if !this.tabs.iter().any(|tab| tab.id() == id) {
                    return;
                }
                match result {
                    Ok((Some(candidates), _)) => {
                        this.branch_dialog = Some(BranchDialog {
                            project_id: id,
                            path,
                            action: BranchAction::ChooseCheckout {
                                commit,
                                candidates,
                                selected: 0,
                            },
                            editor: None,
                            error: None,
                        });
                    }
                    Ok((None, true)) => this.refresh_after_branch(id, true, None, cx),
                    Ok((None, false)) => {}
                    Err(error) => {
                        this.refresh_after_branch(id, false, Some(format!("{error:#}")), cx)
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
        if project.snapshot.current_branch.as_deref() == Some(reference.as_str()) {
            return;
        }
        let path = project.path.clone();
        if let Some(project) = self.project_mut() {
            project.history_target = None;
        }
        let worker_path = path.clone();
        self.pending += 1;
        self.branch_busy = true;
        self.remote_status = Some((id, "브랜치 전환 중…"));
        let task = cx.background_executor().spawn(async move {
            if !reference.starts_with("refs/") {
                crate::branches::checkout_commit(&worker_path, &reference)?;
                return Ok(None);
            }
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
            || self.stash_dialog.is_some()
            || self.delete_target.is_some()
            || self.settings.is_some()
            || self.branch_dialog.is_some()
            || self.branch_busy
        {
            return;
        }
        let path = self.project().map(|project| project.path.clone());
        self.help = false;
        let settings = cx.new(|cx| SettingsPanel::new(path, cx));
        cx.observe(&settings, |this, _, cx| this.notify(cx))
            .detach();
        self.settings = Some(settings);
        self.focus.focus(window);
        cx.notify();
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(settings) = self.settings.clone()
            && settings.update(cx, |settings, cx| settings.close_child_popup(window, cx))
        {
            return;
        }
        if let Some(settings) = &self.settings {
            if !settings.read(cx).can_close(cx) {
                return;
            }
            settings.read(cx).cancel();
        }
        self.persist();
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
            .child(crate::shortcut::hint(
                "세팅",
                &self.shortcuts.label("settings"),
            ))
            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
    }

    fn activate_project_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id() == id) else {
            return;
        };
        self.active = Some(id);
        self.search_tab = false;
        self.search_generation += 1;
        self.search_loading = false;
        self.search_results.clear();
        self.ensure_active_loaded(cx);
        self.list_focus = None;
        self.text_selection = None;
        self.focus.focus(window);
        self.tabs_scroll.scroll_to_item(index);
        self.persist();
        self.notify(cx);
    }

    fn add_empty_tab(&mut self) {
        self.search_tab = false;
        self.search_generation += 1;
        self.search_loading = false;
        self.search_results.clear();
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(WorkspaceTab::Empty(id));
        self.start_pages.insert(id, StartPage::default());
        self.active = Some(id);
        self.error = None;
        self.tabs_scroll.scroll_to_item(self.tabs.len() - 1);
        self.persist();
    }

    fn activate_start_action(
        &mut self,
        id: u64,
        mode: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.start_pages.get(&id).is_none_or(|page| page.busy) {
            return;
        }
        if mode == 2 {
            let page = self.start_pages.get_mut(&id).unwrap();
            page.clone_form = true;
            let editor = page
                .url_editor
                .get_or_insert_with(|| cx.new(|cx| CommitEditor::single_line("", cx)));
            editor.read(cx).focus.focus(window);
            cx.notify();
        } else {
            self.start_repository(id, mode, cx);
        }
    }

    fn start_focus_order(&self, cx: &Context<Self>) -> Vec<FocusHandle> {
        let mut order = self.start_action_focus[..3].to_vec();
        if let Some(page) = self.active.and_then(|id| self.start_pages.get(&id))
            && page.clone_form
        {
            if let Some(editor) = &page.url_editor {
                order.push(editor.read(cx).focus.clone());
            }
            order.push(self.start_action_focus[3].clone());
        }
        for path in &self.recent {
            if let Some(handles) = self.recent_action_focus.get(path) {
                order.extend(handles.iter().cloned());
            }
        }
        order
    }

    fn focus_style(button: Stateful<Div>, focused: bool) -> Stateful<Div> {
        button.when(focused, |button| {
            button.border_1().border_color(rgb(ACCENT)).bg(rgb(HOVER))
        })
    }

    fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        self.list_scroll.retain(|(project, _), _| *project != id);
        self.tracks.retain(|((project, _), _), _| *project != id);
        self.file_items.retain(|(project, _), _| *project != id);
        if self
            .selected_folder
            .as_ref()
            .is_some_and(|(key, _)| key.0 == id)
        {
            self.selected_folder = None;
        }
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
            self.search_tab = false;
            self.search_generation += 1;
            self.search_loading = false;
            self.search_results.clear();
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

    fn resize_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_in_progress()
            || self.help
            || self.settings.is_some()
            || self.branch_dialog.is_some()
            || self.branch_busy
            || self.delete_target.is_some()
            || self.commit_dialog.is_some()
            || self.discard_dialog.is_some()
            || self.stash_dialog.is_some()
        {
            self.resize_drag = None;
            self.column_drag = None;
            self.scroll_drag = None;
            self.text_drag = false;
            return;
        }
        if self.text_drag {
            if event.pressed_button != Some(MouseButton::Left) {
                self.text_drag = false;
                return;
            }
            if let Some(key) = self.text_selection.as_ref().map(|(key, _)| *key)
                && self.active == Some(key.0)
                && let Some(viewport) = self.viewports.get(&key).copied()
                && viewport.size.width > px(0.)
                && viewport.size.height > px(0.)
                && let Some(handle) = self.list_scroll.get(&key)
            {
                let base = handle.0.borrow().base_handle.clone();
                let offset = base.offset();
                let max = base.max_offset();
                let delta = |position: Pixels, start: Pixels, length: Pixels| {
                    if position < start {
                        (start - position).min(px(44.))
                    } else if position > start + length {
                        -(position - start - length).min(px(44.))
                    } else {
                        px(0.)
                    }
                };
                let next = point(
                    (offset.x + delta(event.position.x, viewport.origin.x, viewport.size.width))
                        .clamp(-max.width, px(0.)),
                    (offset.y + delta(event.position.y, viewport.origin.y, viewport.size.height))
                        .clamp(-max.height, px(0.)),
                );
                if next != offset {
                    base.set_offset(next);
                    cx.notify();
                }
                let offset = base.offset();
                let count = if key.1 == 31 {
                    self.detail_items
                        .get(&key.0)
                        .map_or(0, |rows| rows.lines.len())
                } else {
                    self.project().map_or(0, |project| project.diff.lines.len())
                };
                if count > 0 {
                    let row = drag_row(
                        event.position.y.into(),
                        viewport.origin.y.into(),
                        viewport.size.height.into(),
                        offset.y.into(),
                        count,
                    );
                    let raw = if key.1 == 31 {
                        self.detail_items
                            .get(&key.0)
                            .and_then(|rows| rows.lines.get(row))
                            .cloned()
                    } else {
                        self.project()
                            .and_then(|project| project.diff.lines.get(row))
                            .map(|line| line.text.clone())
                    };
                    if let Some(raw) = raw {
                        let left = if key.1 == 31 {
                            12.
                        } else {
                            116. + if self.project().is_some_and(|p| {
                                p.file_tab == FileTab::Stage && p.diff.partial.is_some()
                            }) {
                                24.
                            } else {
                                0.
                            }
                        };
                        let x = event.position.x.clamp(
                            viewport.origin.x,
                            viewport.origin.x + viewport.size.width - px(0.01),
                        ) - viewport.origin.x
                            - px(left)
                            - offset.x;
                        let displayed = raw.replace('\t', "    ");
                        let shaped = shape_styled_line(
                            window,
                            &displayed,
                            TEXT,
                            if key.1 == 31 {
                                detail_label_len(&raw)
                            } else {
                                0
                            },
                        );
                        let byte = shaped.unwrapped_layout.closest_index_for_x(x.max(px(0.)));
                        if let Some((selected_key, selection)) = &mut self.text_selection
                            && *selected_key == key
                        {
                            selection.cursor = Position {
                                row,
                                column: original_column(&raw, byte),
                            };
                            cx.notify();
                        }
                    }
                }
            }
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
        if event.button == MouseButton::Left {
            self.text_drag = false;
        }
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
                    this.notify(cx);
                    return;
                };
                page.busy = false;
                match result {
                    Ok(path) => {
                        this.finish_open(source, path, cx);
                    }
                    Err(err) => {
                        page.error = this.error_history.report(
                            format!("start-{source}"),
                            "저장소 시작",
                            Some(format!("{err:#}")),
                        )
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
        let source = (mode == 2).then(|| {
            page.url_editor
                .as_ref()
                .map(|editor| editor.read(cx).message().trim().to_owned())
                .unwrap_or_default()
        });
        if source.as_ref().is_some_and(|s| s.is_empty()) {
            page.error = self.error_history.report(
                format!("start-{id}"),
                "저장소 시작",
                Some("저장소 URL을 입력하세요.".into()),
            );
            self.notify(cx);
            return;
        }
        page.busy = true;
        if let Some(editor) = &page.url_editor {
            editor.update(cx, |editor, cx| {
                editor.enabled = false;
                cx.notify();
            });
        }
        page.error = None;
        let selected = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(
                match mode {
                    0 => "신규 저장소 디렉터리 선택",
                    2 => "Clone 대상 디렉터리 선택",
                    _ => "저장소 열기",
                }
                .into(),
            ),
        });
        cx.spawn(async move |this, cx| {
            let selected = selected
                .await
                .map_err(anyhow::Error::from)
                .and_then(|r| r)
                .map(|paths| paths.and_then(|p| p.into_iter().next()));
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
                                if mode == 0 {
                                    crate::setup::init_existing_directory(&path)?;
                                } else if mode == 2 {
                                    crate::setup::clone_into_existing_directory(
                                        &path,
                                        source.as_deref().unwrap_or_default(),
                                    )?;
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
                if let Some(editor) = &page.url_editor {
                    editor.update(cx, |editor, cx| {
                        editor.enabled = true;
                        cx.notify();
                    });
                }
                match result {
                    Some(Ok(path)) => {
                        this.finish_open(id, path, cx);
                    }
                    Some(Err(error)) => {
                        page.error = this.error_history.report(
                            format!("start-{id}"),
                            "저장소 시작",
                            Some(format!("{error:#}")),
                        )
                    }
                    None => {}
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn start_page(&self, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
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
            content = content.child(
                Self::focus_style(
                    button(("start", mode as usize), label, false)
                        .track_focus(&self.start_action_focus[mode as usize]),
                    self.start_action_focus[mode as usize].is_focused(window),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.start_action_focus[mode as usize].focus(window);
                    this.activate_start_action(id, mode, window, cx);
                })),
            );
        }
        if page.clone_form {
            content = content
                .child(div().text_sm().child("저장소 URL"))
                .when_some(page.url_editor.clone(), |content, editor| {
                    content.child(
                        div()
                            .id("clone-url")
                            .h(px(36.))
                            .p_1()
                            .border_1()
                            .border_color(rgb(LINE))
                            .overflow_hidden()
                            .child(editor),
                    )
                })
                .child(
                    Self::focus_style(
                        self.shortcut_button("execute-clone", "Clone 실행", "confirm", true)
                            .track_focus(&self.start_action_focus[3]),
                        self.start_action_focus[3].is_focused(window),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.start_action_focus[3].focus(window);
                        this.start_repository(id, 2, cx)
                    })),
                );
        }
        if page.busy {
            content = content.child(empty("처리 중입니다…"));
        }
        if let Some(error) = &page.error {
            content = content.child(div().text_sm().text_color(rgb(ERROR)).child(error.clone()));
        }
        let key = (id, 40);
        let scroll = page.recent_scroll.clone();
        let count = self.recent.len();
        let list = uniform_list(
            ("recent-list", id as usize),
            count,
            cx.processor(|this, range: std::ops::Range<usize>, window, cx| {
                range
                    .map(|index| this.recent_row(index, window, cx))
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
                (false, None),
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

    fn recent_row(&self, index: usize, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
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
        let focus = self.recent_action_focus.get(path);
        row.cursor_pointer()
            .when_some(focus, |row, focus| row.track_focus(&focus[0]))
            .when(
                focus.is_some_and(|focus| focus[0].is_focused(window)),
                |row| row.bg(rgb(HOVER)).border_l_2().border_color(rgb(ACCENT)),
            )
            .hover(|row| row.bg(rgb(HOVER)))
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
                    .when_some(focus, |button, focus| button.track_focus(&focus[1]))
                    .when(
                        focus.is_some_and(|focus| focus[1].is_focused(window)),
                        |button| {
                            button
                                .bg(rgb(DANGER_HOVER))
                                .border_1()
                                .border_color(rgb(ACCENT))
                        },
                    )
                    .size(px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .hover(|icon| icon.bg(rgb(DANGER_HOVER)))
                    .child(trash_icon())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        if let Some(focus) = this.recent_action_focus.get(&delete_path) {
                            focus[1].focus(window);
                        }
                        this.delete_target = Some(delete_path.clone());
                        this.delete_confirm = false;
                        this.focus.focus(window);
                        cx.notify();
                    })),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(focus) = this.recent_action_focus.get(&open_path) {
                    focus[0].focus(window);
                }
                this.open(open_path.clone(), cx)
            }))
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
            .spawn(async move { git::load_for_refresh(&path, commit.as_deref()) });
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
                        let entering =
                            snapshot.integration_active() && !p.snapshot.integration_active();
                        let advanced = snapshot
                            .rebase
                            .as_ref()
                            .zip(p.snapshot.rebase.as_ref())
                            .is_some_and(|(next, previous)| !next.same_operation(previous))
                            || snapshot
                                .revert
                                .as_ref()
                                .zip(p.snapshot.revert.as_ref())
                                .is_some_and(|(next, previous)| !next.same_operation(previous));
                        let integration_dialog_failed =
                            this.branch_dialog.as_ref().is_some_and(|dialog| {
                                dialog.project_id == id
                                    && match &dialog.action {
                                        BranchAction::Integrate {
                                            kind: crate::branches::IntegrateKind::Merge,
                                            ..
                                        } => snapshot.merge.is_some(),
                                        BranchAction::Integrate {
                                            kind: crate::branches::IntegrateKind::Rebase,
                                            ..
                                        } => snapshot.rebase.is_some(),
                                        BranchAction::Revert { .. } => snapshot
                                            .revert
                                            .as_ref()
                                            .is_some_and(|state| !state.conflicts.is_empty()),
                                        _ => false,
                                    }
                            });
                        if entering || integration_dialog_failed || advanced {
                            p.file_tab = FileTab::Stage;
                            p.show_commit_detail = false;
                            if this.active == Some(id) {
                                this.list_focus = Some(ListFocus::Files(id));
                                this.selected_folder = None;
                                this.focus_after_popup = true;
                            }
                        }
                        if integration_dialog_failed {
                            this.branch_dialog = None;
                            this.error = None;
                        }
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
                        let stage = display_file_rows(&snapshot.stage, this.file_tree);
                        let history = display_file_rows(&snapshot.history, this.file_tree);
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
                        p.error = this.error_history.report(
                            format!("project-{}", p.id),
                            format!("{} · {}", p.name, session::display_path(&p.path)),
                            Some(format!("{err:#}")),
                        );
                        p.snapshot = Snapshot::default();
                        p.commit = None;
                        this.file_items.remove(&(id, 20));
                        this.file_items.remove(&(id, 21));
                        this.detail_items.remove(&id);
                        this.graphs.remove(&id);
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn select_commit(&mut self, commit: String, cx: &mut Context<Self>) {
        self.selected_folder = None;
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
                        this.file_items.insert(
                            (id, 21),
                            display_file_rows(&selection.history, this.file_tree),
                        );
                        this.detail_items
                            .insert(id, TextRows::new(detail_rows(&selection.detail)));
                        p.snapshot.history = selection.history;
                        p.snapshot.commit_detail = Some(selection.detail);
                    }
                    Err(error) => {
                        p.error = this.error_history.report(
                            format!("project-{}", p.id),
                            format!("{} · {}", p.name, session::display_path(&p.path)),
                            Some(format!("{error:#}")),
                        )
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
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
        if p.snapshot.references.iter().any(|reference| {
            reference.kind == RefTab::Stash && Some(&reference.revision) == p.revision.as_ref()
        }) {
            p.history_target = None;
            self.notify(cx);
            return;
        }
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
                            && this.stash_dialog.is_none()
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
        self.notify(cx);
    }

    fn select_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(id) = self.active else {
            return;
        };
        self.select_project_file(id, index, cx);
    }

    fn select_project_file(&mut self, project_id: u64, index: usize, cx: &mut Context<Self>) {
        self.partial_selection.remove(&project_id);
        if self
            .selected_folder
            .as_ref()
            .is_some_and(|(key, _)| key.0 == project_id)
        {
            self.selected_folder = None;
        }
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
                        if this.active == Some(id)
                            && this.list_focus == Some(ListFocus::Text((id, 30)))
                        {
                            this.text_selection = p.diff.lines.first().map(|_| {
                                let row = 0;
                                let position = Position { row, column: 0 };
                                (
                                    (id, 30),
                                    Selection {
                                        anchor: position,
                                        cursor: position,
                                    },
                                )
                            });
                        }
                    }
                    Err(err) => {
                        p.error = this.error_history.report(
                            format!("project-{}", p.id),
                            format!("{} · {}", p.name, session::display_path(&p.path)),
                            Some(format!("{err:#}")),
                        )
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn show_detail(&mut self) {
        self.selected_folder = None;
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

    fn switch_reference_tab(&mut self, tab: RefTab, window: &mut Window, cx: &mut Context<Self>) {
        self.search_tab = false;
        if let Some(project) = self.project_mut() {
            project.ref_tab = tab;
            self.list_focus = Some(ListFocus::Branches(project.id));
            self.focus.focus(window);
            cx.notify();
        }
    }

    fn switch_file_tab(&mut self, tab: FileTab, window: &mut Window, cx: &mut Context<Self>) {
        self.text_selection = None;
        self.selected_folder = None;
        if let Some(project) = self.project_mut() {
            project.file_tab = tab;
            project.stage_selection.clear();
            project.show_commit_detail = tab == FileTab::History;
            if tab == FileTab::History && project.snapshot.commit_detail.is_some() {
                project.error = None;
            }
            project.selected_file = None;
            project.diff = Diff::default();
            project.diff_generation += 1;
            project.diff_loading = false;
            self.list_focus = Some(ListFocus::Files(project.id));
            self.focus.focus(window);
            self.notify(cx);
        }
    }

    fn cycle_panel_tab(&mut self, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project() else {
            return;
        };
        match self.list_focus {
            Some(ListFocus::Branches(id)) if id == project.id => {
                let index = adjacent_reference_tab(project.ref_tab, self.search_tab, next);
                if index == RefTab::ALL.len() {
                    self.search_tab = true;
                    self.search_selected = SEARCH_VIRTUAL_ROW;
                    self.list_focus = Some(ListFocus::Branches(id));
                    self.focus.focus(window);
                    self.notify(cx);
                } else {
                    self.switch_reference_tab(RefTab::ALL[index], window, cx);
                }
            }
            Some(ListFocus::Files(id)) if id == project.id => {
                let tabs = [FileTab::Stage, FileTab::History];
                let index = adjacent_tab(
                    usize::from(project.file_tab == FileTab::History),
                    tabs.len(),
                    next,
                )
                .unwrap();
                self.switch_file_tab(tabs[index], window, cx);
            }
            _ => {}
        }
    }

    fn run_search(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project() else {
            return;
        };
        let root = project.path.clone();
        let query = self.search_editor.read(cx).message().to_owned();
        self.search_generation += 1;
        let generation = self.search_generation;
        self.search_loading = true;
        self.search_results.clear();
        self.search_selected = SEARCH_VIRTUAL_ROW;
        let task = cx
            .background_executor()
            .spawn(async move { feather_git::git_search::search(&root, &query) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.search_generation != generation {
                    return;
                }
                this.search_loading = false;
                match result {
                    Ok(results) => this.search_results = results,
                    Err(error) => {
                        this.error =
                            this.error_history
                                .report("search", "검색", Some(error.to_string()))
                    }
                }
                this.notify(cx);
            })
            .ok();
        })
        .detach();
        self.notify(cx);
    }

    fn activate_search_result(&mut self, cx: &mut Context<Self>) {
        let Some(result) = self.search_results.get(self.search_selected).cloned() else {
            return;
        };
        match result {
            feather_git::git_search::Match::Commit { id, .. } => {
                self.select_commit(id.clone(), cx);
                let mut load_more = None;
                let mut visible_index = None;
                if let Some(project) = self.project_mut() {
                    visible_index = project
                        .snapshot
                        .commits
                        .iter()
                        .position(|commit| commit.id == id);
                    if visible_index.is_none() {
                        load_more = Some(project.id);
                        project.history_target = Some(id);
                    }
                }
                if let Some(index) = visible_index
                    && let Some(project_id) = self.active
                {
                    self.history_scroll
                        .entry(project_id)
                        .or_default()
                        .scroll_to_item(index, ScrollStrategy::Center);
                }
                if let Some(project_id) = load_more {
                    self.load_more(project_id, cx);
                }
                self.notify(cx);
            }
            feather_git::git_search::Match::File { path, .. } => self.open_search_file(path, cx),
        }
    }

    fn open_search_file(&mut self, path: String, cx: &mut Context<Self>) {
        if let Some(handle) = self.file_window {
            handle
                .update(cx, |_, child, _| child.activate_window())
                .ok();
            return;
        }
        let Some(project) = self.project() else {
            return;
        };
        let root = project.path.clone();
        let title = format!("{} · 파일 히스토리", path);
        let shortcuts = self.shortcuts.clone();
        let bounds = Bounds::centered(None, size(px(960.), px(640.)), cx);
        match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(720.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| crate::search_window::FileHistoryWindow::new(root, path, shortcuts, cx))
            },
        ) {
            Ok(handle) => {
                let child: AnyWindowHandle = handle.into();
                self.file_window = Some(child);
                let parent = cx.weak_entity();
                cx.on_window_closed(move |cx| {
                    if !cx.windows().contains(&child) {
                        parent
                            .update(cx, |this, cx| {
                                if this.file_window == Some(child) {
                                    this.file_window = None;
                                    this.notify(cx);
                                }
                            })
                            .ok();
                    }
                })
                .detach();
                self.notify(cx);
            }
            Err(error) => {
                self.error =
                    self.error_history
                        .report("search", "파일 히스토리", Some(error.to_string()));
                self.notify(cx);
            }
        }
    }

    fn search_panel(&self, window: &Window, cx: &Context<Self>) -> Div {
        let mut results = div()
            .id("search-results")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .on_click(cx.listener(|this, _, window, cx| {
                if let Some(project) = this.project() {
                    this.list_focus = Some(ListFocus::Branches(project.id));
                    this.focus.focus(window);
                    this.notify(cx);
                }
            }));
        for (index, result) in self.search_results.iter().enumerate() {
            let label = match result {
                feather_git::git_search::Match::Commit { id, title } => {
                    format!("{}  {}", &id[..8], title)
                }
                feather_git::git_search::Match::File { path, content } => {
                    format!("{}{}", path, if *content { " · 내용 일치" } else { "" })
                }
            };
            let file = matches!(result, feather_git::git_search::Match::File { .. });
            let path = if let feather_git::git_search::Match::File { path, .. } = result {
                path.clone()
            } else {
                String::new()
            };
            results = results.child(
                div()
                    .id(("search-result", index))
                    .h(px(history_view::ROW_HEIGHT))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .text_xs()
                    .cursor_pointer()
                    .bg(rgb(if self.search_selected == index {
                        if self.list_focus == self.project().map(|p| ListFocus::Branches(p.id))
                            && self.focus.is_focused(window)
                        {
                            SELECT
                        } else {
                            SELECT_INACTIVE
                        }
                    } else {
                        PANEL
                    }))
                    .when(self.search_selected != index, |row| {
                        row.hover(|row| row.bg(rgb(HISTORY_HOVER)))
                    })
                    .child(
                        div()
                            .w(px(18.))
                            .h(px(18.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(rgb(if file { TAG_BG } else { LOCAL_BG }))
                            .text_color(rgb(if file { TAG_TEXT } else { ADDED }))
                            .child(if file { "F" } else { "C" }),
                    )
                    .child(div().min_w_0().text_ellipsis().child(label))
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        this.search_selected = index;
                        this.list_focus = this.project().map(|p| ListFocus::Branches(p.id));
                        this.focus.focus(window);
                        if file && event.click_count() == 2 {
                            this.open_search_file(path.clone(), cx);
                        }
                        this.notify(cx);
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(
                        div()
                            .h(px(30.))
                            .px_2()
                            .flex()
                            .items_center()
                            .border_1()
                            .border_color(rgb(LINE))
                            .child(self.search_editor.clone()),
                    ),
            )
            .when(self.search_loading, |body| {
                body.child(div().px_3().child("검색 중…"))
            })
            .child(results)
    }

    fn branch_panel(&mut self, window: &Window, cx: &Context<Self>) -> Div {
        let mut tabs = div()
            .id("reference-tabs")
            .border_b_1()
            .border_color(rgb(LINE))
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
                    !self.search_tab && self.project().is_some_and(|p| p.ref_tab == tab),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.switch_reference_tab(tab, window, cx);
                })),
            );
        }
        tabs = tabs.child(
            button("ref-tab-search", "Search", self.search_tab).on_click(cx.listener(
                |this, _, window, cx| {
                    this.search_tab = true;
                    this.search_selected = SEARCH_VIRTUAL_ROW;
                    this.list_focus = this
                        .project()
                        .map(|project| ListFocus::Branches(project.id));
                    this.focus.focus(window);
                    this.notify(cx);
                },
            )),
        );
        if self.search_tab {
            return panel().child(tabs).child(self.search_panel(window, cx));
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
            .when(count == 0, |p| {
                p.child(empty("표시할 항목이 없습니다.").px_3().py_2())
            })
            .child(
                self.scroll_frame(
                    key,
                    scroll,
                    list,
                    size(0., count as f32 * history_view::ROW_HEIGHT),
                    (false, None),
                    cx,
                )
                .id(("branch-frame", id as usize))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.active == Some(id) {
                        this.list_focus = Some(ListFocus::Branches(id));
                        this.focus.focus(window);
                        this.notify(cx);
                    }
                })),
            )
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
            .text_xs();
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
            .when(!active, |row| row.hover(|s| s.bg(rgb(HOVER))))
            .gap_2()
            .child(
                div()
                    .size(px(18.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(current, |icon| icon.child(current_branch_dot(18.)))
                    .when(!current, |icon| {
                        icon.child(div().text_size(px(18.)).line_height(px(18.)).child("⑂"))
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .text_ellipsis()
                    .child(reference.name.clone()),
            );
        row.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            if this.active != Some(id) {
                return;
            }
            this.list_focus = Some(ListFocus::Branches(id));
            this.focus.focus(window);
            if event.click_count() == 2 && kind == RefTab::Stash.index() {
                this.restore_stash_reference(id, revision.clone(), cx);
            } else if event.click_count() == 2 && kind == RefTab::Tag.index() {
                this.checkout_reference(id, target.clone(), cx);
            } else if event.click_count() == 2 && kind < 2 {
                this.checkout_reference(id, revision.clone(), cx);
            } else if event.click_count() == 1 {
                if let Some(project) = this.project_mut() {
                    project.revision = Some(revision.clone());
                    project.history_target = None;
                }
                cx.notify();
            }
        }))
    }

    fn history_row(
        &mut self,
        project_id: u64,
        index: usize,
        layout: ([f32; 5], usize),
        window: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let (widths, graph_lanes) = layout;
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
            .when(
                p.snapshot.head_commit.as_deref() == Some(commit.id.as_str()),
                |row| row.font_weight(FontWeight::BOLD),
            )
            .bg(rgb(if selected { selection_color } else { PANEL }))
            .when(!selected, |row| row.hover(|s| s.bg(rgb(HISTORY_HOVER))))
            .child(history_view::cell(0, widths[0]).child(graph.render(
                p.snapshot.head_commit.as_deref() == Some(commit.id.as_str()),
                graph_lanes,
            )));
        let mut description = div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .overflow_hidden();
        for tag in &commit.tags {
            description = description.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(11.))
                    .font_weight(FontWeight::NORMAL)
                    .h(px(18.))
                    .flex_shrink_0()
                    .px_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(TAG_TEXT))
                    .bg(rgb(TAG_BG))
                    .text_color(rgb(TAG_TEXT))
                    .child(reference_badge_icon(true, TAG_TEXT))
                    .child(tag.clone()),
            );
        }
        for branch in &commit.branches {
            let remote = p.snapshot.references.iter().any(|reference| {
                reference.kind == RefTab::Remote
                    && reference.name == *branch
                    && reference.target == commit.id
            });
            let tint = if remote { REMOTE_TEXT } else { ACCENT };
            description = description.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(11.))
                    .font_weight(FontWeight::NORMAL)
                    .h(px(18.))
                    .flex_shrink_0()
                    .px_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(tint))
                    .bg(rgb(if remote { REMOTE_BG } else { LOCAL_BG }))
                    .text_color(rgb(tint))
                    .child(reference_badge_icon(false, tint))
                    .when(
                        p.snapshot.current_branch.as_deref()
                            == Some(format!("refs/heads/{branch}").as_str()),
                        |badge| badge.child(current_branch_dot(12.)),
                    )
                    .child(branch.clone()),
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
        row.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            if this.active == Some(project_id) {
                this.list_focus = Some(ListFocus::History(project_id));
                this.focus.focus(window);
                if event.click_count() == 2 {
                    this.checkout_history_commit(project_id, id.clone(), cx);
                } else if event.click_count() == 1 {
                    this.select_commit(id.clone(), cx);
                }
            }
        }))
    }

    fn history_panel(&mut self, cx: &Context<Self>) -> Div {
        let (project_id, count) = self
            .project()
            .map(|p| (p.id, p.snapshot.commits.len()))
            .unwrap_or_default();
        let graph_lanes = self
            .project()
            .map(|p| {
                p.snapshot
                    .commits
                    .iter()
                    .map(|commit| {
                        commit
                            .input_edges
                            .iter()
                            .chain(&commit.output_edges)
                            .flat_map(|edge| [edge.from_lane, edge.to_lane])
                            .chain(std::iter::once(commit.marker.lane))
                            .max()
                            .unwrap_or(0)
                    })
                    .max()
                    .unwrap_or(0)
                    + 1
            })
            .unwrap_or(1);
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
                    .when(i != 0, |cell| cell.border_l_1().border_color(rgb(LINE)))
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
                                        this.notify(cx);
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
                    .map(|index| {
                        this.history_row(project_id, index, (widths, graph_lanes), window, cx)
                    })
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
                                        this.notify(cx);
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
                    .child(
                        self.scroll_frame(
                            (project_id, 0),
                            scroll,
                            list,
                            size(0., count as f32 * history_view::ROW_HEIGHT),
                            (false, None),
                            cx,
                        )
                        .id(("history-frame", project_id as usize))
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                if this.active == Some(project_id) {
                                    this.list_focus = Some(ListFocus::History(project_id));
                                    this.focus.focus(window);
                                    this.notify(cx);
                                }
                            },
                        )),
                    )
                    .when(page_loading, |grid| grid.child(page_progress())),
            )
    }

    fn select_file_folder(&mut self, key: ListKey, path: Vec<u8>) {
        self.selected_folder = Some((key, path));
        self.text_selection = None;
        if let Some(project) = self.project_mut() {
            project.stage_selection.clear();
            project.selected_file = None;
            project.show_commit_detail = false;
            project.diff = Diff::default();
            project.diff_generation += 1;
            project.diff_loading = false;
        }
    }

    fn rebuild_file_rows(&mut self, reset_scroll: bool) {
        for project in self.tabs.iter().filter_map(WorkspaceTab::project) {
            for (kind, files) in [
                (20, &project.snapshot.stage),
                (21, &project.snapshot.history),
            ] {
                let key = (project.id, kind);
                self.file_items
                    .insert(key, display_file_rows(files, self.file_tree));
            }
        }
        for (key, scroll) in &self.list_scroll {
            if reset_scroll && [20, 21].contains(&key.1) {
                scroll
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.), px(0.)));
            }
        }
    }

    fn files_panel(&mut self, window: &Window, cx: &Context<Self>) -> Div {
        let mut tabs = div()
            .flex()
            .gap_1()
            .px_3()
            .py_2()
            .flex_shrink_0()
            .border_b_1()
            .border_color(rgb(LINE));
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
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.switch_file_tab(tab, window, cx);
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
            .when(!history && self.project().is_some_and(|project| project.snapshot.integration_active()), |panel| {
                panel.child(div().px_3().py_2().text_xs().text_color(rgb(MUTED))
                    .child("DiffViewer에서 충돌 구간별 내용을 선택하거나 외부 편집기로 수정한 뒤 Space로 Stage하세요. 삭제로 해결할 수도 있습니다."))
            })
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
                        .when(!detail, |row| row.hover(|s| s.bg(rgb(HOVER))))
                        .text_xs()
                        .h(px(history_view::ROW_HEIGHT))
                        .w_full()
                        .px_3()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .rounded_none()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .gap_1()
                        .child(commit_icon())
                        .child("커밋상세")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.list_focus = Some(ListFocus::Files(id));
                            this.focus.focus(window);
                            this.show_detail();
                            this.notify(cx);
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
            .child(
                self.scroll_frame(
                    key,
                    scroll,
                    list,
                    size(0., count as f32 * history_view::ROW_HEIGHT),
                    (false, None),
                    cx,
                )
                .id(("file-frame", key.1 as usize))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.active == Some(id) {
                        this.list_focus = Some(ListFocus::Files(id));
                        this.focus.focus(window);
                        this.notify(cx);
                    }
                })),
            )
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
        if let FileRow::Folder { path } = item {
            let depth = path.iter().filter(|byte| **byte == b'/').count();
            let label = String::from_utf8_lossy(
                path.rsplit(|byte| *byte == b'/').next().unwrap_or_default(),
            )
            .into_owned();
            let path = path.clone();
            let selected =
                self.selected_folder
                    .as_ref()
                    .is_some_and(|(selected_key, selected_path)| {
                        *selected_key == key && *selected_path == path
                    });
            return row
                .pl(px(12. + depth as f32 * 16.))
                .text_color(rgb(MUTED))
                .bg(rgb(if selected {
                    self.selection_color(ListFocus::Files(key.0), window)
                } else {
                    PANEL
                }))
                .cursor_pointer()
                .when(!selected, |row| row.hover(|style| style.bg(rgb(HOVER))))
                .child(label)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.active != Some(key.0) {
                        return;
                    }
                    this.list_focus = Some(ListFocus::Files(key.0));
                    this.focus.focus(window);
                    this.select_file_folder(key, path.clone());
                    cx.notify();
                }));
        }
        let FileRow::File(file_index) = item.clone() else {
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
        if self.file_tree {
            let depth = file.path.iter().filter(|byte| **byte == b'/').count();
            row = row.pl(px(12. + depth as f32 * 16.));
        }
        let selected = (key.1 == 20 && p.stage_selection.contains(file))
            || (key.1 != 20 && p.selected_file == Some(file_index));
        row = row
            .bg(rgb(if selected {
                self.selection_color(ListFocus::Files(key.0), window)
            } else {
                PANEL
            }))
            .cursor_pointer()
            .when(!selected, |row| row.hover(|s| s.bg(rgb(HOVER))))
            .child(
                div()
                    .text_xs()
                    .flex_shrink_0()
                    .when(key.1 == 20, |status| {
                        let label = file.stage_status();
                        let mut chars = label.chars();
                        let left = chars
                            .next()
                            .filter(|c| *c != ' ')
                            .map_or_else(String::new, |c| c.to_string());
                        let right = chars
                            .next()
                            .filter(|c| *c != ' ')
                            .map_or_else(String::new, |c| c.to_string());
                        status
                            .w(px(18.))
                            .h_full()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(9.))
                                    .h_full()
                                    .flex_shrink_0()
                                    .line_height(px(history_view::ROW_HEIGHT))
                                    .text_center()
                                    .child(left),
                            )
                            .child(
                                div()
                                    .w(px(9.))
                                    .h_full()
                                    .flex_shrink_0()
                                    .line_height(px(history_view::ROW_HEIGHT))
                                    .text_center()
                                    .child(right),
                            )
                    })
                    .text_color(rgb(if file.status == "U" {
                        ERROR
                    } else if key.1 == 20 {
                        if file.staged { ADDED } else { REMOVED }
                    } else {
                        match file.status.as_str() {
                            "A" => ADDED,
                            "D" => REMOVED,
                            _ => MODIFIED,
                        }
                    }))
                    .when(key.1 != 20, |status| status.child(file.status.clone())),
            )
            .child(
                div()
                    .min_w_0()
                    .text_ellipsis()
                    .text_xs()
                    .h_full()
                    .line_height(px(history_view::ROW_HEIGHT))
                    .when(key.1 == 20 && file.staged, |name| {
                        name.text_color(rgb(ADDED))
                    })
                    .child(if self.file_tree {
                        String::from_utf8_lossy(
                            file.path
                                .rsplit(|byte| *byte == b'/')
                                .next()
                                .unwrap_or_default(),
                        )
                        .into_owned()
                    } else {
                        file.label.clone()
                    }),
            );
        row = row.when(key.1 == 20 && file.status == "U", |row| {
            row.child(
                div()
                    .text_xs()
                    .text_color(rgb(ERROR))
                    .flex_shrink_0()
                    .child("충돌"),
            )
        });
        row.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            if this.active == Some(key.0) {
                this.list_focus = Some(ListFocus::Files(key.0));
                this.focus.focus(window);
                if key.1 == 20 {
                    let modifiers = event.modifiers();
                    let order: Vec<_> = this
                        .file_items
                        .get(&key)
                        .into_iter()
                        .flatten()
                        .filter_map(FileRow::file_index)
                        .collect();
                    let selected = this.project_mut().and_then(|project| {
                        project.stage_selection.click_visible(
                            &project.snapshot.stage,
                            &order,
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
        let generation = p.diff_generation;
        let partial = !details
            && !loading
            && !error
            && p.file_tab == FileTab::Stage
            && p.diff.partial.is_some();
        let conflicts = !details
            && !loading
            && !error
            && p.file_tab == FileTab::Stage
            && p.diff.conflict.is_some();
        let conflict_region = self
            .diff_cursor_row(id)
            .and_then(|row| p.diff.conflict_at_row(row));
        let conflict_count = p
            .diff
            .conflict
            .as_ref()
            .map_or(0, |preview| preview.regions.len());
        let selected_count = self
            .partial_selection
            .get(&id)
            .filter(|(selected_generation, _)| *selected_generation == generation)
            .map_or(0, |(_, rows)| rows.len());
        let blocked = self.busy() || self.branch_busy || self.branch_dialog.is_some();
        if details {
            if let Some(rows) = self.detail_items.get_mut(&id)
                && !rows.measured
            {
                rows.width =
                    measure_text_width(window, rows.lines.iter().map(String::as_str), 24., true);
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
                false,
            );
            self.diff_content_width.insert(id, width);
            self.diff_measured.insert(id);
        }
        let content_width = if details {
            self.detail_items.get(&id).map_or(24., |r| r.width)
        } else {
            self.diff_content_width.get(&id).copied().unwrap_or(132.)
        };
        let content_width = if !details && self.project().is_some_and(|p| p.diff.partial.is_some())
        {
            (content_width + 24.).max(620.)
        } else {
            content_width
        };
        let scroll = self.scroll_handle(key);
        let guide = (!details && count > 0).then(|| {
            let offset = scroll.0.borrow().base_handle.offset().x;
            diff_column_guides(partial, offset)
        });
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
            .when(!title.is_empty(), |panel| {
                panel.child(
                    div()
                        .px_4()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .flex_shrink_0()
                        .child(title),
                )
            })
            .when(partial, |panel| {
                panel.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .flex_shrink_0()
                        .text_xs()
                        .child(format!("변경 줄 {selected_count}개 선택"))
                        .child(
                            self.shortcut_button(
                                "stage-selected-lines",
                                "선택한 줄 Stage",
                                "viewer-stage-lines",
                                selected_count > 0 && !blocked,
                            )
                            .min_w(px(220.))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .when(selected_count == 0 || blocked, |button| button.opacity(0.4))
                            .on_click(cx.listener(
                                move |this, _, _, cx| this.stage_partial(id, generation, None, cx),
                            )),
                        )
                        .child(div().text_color(rgb(MUTED)).child(format!(
                            "{}: 선택 범위/현재 줄 체크·해제",
                            self.shortcuts.label("viewer-toggle-line")
                        ))),
                )
            })
            .when(conflicts, |panel| {
                use feather_git::conflict::Choice;
                let disabled = blocked || conflict_region.is_none();
                let label = conflict_region.map_or_else(
                    || "충돌 구간 안에 커서를 놓으세요.".to_owned(),
                    |region| format!("충돌 {} / {conflict_count}", region + 1),
                );
                let mut toolbar = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .flex_shrink_0()
                    .text_xs()
                    .child(label);
                for (key, label, choice) in [
                    ("conflict-current", "현재 변경 사용", Choice::Current),
                    ("conflict-incoming", "들어오는 변경 사용", Choice::Incoming),
                    ("conflict-both", "양쪽 모두 사용", Choice::Both),
                ] {
                    toolbar = toolbar.child(
                        button(key, label, !disabled)
                            .when(disabled, |button| button.opacity(0.4))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.resolve_conflict(id, generation, choice, cx)
                            })),
                    );
                }
                panel.child(toolbar)
            })
            .when(count == 0, |p| {
                p.child(
                    empty(if loading {
                        "조회 중입니다…"
                    } else if error {
                        "조회하지 못했습니다. 오류 내용을 확인하세요."
                    } else if details {
                        "표시할 커밋이 없습니다."
                    } else {
                        "파일을 선택해 변경 내용을 확인하세요."
                    })
                    .px_4()
                    .py_2(),
                )
            })
            .child(
                self.scroll_frame(
                    key,
                    scroll,
                    list,
                    size(
                        if count > 0 { content_width } else { 0. },
                        count as f32 * 22.,
                    ),
                    (true, guide),
                    cx,
                )
                .id(("diff-frame", key.1 as usize))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.active != Some(id)
                        || this.history_in_progress()
                        || this.help
                        || this.settings.is_some()
                        || this.commit_dialog.is_some()
                        || this.discard_dialog.is_some()
                        || this.stash_dialog.is_some()
                        || this.delete_target.is_some()
                    {
                        return;
                    }
                    if !this
                        .text_selection
                        .as_ref()
                        .is_some_and(|(selected, _)| *selected == key)
                    {
                        this.text_selection = this.initial_text_selection(key);
                    }
                    this.list_focus = Some(ListFocus::Text(key));
                    this.focus.focus(window);
                    cx.notify();
                })),
            )
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
        let Some(project) = self
            .tabs
            .iter()
            .filter_map(WorkspaceTab::project)
            .find(|p| p.id == id)
        else {
            return row;
        };
        let Some(line) = project.diff.lines.get(index) else {
            return row;
        };
        let (bg, fg) = match line.kind {
            '+' => (DIFF_ADD_BG, DIFF_ADD_TEXT),
            '-' => (DIFF_DELETE_BG, DIFF_DELETE_TEXT),
            '@' => (DIFF_HEADER_BG, DIFF_HEADER_TEXT),
            _ => (PANEL, TEXT),
        };
        let generation = project.diff_generation;
        let partial = project.file_tab == FileTab::Stage && project.diff.partial.is_some();
        let blocked = self.busy() || self.branch_busy || self.branch_dialog.is_some();
        let checked = self
            .partial_selection
            .get(&id)
            .is_some_and(|(selected_generation, rows)| {
                *selected_generation == generation && rows.contains(&index)
            });
        row.bg(rgb(bg))
            .text_color(rgb(fg))
            .when(partial, |row| {
                row.child(
                    div()
                        .w(px(24.))
                        .h(px(22.))
                        .flex_shrink_0()
                        .bg(rgb(HISTORY_HOVER))
                        .line_height(px(22.))
                        .text_center()
                        .when(matches!(line.kind, '+' | '-'), |cell| {
                            cell.child(
                                div()
                                    .id(("partial-line", index))
                                    .cursor_pointer()
                                    .text_color(rgb(if checked { ACCENT } else { MUTED }))
                                    .when(blocked, |cell| cell.opacity(0.4))
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation()
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.toggle_partial_line(id, generation, index, cx)
                                    }))
                                    .child(if checked { "☑" } else { "☐" }),
                            )
                        }),
                )
            })
            .child(
                div()
                    .w(px(48.))
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .flex_shrink_0()
                    .px_2()
                    .bg(rgb(HISTORY_HOVER))
                    .text_color(rgb(MUTED))
                    .child(line.old.map(|v| v.to_string()).unwrap_or_default()),
            )
            .child(
                div()
                    .w(px(48.))
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .flex_shrink_0()
                    .px_2()
                    .bg(rgb(HISTORY_HOVER))
                    .border_r_1()
                    .border_color(rgb(LINE))
                    .text_color(rgb(MUTED))
                    .child(line.new.map(|v| v.to_string()).unwrap_or_default()),
            )
            .child(
                div()
                    .w_5()
                    .h(px(22.))
                    .flex_shrink_0()
                    .line_height(px(22.))
                    .text_center()
                    .border_r_1()
                    .border_color(rgb(LINE))
                    .child(line.kind.to_string()),
            )
            .when(partial && line.kind == '@', |row| {
                row.child(
                    div()
                        .id(("partial-hunk", index))
                        .px_2()
                        .h(px(22.))
                        .line_height(px(22.))
                        .flex_shrink_0()
                        .cursor_pointer()
                        .text_color(rgb(ACCENT))
                        .hover(|style| style.bg(rgb(HOVER)))
                        .when(blocked, |button| button.opacity(0.4))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.stage_partial(id, generation, Some(index), cx)
                        }))
                        .child("이 구간 Stage"),
                )
            })
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
        let bold_len = if key.1 == 31
            && self.detail_items.get(&key.0).is_some_and(|rows| {
                row < rows
                    .lines
                    .iter()
                    .position(|line| line.is_empty())
                    .unwrap_or(rows.lines.len())
            }) {
            detail_label_len(&raw)
        } else {
            0
        };
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
                        let shaped = shape_styled_line(window, &displayed, color, bold_len);
                        let x_for_column = |column: usize| {
                            shaped
                                .unwrapped_layout
                                .x_for_index(displayed_byte(&raw, column))
                        };
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
                                    rgb(TEXT_SELECTION),
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
                        shaped
                            .paint(bounds.origin, px(22.), TextAlign::Left, None, window, cx)
                            .ok();
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
                                        || this.stash_dialog.is_some()
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
                                        .unwrapped_layout
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

fn toggle_checked_rows(selected: &mut std::collections::BTreeSet<usize>, rows: &[usize]) {
    if rows.iter().all(|row| selected.contains(row)) {
        for row in rows {
            selected.remove(row);
        }
    } else {
        selected.extend(rows.iter().copied());
    }
}

fn display_file_rows(files: &[FileChange], tree: bool) -> Vec<FileRow> {
    if tree {
        file_tree_rows(files)
    } else {
        file_rows(files, false)
    }
}

fn clamp_drag_y(y: f32, top: f32, height: f32) -> f32 {
    y.clamp(top, (top + height - 0.01).max(top))
}

fn drag_row(y: f32, top: f32, height: f32, offset: f32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    (((clamp_drag_y(y, top, height) - top - offset) / 22.).max(0.) as usize).min(count - 1)
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

fn detail_label_len(text: &str) -> usize {
    text.split_once(':')
        .filter(|(label, _)| {
            [
                "커밋",
                "상위 항목",
                "작성자",
                "작성시간",
                "커미터",
                "커밋시간",
            ]
            .contains(label)
        })
        .map_or(0, |(label, _)| label.len() + 1)
}

fn styled_runs(text: &str, color: u32, bold_len: usize) -> Vec<TextRun> {
    let make_run = |len, bold| {
        let mut face = font("Cascadia Mono");
        face.weight = if bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        };
        TextRun {
            len,
            font: face,
            color: rgb(color).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    };
    if bold_len > 0 {
        vec![
            make_run(bold_len, true),
            make_run(text.len() - bold_len, false),
        ]
    } else {
        vec![make_run(text.len(), false)]
    }
}

fn shape_styled_line(window: &Window, text: &str, color: u32, bold_len: usize) -> WrappedLine {
    let runs = styled_runs(text, color, bold_len);
    // shape_line merges equally colored runs before checking their fonts in GPUI 0.2.2.
    // shape_text compares font IDs and keeps bold labels separate from normal values.
    window
        .text_system()
        .shape_text(
            text.to_owned().into(),
            window.rem_size() * 0.75,
            &runs,
            None,
            None,
        )
        .unwrap_or_default()
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn measure_text_width<'a>(
    window: &Window,
    lines: impl Iterator<Item = &'a str>,
    padding: f32,
    bold_labels: bool,
) -> f32 {
    let mut metadata = bold_labels;
    lines
        .map(|line| {
            let text = line.replace('\t', "    ");
            let len = text.len();
            if text.is_empty() {
                metadata = false;
            }
            let shaped = shape_styled_line(
                window,
                &text,
                TEXT,
                if metadata { detail_label_len(&text) } else { 0 },
            );
            f32::from(shaped.unwrapped_layout.x_for_index(len))
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
            "작성시간: {}",
            format_detail_time(detail.author_time, detail.author_offset)
        ),
        format!("커미터: {} <{}>", detail.committer, detail.committer_email),
        format!(
            "커밋시간: {}",
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
    fn show_conflict_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_folder = None;
        if let Some(project) = self.project_mut() {
            project.file_tab = FileTab::Stage;
            project.show_commit_detail = false;
            project.selected_file = None;
            project.stage_selection.clear();
            project.diff = Diff::default();
            project.diff_generation += 1;
            project.diff_loading = false;
            self.list_focus = Some(ListFocus::Files(project.id));
        }
        self.focus.focus(window);
        cx.notify();
    }

    fn stash_banner(&self, cx: &Context<Self>) -> Div {
        let Some(project) = self.project() else {
            return div();
        };
        let state = project.snapshot.stash_restore.as_ref();
        let count = state.map_or(project.snapshot.unmerged.len(), |state| {
            state.conflicts.len()
        });
        let label = match state {
            Some(state) if state.applying => {
                "Stash 복원 결과를 확인해야 합니다. 적용 중 상태를 자동으로 종료하지 않습니다."
                    .to_owned()
            }
            Some(_) => format!("Stash 복원 결과 확인 · 미해결 충돌 {count}개 · 원본 Stash 유지"),
            None => format!(
                "미해결 충돌 {count}개 · DiffViewer에서 선택하거나 외부 편집 후 Stage하세요. 복원 전 백업이 없어 자동 취소할 수 없습니다."
            ),
        };
        let blocked = self.busy() || self.branch_busy || state.is_some_and(|state| state.applying);
        div()
            .px_3()
            .py_2()
            .flex()
            .items_center()
            .gap_2()
            .flex_shrink_0()
            .bg(rgb(PANEL))
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .text_ellipsis()
                    .child(label),
            )
            .child(
                button("stash-files", "해결 파일 보기", false).on_click(
                    cx.listener(|this, _, window, cx| this.show_conflict_files(window, cx)),
                ),
            )
            .when(state.is_some(), |banner| {
                banner
                    .child(
                        button("stash-recovery-finish", "해결 완료", count == 0 && !blocked)
                            .when(count != 0 || blocked, |button| button.opacity(0.4))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_stash_control(false, window, cx)
                            })),
                    )
                    .child(
                        button("stash-recovery-cancel", "복원 취소", false)
                            .when(blocked, |button| button.opacity(0.4))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_stash_control(true, window, cx)
                            })),
                    )
            })
    }

    fn integration_banner(&self, cx: &Context<Self>) -> Div {
        let Some(project) = self.project() else {
            return div();
        };
        if project.snapshot.stash_restore.is_some()
            || (project.snapshot.merge.is_none()
                && project.snapshot.rebase.is_none()
                && project.snapshot.revert.is_none())
        {
            return self.stash_banner(cx);
        }
        let (conflicts, label) = if let Some(revert) = &project.snapshot.revert {
            let mode = match revert.no_commit {
                Some(true) => " · 커밋 없음",
                Some(false) => "",
                None => " · 커밋 옵션 확인 필요",
            };
            (
                revert.conflicts.len(),
                format!(
                    "Revert 진행 중{mode} · {} · 미해결 충돌 {}개",
                    revert.target.chars().take(8).collect::<String>(),
                    revert.conflicts.len()
                ),
            )
        } else if let Some(rebase) = &project.snapshot.rebase {
            let progress = rebase
                .step
                .zip(rebase.total)
                .map(|(step, total)| format!(" · {step}/{total}단계"))
                .unwrap_or_default();
            let commit = rebase
                .current_commit
                .as_ref()
                .map(|id| format!(" · {}", id.chars().take(8).collect::<String>()))
                .unwrap_or_default();
            (
                rebase.conflicts.len(),
                format!(
                    "Rebase 진행 중{progress}{commit} · 미해결 충돌 {}개",
                    rebase.conflicts.len()
                ),
            )
        } else if let Some(merge) = &project.snapshot.merge {
            (
                merge.conflicts.len(),
                format!("Merge 진행 중 · 미해결 충돌 {}개", merge.conflicts.len()),
            )
        } else {
            return div();
        };
        let rebase = project.snapshot.rebase.is_some();
        let revert = project.snapshot.revert.is_some();
        let blocked = self.busy() || self.branch_busy;
        let ready = conflicts == 0 && !blocked;
        let banner = div()
            .px_3()
            .py_2()
            .flex()
            .items_center()
            .gap_2()
            .flex_shrink_0()
            .bg(rgb(PANEL))
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .text_ellipsis()
                    .child(label),
            )
            .child(
                button("merge-files", "해결 파일 보기", false).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.selected_folder = None;
                        if let Some(project) = this.project_mut() {
                            project.file_tab = FileTab::Stage;
                            this.list_focus = Some(ListFocus::Files(project.id));
                        }
                        this.focus.focus(window);
                        cx.notify();
                    },
                )),
            )
            .child(
                button(
                    "merge-continue",
                    if revert {
                        "되돌리기 계속"
                    } else if rebase {
                        "리베이스 계속"
                    } else {
                        "병합 완료"
                    },
                    ready,
                )
                .when(!ready, |button| button.opacity(0.4))
                .on_click(cx.listener(|this, _, window, cx| {
                    if this
                        .project()
                        .is_some_and(|project| project.snapshot.revert.is_some())
                    {
                        this.open_revert_control(
                            crate::branches::RevertControl::Continue,
                            window,
                            cx,
                        )
                    } else if this
                        .project()
                        .is_some_and(|project| project.snapshot.rebase.is_some())
                    {
                        this.open_rebase_control(
                            crate::branches::RebaseControl::Continue,
                            window,
                            cx,
                        )
                    } else {
                        this.open_merge_control(crate::branches::MergeControl::Continue, window, cx)
                    }
                })),
            )
            .when(rebase || revert, |banner| {
                banner.child(
                    button(
                        "rebase-skip",
                        if revert {
                            "되돌리기 건너뛰기"
                        } else {
                            "커밋 건너뛰기"
                        },
                        false,
                    )
                    .when(blocked, |button| button.opacity(0.4))
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this
                            .project()
                            .is_some_and(|project| project.snapshot.revert.is_some())
                        {
                            this.open_revert_control(
                                crate::branches::RevertControl::Skip,
                                window,
                                cx,
                            )
                        } else {
                            this.open_rebase_control(
                                crate::branches::RebaseControl::Skip,
                                window,
                                cx,
                            )
                        }
                    })),
                )
            })
            .child(
                button(
                    "merge-abort",
                    if revert {
                        "되돌리기 중단"
                    } else if rebase {
                        "리베이스 중단"
                    } else {
                        "병합 중단"
                    },
                    false,
                )
                .when(blocked, |button| button.opacity(0.4))
                .on_click(cx.listener(|this, _, window, cx| {
                    if this
                        .project()
                        .is_some_and(|project| project.snapshot.revert.is_some())
                    {
                        this.open_revert_control(crate::branches::RevertControl::Abort, window, cx)
                    } else if this
                        .project()
                        .is_some_and(|project| project.snapshot.rebase.is_some())
                    {
                        this.open_rebase_control(crate::branches::RebaseControl::Abort, window, cx)
                    } else {
                        this.open_merge_control(crate::branches::MergeControl::Abort, window, cx)
                    }
                })),
            );
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .child(banner)
            .when_some(
                project
                    .snapshot
                    .revert
                    .as_ref()
                    .filter(|state| !state.conflicts.is_empty()),
                |banner, state| {
                    let mut files = div()
                        .id("revert-conflict-files")
                        .max_h(px(120.))
                        .overflow_y_scroll()
                        .px_3()
                        .py_2()
                        .bg(rgb(PANEL))
                        .text_sm()
                        .child(div().text_color(rgb(ERROR)).child("되돌리기 충돌 파일"));
                    for path in &state.conflicts {
                        files =
                            files.child(div().child(String::from_utf8_lossy(path).into_owned()));
                    }
                    banner.child(files)
                },
            )
    }

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
                    .hover(|style| style.bg(rgb(HOVER)))
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
                            .hover(|style| style.bg(rgb(BUTTON_HOVER)).text_color(rgb(TEXT)))
                            .child("×")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_tab(id, cx);
                                cx.notify();
                            })),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_project_tab(id, window, cx);
                    })),
            );
        }
        let top = div()
            .border_t_1()
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
                .child(self.start_page(window, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .h_8()
                        .flex_shrink_0()
                        .px_4()
                        .bg(rgb(BG))
                        .when(self.error_history.len() > 0, |footer| {
                            footer.child(self.error_indicator(cx))
                        })
                        .child(div().flex_1().min_w_0())
                        .child(self.shortcuts_link(cx))
                        .child(self.settings_link(cx)),
                );
        }
        let branch = self.branch_panel(window, cx);
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
        let status = if let Some(project) = self.project() {
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
            .when(
                self.project()
                    .is_some_and(|project| project.snapshot.integration_active()),
                |root| root.child(self.integration_banner(cx)),
            )
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
                    .bg(rgb(BG))
                    .when(self.error_history.len() > 0, |footer| {
                        footer.child(self.error_indicator(cx))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_ellipsis()
                            .text_color(rgb(MUTED))
                            .when(self.error_history.len() > 0, |status| status.ml_2())
                            .child(status),
                    )
                    .child(self.terminal_link(cx))
                    .child(self.explorer_link(cx))
                    .child(self.shortcuts_link(cx))
                    .child(self.settings_link(cx)),
            )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.file_window_activation.is_none() {
            self.file_window_activation =
                Some(cx.observe_window_activation(window, |this, window, cx| {
                    if window.is_window_active()
                        && let Some(handle) = this.file_window
                    {
                        handle
                            .update(cx, |_, child, _| child.activate_window())
                            .ok();
                    }
                }));
        }
        self.collect_errors(cx);
        self.sync_error_editor(cx);
        self.recent_action_focus
            .retain(|path, _| self.recent.contains(path));
        for path in &self.recent {
            self.recent_action_focus
                .entry(path.clone())
                .or_insert_with(|| [cx.focus_handle(), cx.focus_handle()]);
        }
        if self.focus_after_popup || window.focused(cx).is_none() {
            self.focus_after_popup = false;
            if matches!(self.branch_dialog.as_ref().map(|dialog| &dialog.action), Some(BranchAction::Connect { plan, .. }) if !plan.candidates.is_empty())
            {
                self.upstream_focus.focus(window);
            } else {
                self.focus.focus(window);
            }
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
            || self.shortcut_settings
            || self.help
            || self.settings.is_some()
            || self.branch_dialog.is_some()
            || self.delete_target.is_some()
            || self.commit_dialog.is_some()
            || self.discard_dialog.is_some()
            || self.stash_dialog.is_some()
        {
            let title;
            let mut width = 500.;
            let mut footer: Option<AnyElement> = None;
            let header_action: Option<AnyElement> = None;
            let mut body_header = None;
            let mut card = div().flex().flex_col().gap_3().min_w_0().text_sm();
            if busy {
                let message = if self.branch_busy {
                    self.remote_status
                        .map(|(_, label)| label)
                        .unwrap_or("작업 중입니다…")
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
                title = "처리 중";
                card = card.child(message).child(
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
                                    Animation::new(std::time::Duration::from_millis(900)).repeat(),
                                    |bar, progress| bar.left(relative(progress * 1.3 - 0.3)),
                                ),
                        ),
                );
            } else if let Some(dialog) = &self.branch_dialog {
                title = match &dialog.action {
                    BranchAction::Create { .. } => "브랜치 만들기",
                    BranchAction::Tag { .. } => "태그 만들기",
                    BranchAction::Delete { plan, .. } => plan.kind.title(),
                    BranchAction::Connect { .. } => "원격 브랜치 연결",
                    BranchAction::ChooseCheckout { .. } => "체크아웃할 브랜치 선택",
                    BranchAction::Reset { .. } => "Reset",
                    BranchAction::Revert { .. } => "Revert",
                    BranchAction::Integrate { kind, .. } => kind.label(),
                    BranchAction::MergeControl { action, .. } => action.label(),
                    BranchAction::RebaseControl { action, .. } => action.label(),
                    BranchAction::RevertControl { action, .. } => action.label(),
                    BranchAction::StashControl { cancel, .. } => {
                        if *cancel {
                            "Stash 복원 취소"
                        } else {
                            "Stash 해결 완료"
                        }
                    }
                    BranchAction::Track(_) => "로컬 브랜치 만들기",
                };
                let create = matches!(dialog.action, BranchAction::Create { .. });
                let minimal = create
                    || matches!(
                        dialog.action,
                        BranchAction::Tag { .. }
                            | BranchAction::Delete { .. }
                            | BranchAction::Connect { .. }
                            | BranchAction::Reset { .. }
                            | BranchAction::Revert { .. }
                            | BranchAction::Integrate { .. }
                            | BranchAction::MergeControl { .. }
                            | BranchAction::RebaseControl { .. }
                            | BranchAction::RevertControl { .. }
                            | BranchAction::StashControl { .. }
                    );
                if create {
                    card = card.text_size(px(13.));
                }
                match &dialog.action {
                    BranchAction::Create { .. } | BranchAction::Tag { .. } => {
                        let tag = matches!(dialog.action, BranchAction::Tag { .. });
                        card = card
                            .child(if tag { "태그명" } else { "브랜치명" })
                            .when_some(dialog.editor.clone(), |card, editor| {
                                card.child(
                                    div()
                                        .h(px(36.))
                                        .overflow_hidden()
                                        .border_1()
                                        .border_color(rgb(LINE))
                                        .p_1()
                                        .child(editor),
                                )
                            });
                    }
                    // Other actions retain their existing controls.
                    BranchAction::Delete {
                        plan,
                        delete_related,
                    } => {
                        use crate::branches::DeleteKind;
                        card = card.child(match plan.kind {
                            DeleteKind::Local => "선택한 로컬 브랜치를 삭제할까요?",
                            DeleteKind::Remote => "선택한 서버의 원격 브랜치를 삭제할까요?",
                            DeleteKind::Tag => "선택한 태그를 삭제할까요?",
                        });
                        if plan.kind != DeleteKind::Tag {
                            let (relationship, checkbox) = if plan.kind == DeleteKind::Local {
                                ("원격 브랜치", "원격 브랜치 삭제")
                            } else {
                                ("로컬 브랜치", "로컬 브랜치 삭제")
                            };
                            let can_delete_related = !plan.related.is_empty();
                            let names = plan
                                .related
                                .iter()
                                .map(|target| target.label())
                                .collect::<Vec<_>>()
                                .join(", ");
                            card = card
                                .child(format!(
                                    "{relationship}: {}",
                                    if names.is_empty() { "없음" } else { &names }
                                ))
                                .child(
                                    button("delete-related-branch", "", false)
                                        .when(can_delete_related, |button| {
                                            button.track_focus(&self.branch_checks_focus[0])
                                        })
                                        .when(
                                            can_delete_related
                                                && self.branch_checks_focus[0].is_focused(window),
                                            |button| button.bg(rgb(SELECT)),
                                        )
                                        .when(!can_delete_related, |button| button.opacity(0.45))
                                        .child(crate::controls::checkbox(*delete_related, checkbox))
                                        .on_click(cx.listener(|this, event, window, cx| {
                                            if !crate::controls::checkbox_click(event) {
                                                return;
                                            }
                                            if this.branch_busy {
                                                return;
                                            }
                                            if let Some(dialog) = &mut this.branch_dialog
                                                && let BranchAction::Delete {
                                                    plan,
                                                    delete_related,
                                                } = &mut dialog.action
                                                && !plan.related.is_empty()
                                            {
                                                *delete_related = !*delete_related;
                                                this.branch_checks_focus[0].focus(window);
                                                this.notify(cx);
                                            }
                                        })),
                                );
                        }
                    }
                    BranchAction::Connect { plan, selected } => {
                        let mut choices = div()
                            .id("upstream-choices")
                            .flex()
                            .flex_col()
                            .when(!plan.candidates.is_empty(), |list| {
                                list.h(px(plan.candidates.len().min(8) as f32 * 30.))
                            })
                            .max_h((window.viewport_size().height - px(220.)).max(px(30.)))
                            .flex_shrink_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.upstream_scroll);
                        if plan.candidates.is_empty() {
                            choices = choices
                                .child("연결할 원격 브랜치가 없습니다. Fetch 후 다시 시도하세요.");
                        }
                        for (index, candidate) in plan.candidates.iter().enumerate() {
                            let label = if candidate.reference == plan.current {
                                format!("{} · 현재 연결", candidate.label)
                            } else {
                                candidate.label.clone()
                            };
                            choices = choices.child(
                                button(("upstream-choice", index), "", *selected == index)
                                    .h(px(30.))
                                    .w_full()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .flex_shrink_0()
                                    .py_0()
                                    .when(*selected == index, |row| {
                                        row.track_focus(&self.upstream_focus)
                                    })
                                    .child(
                                        crate::controls::radio(
                                            *selected == index,
                                            div().flex_1().min_w_0().text_ellipsis().child(label),
                                        )
                                        .min_w_0()
                                        .w_full(),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if this.branch_busy {
                                            return;
                                        }
                                        if let Some(dialog) = &mut this.branch_dialog
                                            && let BranchAction::Connect { selected, .. } =
                                                &mut dialog.action
                                        {
                                            *selected = index;
                                        }
                                        this.upstream_focus.focus(window);
                                        this.notify(cx);
                                    })),
                            );
                        }
                        card = card.child(choices);
                    }
                    BranchAction::ChooseCheckout { candidates, .. } => {
                        let mut choices = div()
                            .id("checkout-choices")
                            .flex()
                            .flex_col()
                            .gap_2()
                            .max_h(px(240.))
                            .overflow_y_scroll();
                        for (index, candidate) in candidates.iter().enumerate() {
                            let label = div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .when(index < 10, |row| {
                                    row.child(crate::shortcut::badge(
                                        &self
                                            .shortcuts
                                            .label(&format!("checkout-{}", (index + 1) % 10)),
                                    ))
                                })
                                .child(candidate.label.clone());
                            choices = choices.child(
                                button(("checkout-choice", index), "", false)
                                    .child(label)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.choose_checkout_candidate(index, cx)
                                    })),
                            );
                        }
                        card = card.child(choices);
                    }
                    BranchAction::Reset { mode, .. } => {
                        for (index, option) in
                            crate::branches::ResetMode::ALL.iter().copied().enumerate()
                        {
                            let checked = option == *mode;
                            card = card.child(
                                button(("reset-radio", index), "", checked)
                                    .when(checked, |button| {
                                        button.track_focus(&self.reset_mode_focus)
                                    })
                                    .child(crate::controls::radio(
                                        checked,
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(crate::shortcut::badge(
                                                &self
                                                    .shortcuts
                                                    .label(&format!("reset-{}", index + 1)),
                                            ))
                                            .child(option.label()),
                                    ))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if this.branch_busy {
                                            return;
                                        }
                                        if let Some(dialog) = &mut this.branch_dialog
                                            && let BranchAction::Reset { mode, .. } =
                                                &mut dialog.action
                                        {
                                            *mode = option;
                                        }
                                        this.reset_mode_focus.focus(window);
                                        this.notify(cx);
                                    })),
                            );
                        }
                        card = card.child(
                            div()
                                .text_sm()
                                .text_color(rgb(if *mode == crate::branches::ResetMode::Hard {
                                    ERROR
                                } else {
                                    MUTED
                                }))
                                .child(mode.description()),
                        );
                    }
                    BranchAction::Revert { no_commit, .. } => {
                        card = card.child(
                            button("revert-no-commit", "", false)
                                .track_focus(&self.branch_checks_focus[0])
                                .when(self.branch_checks_focus[0].is_focused(window), |button| {
                                    button.bg(rgb(SELECT))
                                })
                                .child(crate::controls::checkbox(*no_commit, "no commit"))
                                .on_click(cx.listener(|this, event, window, cx| {
                                    if !crate::controls::checkbox_click(event) {
                                        return;
                                    }
                                    if this.branch_busy {
                                        return;
                                    }
                                    if let Some(dialog) = &mut this.branch_dialog
                                        && let BranchAction::Revert { no_commit, .. } =
                                            &mut dialog.action
                                    {
                                        *no_commit = !*no_commit;
                                        this.popup_options.revert_no_commit = *no_commit;
                                    }
                                    this.branch_checks_focus[0].focus(window);
                                    this.persist();
                                    this.notify(cx);
                                })),
                        );
                    }
                    BranchAction::Integrate { kind, .. } => {
                        card = card.child(match kind {
                            crate::branches::IntegrateKind::Merge => {
                                "선택한 커밋의 이력을 현재 체크아웃된 HEAD에 병합할까요?"
                            }
                            crate::branches::IntegrateKind::Rebase => {
                                "현재 브랜치의 고유 커밋들을 선택한 커밋 위에 재배치할까요?"
                            }
                        });
                    }
                    BranchAction::MergeControl { action, .. } => {
                        card = card.child(match action {
                            crate::branches::MergeControl::Continue => "Stage된 내용으로 병합을 완료할까요? 기본 병합 메시지로 커밋합니다.",
                            crate::branches::MergeControl::Abort => "병합을 중단할까요? 현재 충돌 해결 내용이 사라질 수 있습니다. 병합 전 미커밋 변경은 완전히 복원되지 않을 수 있습니다.",
                        });
                    }
                    BranchAction::RebaseControl { action, .. } => {
                        card = card.child(match action {
                            crate::branches::RebaseControl::Continue => "Stage된 해결 내용으로 리베이스를 계속할까요? 원래 커밋 메시지를 사용하며 다음 커밋에서 다시 충돌할 수 있습니다.",
                            crate::branches::RebaseControl::Abort => "리베이스를 중단하고 시작 전 상태로 돌아갈까요? 현재 충돌 해결 내용이 사라질 수 있습니다.",
                            crate::branches::RebaseControl::Skip => "현재 커밋을 건너뛸까요? 이 커밋의 변경과 현재 충돌 해결 내용이 제외되고 다음 커밋으로 진행합니다.",
                        });
                    }
                    BranchAction::RevertControl {
                        state,
                        action,
                        no_commit,
                    } => {
                        card = card.child(match action {
                            crate::branches::RevertControl::Continue if *no_commit => "커밋을 만들지 않고 되돌리기 해결을 완료할까요? 현재 해결 결과와 기존 Stage 내용을 남깁니다.",
                            crate::branches::RevertControl::Continue => "Stage된 해결 내용으로 되돌리기를 계속할까요? 기본 메시지로 새 Revert 커밋을 만듭니다.",
                            crate::branches::RevertControl::Abort => "되돌리기를 중단할까요? 현재 해결 내용과 미커밋/Stage 변경이 사라질 수 있습니다.",
                            crate::branches::RevertControl::Skip => "현재 되돌리기를 건너뛸까요? 이 대상의 해결 내용과 미커밋/Stage 변경이 사라질 수 있습니다.",
                        });
                        if *action == crate::branches::RevertControl::Continue
                            && state.no_commit.is_none()
                        {
                            card = card.child("외부에서 시작한 Revert의 커밋 옵션을 확인할 수 없습니다. 커밋 여부를 선택하세요.")
                                .child(button("revert-recovery-mode", "", false)
                                    .track_focus(&self.branch_checks_focus[0])
                                    .child(crate::controls::checkbox(*no_commit, "커밋하지 않기"))
                                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                                        if !crate::controls::checkbox_click(event) { return; }
                                        if this.branch_busy { return; }
                                        if let Some(dialog) = &mut this.branch_dialog
                                            && let BranchAction::RevertControl { no_commit, .. } = &mut dialog.action {
                                            *no_commit = !*no_commit;
                                        }
                                        this.branch_checks_focus[0].focus(window);
                                        this.notify(cx);
                                    })));
                        }
                    }
                    BranchAction::StashControl { cancel, .. } => {
                        card = card.child(if *cancel {
                            "보관한 복원 전 파일과 Stage 상태로 돌아갈까요? 복원 후 해당 경로에 한 수정은 사라질 수 있습니다. 원본 Stash는 유지됩니다."
                        } else { "현재 해결 결과를 유지하고 복원 상태를 종료할까요? 커밋을 만들지 않으며 원본 Stash는 유지됩니다." });
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
                if let BranchAction::Create {
                    remote, checkout, ..
                } = &dialog.action
                {
                    for (id, label, checked, switching) in [
                        ("create-remote-option", "원격브랜치 생성", *remote, false),
                        (
                            "create-checkout-option",
                            "새로운 브랜치 체크아웃",
                            *checkout,
                            true,
                        ),
                    ] {
                        card = card.child(
                            button(id, "", false)
                                .track_focus(&self.branch_checks_focus[usize::from(switching)])
                                .when(
                                    self.branch_checks_focus[usize::from(switching)]
                                        .is_focused(window),
                                    |button| button.bg(rgb(SELECT)),
                                )
                                .py(px(crate::controls::CHECKBOX_PADDING))
                                .flex()
                                .items_center()
                                .text_size(px(13.))
                                .child(crate::controls::checkbox(checked, label))
                                .on_click(cx.listener(move |this, event, window, cx| {
                                    if !crate::controls::checkbox_click(event) {
                                        return;
                                    }
                                    this.branch_checks_focus[usize::from(switching)].focus(window);
                                    if let Some(dialog) = &mut this.branch_dialog
                                        && let BranchAction::Create {
                                            remote, checkout, ..
                                        } = &mut dialog.action
                                    {
                                        if switching {
                                            *checkout = !*checkout;
                                        } else {
                                            *remote = !*remote;
                                        }
                                        if let Some(dialog) = &this.branch_dialog
                                            && let BranchAction::Create {
                                                remote, checkout, ..
                                            } = &dialog.action
                                        {
                                            this.popup_options.branch_remote = *remote;
                                            this.popup_options.branch_checkout = *checkout;
                                        }
                                        this.persist();
                                        this.notify(cx);
                                    }
                                })),
                        );
                    }
                }
                card = card.when_some(dialog.error.clone(), |card, error| {
                    card.child(div().text_color(rgb(ERROR)).child(error))
                });
                if !matches!(dialog.action, BranchAction::ChooseCheckout { .. }) {
                    let can_submit = !matches!(&dialog.action, BranchAction::Connect { plan, selected } if plan.candidates.get(*selected).is_none());
                    footer = Some(
                        div()
                            .flex()
                            .flex_row_reverse()
                            .gap_2()
                            .when(minimal, |footer| footer.justify_end())
                            .child(
                                self.shortcut_button("confirm-branch", "확인", "confirm", true)
                                    .when(!can_submit, |button| button.opacity(0.4))
                                    .when(create, |button| button.text_size(px(13.)))
                                    .on_click(cx.listener(|this, _, _, cx| this.submit_branch(cx))),
                            )
                            .when(!minimal, |footer| {
                                footer.child(
                                    self.shortcut_button("cancel-branch", "취소", "cancel", false)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.close_branch_dialog(window, cx)
                                        })),
                                )
                            })
                            .into_any_element(),
                    );
                }
            } else if let Some(settings) = &self.settings {
                title = "세팅";
                width = 760.;
                let keys = self.shortcuts.label("confirm");
                footer = Some(
                    settings
                        .update(cx, |settings, cx| settings.save_button(&keys, cx))
                        .into_any_element(),
                );
                card = card.child(settings.clone());
            } else if let Some(dialog) = &self.stash_dialog {
                title = if dialog.deletion.is_some() {
                    "Stash 삭제"
                } else {
                    "스태시 추가"
                };
                width = 640.;
                card = card
                    .when_some(dialog.editor.clone(), |card, editor| {
                        card.child(div().text_color(rgb(MUTED)).child("스태시 이름"))
                            .child(editor)
                    })
                    .when_some(dialog.deletion.as_ref(), |card, (_, _, name)| {
                        card.child(format!("{name}\n스태시를 목록에서 영구히 삭제합니다. 현재 파일과 stage 상태는 유지됩니다."))
                    })
                    .when_some(dialog.error.clone(), |card, error| card.child(div().text_color(rgb(ERROR)).child(error)));
                if dialog.busy {
                    card = card.child(if dialog.deletion.is_some() {
                        "스태시를 삭제하는 중입니다…"
                    } else {
                        "스태시에 저장하는 중입니다…"
                    });
                } else {
                    footer = Some(
                        div()
                            .flex()
                            .flex_row_reverse()
                            .gap_2()
                            .child(
                                self.shortcut_button("confirm-stash", "확인", "confirm", true)
                                    .on_click(cx.listener(|this, _, _, cx| this.stash_changes(cx))),
                            )
                            .into_any_element(),
                    );
                }
            } else if let Some(dialog) = &self.discard_dialog {
                title = "선택한 변경사항 폐기";
                width = 640.;
                card = card
                    .child(format!("{}개 파일의 staged·unstaged 변경을 HEAD로 복원합니다. 신규 파일은 삭제됩니다.", dialog.paths.len()))
                    .when_some(dialog.error.clone(), |card, error| card.child(div().text_color(rgb(ERROR)).child(error)))
                    .when(dialog.busy, |card| card.child("변경사항을 폐기하는 중입니다…"));
                footer = Some(
                    self.shortcut_button("confirm-discard", "확인", "confirm", true)
                        .when(dialog.busy, |button| button.opacity(0.45))
                        .on_click(cx.listener(|this, _, _, cx| this.discard_selected(cx)))
                        .into_any_element(),
                );
            } else if let Some(dialog) = &self.commit_dialog {
                title = "커밋";
                width = 640.;
                card = card
                    .child(div().text_color(rgb(MUTED)).child("커밋메시지"))
                    .child(dialog.editor.clone())
                    .child(
                        button("commit-push-option", "", false)
                            .track_focus(&self.commit_push_focus)
                            .when(self.commit_push_focus.is_focused(window), |button| {
                                button.bg(rgb(SELECT))
                            })
                            .when(dialog.busy, |button| button.opacity(0.45))
                            .child(crate::controls::checkbox(dialog.push_after, "커밋 후 푸시"))
                            .on_click(cx.listener(|this, event, window, cx| {
                                if !crate::controls::checkbox_click(event) {
                                    return;
                                }
                                if let Some(dialog) = &mut this.commit_dialog
                                    && !dialog.busy
                                    && !dialog.editor.read(cx).is_composing()
                                {
                                    dialog.push_after = !dialog.push_after;
                                    this.commit_push_focus.focus(window);
                                    this.notify(cx);
                                }
                            })),
                    )
                    .when_some(dialog.error.clone(), |card, error| {
                        card.child(div().text_color(rgb(ERROR)).child(error))
                    })
                    .when(dialog.busy, |card| {
                        card.child(if dialog.push_after {
                            "커밋 후 푸시하는 중입니다…"
                        } else {
                            "커밋하는 중입니다…"
                        })
                    });
                footer = Some(
                    self.shortcut_button("confirm-commit", "확인", "confirm", true)
                        .when(dialog.busy, |button| button.opacity(0.45))
                        .on_click(cx.listener(|this, _, _, cx| this.submit_commit(cx)))
                        .into_any_element(),
                );
            } else if let Some(path) = &self.delete_target
                && !self.delete_confirm
            {
                title = "최근 저장소 관리";
                card = card
                    .child(session::display_path(path))
                    .child("목록에서만 제거하거나 저장소 폴더를 삭제할 수 있습니다.");
                footer = Some(
                    div()
                        .flex()
                        .gap_2()
                        .child(button("forget-recent", "목록에서 제거", false).on_click(
                            cx.listener(|this, _, _, cx| {
                                if let Some(path) = this.delete_target.take() {
                                    this.recent.retain(|item| *item != path);
                                    this.persist();
                                }
                                this.notify(cx);
                            }),
                        ))
                        .child(button("choose-delete", "저장소 삭제…", false).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.delete_confirm = true;
                                this.notify(cx);
                            }),
                        ))
                        .into_any_element(),
                );
            } else if let Some(path) = &self.delete_target {
                title = "저장소 삭제";
                card = card
                    .child(session::display_path(path))
                    .child("미커밋 파일을 포함한 저장소 폴더 전체를 영구 삭제합니다.");
                footer = Some(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button("confirm-delete", "삭제", true)
                                .on_click(cx.listener(|this, _, _, cx| this.delete_repository(cx))),
                        )
                        .into_any_element(),
                );
            } else {
                title = if self.shortcut_settings {
                    "단축키 설정"
                } else {
                    "도움말"
                };
                width = 820.;
                card = card.child(self.shortcuts_table(self.shortcut_settings, cx));
                if self.shortcut_settings {
                    body_header = Some(self.shortcut_tabs(cx));
                    footer = Some(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                self.shortcut_button(
                                    "reset-shortcuts",
                                    "기본값 복원",
                                    "shortcuts-reset",
                                    false,
                                )
                                .bg(rgb(BUTTON))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.reset_shortcuts(cx);
                                    },
                                )),
                            )
                            .child(
                                self.shortcut_button("confirm-shortcuts", "확인", "confirm", true)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.apply_shortcuts(window, cx)
                                    })),
                            )
                            .into_any_element(),
                    );
                }
            }
            let card = crate::popup::frame(
                title,
                width,
                crate::popup::Body {
                    header: body_header,
                    content: card,
                },
                footer,
                header_action,
                window.viewport_size().height,
                self.shortcut_settings.then_some(640.),
            );
            root = root.child(
                div()
                    .id("modal-backdrop")
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .occlude()
                    .bg(rgba(BACKDROP))
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
                            && !this.stash_dialog.as_ref().is_some_and(|dialog| dialog.busy)
                        {
                            this.help = false;
                            this.shortcut_settings = false;
                            this.shortcut_draft = None;
                            this.shortcut_capture = None;
                            this.shortcut_message = None;
                            this.delete_target = None;
                            this.commit_dialog = None;
                            this.discard_dialog = None;
                            this.stash_dialog = None;
                            this.focus.focus(window);
                            this.notify(cx);
                        }
                        cx.stop_propagation();
                    }))
                    .child(card),
            );
        }
        if let Some(settings) = &self.settings
            && settings.read(cx).child_popup_open(cx)
        {
            let keys = self.shortcuts.label("confirm");
            root = root
                .child(settings.update(cx, |settings, cx| settings.child_popup(&keys, window, cx)));
        }
        if self.error_history_open && self.error_history.len() > 0 {
            root = root.child(self.error_overlay(window, cx));
        }
        if self.file_window.is_some() {
            root = root.child(
                div()
                    .id("file-window-parent-blocker")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .bg(rgba(0x24324730))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(handle) = this.file_window {
                            handle
                                .update(cx, |_, child, _| child.activate_window())
                                .ok();
                        }
                        cx.stop_propagation();
                    })),
            );
        }
        root
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn search_arrows_move_between_virtual_row_input_and_results() {
        assert_eq!(
            search_arrow_target(SEARCH_VIRTUAL_ROW, 0, true),
            SearchArrowTarget::Input
        );
        assert_eq!(
            search_arrow_target(SEARCH_VIRTUAL_ROW, 2, false),
            SearchArrowTarget::Row(SEARCH_VIRTUAL_ROW)
        );
        assert_eq!(search_arrow_target(0, 2, false), SearchArrowTarget::Input);
        assert_eq!(search_arrow_target(0, 2, true), SearchArrowTarget::Row(1));
        assert_eq!(search_arrow_target(1, 2, true), SearchArrowTarget::Row(1));
    }

    #[::core::prelude::v1::test]
    fn panel_tabs_cycle_forward_and_backward_and_handle_empty_or_single_tabs() {
        let mut current = 0;
        for expected in [1, 2, 3, 0] {
            current = adjacent_tab(current, RefTab::ALL.len(), true).unwrap();
            assert_eq!(current, expected);
        }
        for expected in [3, 2, 1, 0] {
            current = adjacent_tab(current, RefTab::ALL.len(), false).unwrap();
            assert_eq!(current, expected);
        }
        for next in [true, false] {
            assert_eq!(adjacent_tab(0, 2, next), Some(1));
            assert_eq!(adjacent_tab(1, 2, next), Some(0));
            assert_eq!(adjacent_tab(0, 1, next), Some(0));
            assert_eq!(adjacent_tab(0, 0, next), None);
        }
        assert_eq!(
            adjacent_tab(0, crate::shortcut::GROUPS.len(), false),
            Some(4)
        );
        assert_eq!(
            adjacent_tab(4, crate::shortcut::GROUPS.len(), true),
            Some(0)
        );
        assert_eq!(adjacent_tab(0, 3, false), Some(2));
        assert_eq!(adjacent_tab(2, 3, true), Some(0));
    }

    #[::core::prelude::v1::test]
    fn reference_tabs_cycle_through_search_in_both_directions() {
        assert_eq!(adjacent_reference_tab(RefTab::Tag, false, true), 4);
        assert_eq!(adjacent_reference_tab(RefTab::Tag, true, true), 0);
        assert_eq!(adjacent_reference_tab(RefTab::Local, false, false), 4);
        assert_eq!(adjacent_reference_tab(RefTab::Local, true, false), 3);
        assert_eq!(adjacent_reference_tab(RefTab::Tag, false, false), 2);
    }

    #[::core::prelude::v1::test]
    fn range_check_completes_mixed_rows_then_clears_them_without_touching_other_rows() {
        let mut checked = std::collections::BTreeSet::from([1, 9]);
        toggle_checked_rows(&mut checked, &[1, 3, 6]);
        assert_eq!(checked, std::collections::BTreeSet::from([1, 3, 6, 9]));
        toggle_checked_rows(&mut checked, &[1, 3, 6]);
        assert_eq!(checked, std::collections::BTreeSet::from([9]));
        toggle_checked_rows(&mut checked, &[]);
        assert_eq!(checked, std::collections::BTreeSet::from([9]));
        toggle_checked_rows(&mut checked, &[9]);
        assert!(checked.is_empty());
    }

    #[::core::prelude::v1::test]
    fn ijkl_and_checkout_digit_shortcuts_map_correctly() {
        assert_eq!(navigation_key("i"), "up");
        assert_eq!(navigation_key("j"), "left");
        assert_eq!(navigation_key("K"), "down");
        assert_eq!(navigation_key("l"), "right");
        assert_eq!(navigation_key("up"), "up");
        assert_eq!(candidate_shortcut("1"), Some(0));
        assert_eq!(candidate_shortcut("9"), Some(8));
        assert_eq!(candidate_shortcut("0"), Some(9));
        assert_eq!(candidate_shortcut("10"), None);
    }

    #[::core::prelude::v1::test]
    fn numeric_shortcuts_map_to_sections_and_current_viewer() {
        assert_eq!(section_focus("1", 7, false), Some(ListFocus::Branches(7)));
        assert_eq!(section_focus("2", 7, false), Some(ListFocus::History(7)));
        assert_eq!(section_focus("3", 7, false), Some(ListFocus::Files(7)));
        assert_eq!(section_focus("4", 7, false), Some(ListFocus::Text((7, 30))));
        assert_eq!(section_focus("4", 7, true), Some(ListFocus::Text((7, 31))));
        assert_eq!(section_focus("5", 7, false), None);
    }

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

    #[::core::prelude::v1::test]
    fn text_drag_clamps_vertical_position_to_viewport() {
        assert_eq!(clamp_drag_y(5., 10., 100.), 10.);
        assert_eq!(clamp_drag_y(60., 10., 100.), 60.);
        assert!(clamp_drag_y(150., 10., 100.) < 110.);
        assert!(clamp_drag_y(150., 10., 100.) > 109.);
        assert_eq!(clamp_drag_y(150., 10., 0.), 10.);
        assert_eq!(drag_row(5., 10., 100., 0., 20), 0);
        assert_eq!(drag_row(150., 10., 100., 0., 20), 4);
        assert_eq!(drag_row(150., 10., 100., -44., 20), 6);
        assert_eq!(drag_row(150., 10., 100., -440., 20), 19);
        assert_eq!(drag_row(150., 10., 100., 0., 0), 0);
    }

    #[::core::prelude::v1::test]
    fn metadata_labels_only_cover_prefixes() {
        for label in [
            "커밋",
            "상위 항목",
            "작성자",
            "작성시간",
            "커미터",
            "커밋시간",
        ] {
            assert_eq!(
                detail_label_len(&format!("{label}: value")),
                label.len() + 1
            );
        }
        assert_eq!(detail_label_len("제목: message"), 0);
        assert_eq!(detail_label_len(""), 0);
        let text = "작성자: 이름";
        let runs = styled_runs(text, TEXT, detail_label_len(text));
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].font.weight, FontWeight::BOLD);
        assert_eq!(runs[1].font.weight, FontWeight::NORMAL);
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), text.len());
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
        assert!(rows.contains(&"작성시간: 1970-01-01 09:00:00".into()));
        assert!(rows.contains(&"커밋시간: 1969-12-31 20:00:00".into()));
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
