use crate::history_view::{self, ColumnLayout, RowGraph};
use crate::layout::{SplitAxis, SplitLayout};
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
}

const BG: u32 = 0x10151d;
const PANEL: u32 = 0x151c26;
const LINE: u32 = 0x293343;
const TEXT: u32 = 0xd9e2ed;
const MUTED: u32 = 0x8290a6;
const ACCENT: u32 = 0x5edac1;
const SELECT: u32 = 0x203d42;
const TAB_BAR_HEIGHT: f32 = 44.;
const SCROLLBAR: f32 = 12.;
type ListKey = (u64, u8);

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
    tabs: Vec<WorkspaceTab>,
    tabs_scroll: ScrollHandle,
    active: Option<u64>,
    next_id: u64,
    error: Option<String>,
    layout: SplitLayout,
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

impl Workspace {
    pub fn new(paths: Vec<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            tabs: Vec::new(),
            tabs_scroll: ScrollHandle::new(),
            active: None,
            next_id: 0,
            error: None,
            layout: SplitLayout::default(),
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
        if paths.is_empty() {
            this.add_empty_tab();
        }
        for path in paths {
            this.open(path, cx);
        }
        this
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
                match result {
                    Ok(page) => {
                        p.snapshot.commits.extend(page.commits);
                        p.snapshot.has_more = page.has_more;
                    }
                    Err(error) => {
                        p.page_error = Some(format!("{error:#}"));
                    }
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

    fn add_empty_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(WorkspaceTab::Empty(id));
        self.start_pages.insert(id, StartPage::default());
        self.active = Some(id);
        self.error = None;
        self.tabs_scroll.scroll_to_item(self.tabs.len() - 1);
    }

    fn close_tab(&mut self, id: u64) {
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
        // 정규화도 저장소 탐색과 함께 백그라운드에서 수행한다.
        let task = cx
            .background_executor()
            .spawn(async move { git::repository_root(&path) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(path) => {
                        if let Some(project) = this
                            .tabs
                            .iter()
                            .filter_map(WorkspaceTab::project)
                            .find(|p| p.path == path)
                        {
                            this.active = Some(project.id);
                        } else {
                            let id = this.next_id;
                            this.next_id += 1;
                            this.tabs
                                .push(WorkspaceTab::Repository(Box::new(Project::new(id, path))));
                            this.active = Some(id);
                            this.reload(cx);
                        }
                        this.error = None;
                        if let Some(index) = this
                            .tabs
                            .iter()
                            .position(|tab| Some(tab.id()) == this.active)
                        {
                            this.tabs_scroll.scroll_to_item(index);
                        }
                    }
                    Err(err) => {
                        if this.tabs.is_empty() {
                            this.add_empty_tab();
                        }
                        this.error = Some(format!("{err:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
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
                        if let Some(tab) = this.tabs.iter_mut().find(|t| t.id() == id) {
                            *tab = WorkspaceTab::Repository(Box::new(Project::new(id, path)));
                        }
                        this.start_pages.remove(&id);
                        // 조회 시작 시에만 활성 ID를 바꾸고 사용자의 선택은 유지한다.
                        let active = this.active;
                        this.active = Some(id);
                        this.reload(cx);
                        this.active = active;
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

    fn start_page(&self, cx: &Context<Self>) -> Div {
        let Some(id) = self.active else { return div() };
        let Some(page) = self.start_pages.get(&id) else {
            return div();
        };
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(420.))
            .max_w_full()
            .child(div().text_xl().child("저장소 시작"));
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
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .p_5()
            .child(content)
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.project_mut() else { return };
        p.generation += 1;
        p.page_generation += 1;
        p.page_loading = false;
        p.page_error = None;
        p.diff_generation += 1;
        p.diff_loading = false;
        let (id, generation) = (p.id, p.generation);
        let (path, commit) = (p.path.clone(), p.commit.clone());
        p.loading = true;
        p.error = None;
        p.selected_file = None;
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
                match result {
                    Ok(snapshot) => {
                        if !snapshot
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
                        let stage = file_rows(&snapshot.stage, true);
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
        let Some(p) = self.project_mut() else { return };
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

    fn select_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(p) = self.project_mut() else { return };
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
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|index| this.branch_row(id, kind, index, cx))
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

    fn branch_row(&self, id: u64, kind: usize, index: usize, cx: &Context<Self>) -> Stateful<Div> {
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
            .bg(rgb(if active { SELECT } else { PANEL }))
            .text_color(rgb(if current { ACCENT } else { TEXT }))
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0x263443)))
            .child(div().min_w_0().text_ellipsis().child(format!(
                "{}{}",
                if current { "● " } else { "⑂ " },
                reference.name
            )));
        row.on_click(cx.listener(move |this, _, _, cx| {
            if this.active != Some(id) {
                return;
            }
            if let Some(p) = this.project_mut() {
                p.revision = Some(revision.clone());
            }
            if let Some(p) = this.project()
                && let Some(index) = p.snapshot.commits.iter().position(|c| c.id == target)
                && let Some(scroll) = this.history_scroll.get(&id)
            {
                scroll.scroll_to_item(index, ScrollStrategy::Center);
            }
            this.select_commit(target.clone(), cx);
        }))
    }

    fn history_row(
        &mut self,
        project_id: u64,
        index: usize,
        widths: [f32; 5],
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
        let graph = self
            .graphs
            .entry(project_id)
            .or_default()
            .entry(index)
            .or_insert_with(|| RowGraph::new(commit));
        row = row
            .bg(rgb(if selected { SELECT } else { PANEL }))
            .hover(|s| s.bg(rgb(0x202e3c)))
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
        description =
            description.child(div().min_w_0().text_ellipsis().child(commit.title.clone()));
        row = row.child(history_view::cell(1, widths[1]).child(description));
        for (column, text) in [
            (2, commit.author.clone()),
            (
                3,
                history_view::format_time(commit.time, commit.time_offset),
            ),
            (4, commit.id.chars().take(8).collect()),
        ] {
            row = row.child(
                history_view::cell(column, widths[column])
                    .child(div().min_w_0().text_ellipsis().child(text)),
            );
        }
        row.on_click(cx.listener(move |this, _, _, cx| {
            if this.active == Some(project_id) {
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
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
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
                    .map(|index| this.history_row(project_id, index, widths, cx))
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

    fn files_panel(&mut self, cx: &Context<Self>) -> Div {
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
                    if let Some(p) = this.project_mut() {
                        p.file_tab = tab;
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
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|index| this.file_row(key, index, cx))
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
                    button("commit-detail", "커밋상세", detail)
                        .h(px(history_view::ROW_HEIGHT))
                        .w_full()
                        .py_0()
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        .rounded_none()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(p) = this.project_mut() {
                                p.show_commit_detail = true;
                                if p.snapshot.commit_detail.is_some() {
                                    p.error = None;
                                }
                                p.selected_file = None;
                                p.diff_generation += 1;
                                p.diff_loading = false;
                                p.diff = Diff::default();
                            }
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

    fn file_row(&self, key: ListKey, index: usize, cx: &Context<Self>) -> Stateful<Div> {
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
        if let FileRow::Group(staged) = item {
            return row.text_xs().text_color(rgb(MUTED)).child(if *staged {
                "STAGED · 스테이징"
            } else {
                "WORKTREE · 작업 디렉터리"
            });
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
        row = row
            .bg(rgb(if p.selected_file == Some(file_index) {
                SELECT
            } else {
                PANEL
            }))
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0x263443)))
            .child(
                div()
                    .text_xs()
                    .flex_shrink_0()
                    .text_color(rgb(match file.status.as_str() {
                        "A" => 0x80d9a4,
                        "D" => 0xf28d95,
                        _ => 0xe8c783,
                    }))
                    .child(file.status.clone()),
            )
            .child(div().min_w_0().text_ellipsis().child(file.label.clone()));
        row.on_click(cx.listener(move |this, _, _, cx| {
            if this.active == Some(key.0) {
                this.select_file(file_index, cx);
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
            cx.processor(move |this, range: std::ops::Range<usize>, _, _| {
                range
                    .map(|index| this.diff_row(id, index, details, content_width))
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

    fn diff_row(&self, id: u64, index: usize, details: bool, width: f32) -> Div {
        let row = div()
            .flex()
            .w(px(width))
            .h(px(22.))
            .min_w_full()
            .whitespace_nowrap();
        if details {
            return row.px_3().child(
                self.detail_items
                    .get(&id)
                    .and_then(|r| r.lines.get(index))
                    .map(|line| line.replace('\t', "    "))
                    .unwrap_or_default(),
            );
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
            .child(
                div()
                    .flex_shrink_0()
                    .pr_4()
                    .child(line.text.replace('\t', "    ")),
            )
    }
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

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                                this.close_tab(id);
                                cx.notify();
                            })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.active = Some(id);
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
                .when_some(self.error.clone(), |page, error| {
                    page.child(
                        div()
                            .px_4()
                            .py_2()
                            .text_sm()
                            .text_color(rgb(0xf4a0a8))
                            .child(error),
                    )
                });
        }
        let branch = self.branch_panel(cx);
        let history = self.history_panel(cx);
        let files = self.files_panel(cx);
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
        let status = if let Some(err) = &self.error {
            err.clone()
        } else if let Some(p) = self.project() {
            if let Some(err) = &p.error {
                err.clone()
            } else {
                format!(
                    "{}  ·  {}{}",
                    p.path.display(),
                    p.snapshot.head,
                    if p.loading || p.diff_loading {
                        "  ·  조회 중…"
                    } else {
                        ""
                    }
                )
            }
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
                            .text_color(rgb(
                                if self.error.is_some()
                                    || self.project().is_some_and(|p| p.error.is_some())
                                {
                                    0xf4a0a8
                                } else {
                                    MUTED
                                },
                            ))
                            .child(status),
                    )
                    .child(
                        div()
                            .ml_3()
                            .text_xs()
                            .text_color(rgb(ACCENT))
                            .child("● 읽기 전용"),
                    ),
            )
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;

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
