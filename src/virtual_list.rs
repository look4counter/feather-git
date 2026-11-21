use crate::scrolling::{ScrollAxis, ScrollDrag, Thumb, Visibility, wheel_offsets};
use crate::theme::{ACCENT, BASE, LINE, MUTED, RAISED, SELECTED, TEXT};
use gpui::{
    AnyElement, App, Bounds, DispatchPhase, ElementId, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollHandle, ScrollWheelEvent, UniformList,
    UniformListScrollHandle, Window, div, point, prelude::*, px, rgb, size,
};
use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const ROW_HEIGHT: f32 = 24.;
const SCROLLBAR: f32 = 12.;

#[derive(Clone, Default)]
pub struct ListScrollHandle {
    pub vertical: UniformListScrollHandle,
    focus: Rc<RefCell<Option<gpui::FocusHandle>>>,
    pub horizontal: ScrollHandle,
    frame: Rc<Cell<Bounds<Pixels>>>,
    visible_clip: Rc<Cell<Option<Bounds<Pixels>>>>,
    vertical_track: Rc<Cell<Option<Bounds<Pixels>>>>,
    horizontal_track: Rc<Cell<Option<Bounds<Pixels>>>>,
    drag: Rc<Cell<Option<ScrollDrag>>>,
    suspended: Rc<Cell<bool>>,
    hovered: Rc<Cell<Option<ScrollAxis>>>,
}

impl ListScrollHandle {
    pub fn for_vertical_scroll(base: ScrollHandle) -> Self {
        let scroll = Self::default();
        scroll.vertical.0.borrow_mut().base_handle = base;
        scroll
    }

    pub fn vertical_scroll_view(&self, content: impl IntoElement) -> gpui::Div {
        let drag = self.clone();
        let wheel = self.clone();
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .pr(px(SCROLLBAR))
            .child(content)
            .child(
                self.scrollbar(ScrollAxis::Vertical, 0., Visibility::default(), 0.)
                    .child(
                        gpui::canvas(
                            |_, _, _| (),
                            move |_, _, window, _| {
                                drag.register_drag_events(window);
                            },
                        )
                        .absolute()
                        .size_full(),
                    )
                    .on_scroll_wheel(move |event, window, cx| {
                        if wheel.wheel(event, false) {
                            cx.stop_propagation();
                            window.refresh();
                        }
                    }),
            )
    }

    /// 새 내용의 위치를 초기화하고 측정된 뷰포트를 유지합니다.
    pub fn reset_for_content(&mut self) {
        *self = Self {
            frame: Rc::new(Cell::new(self.frame.get())),
            ..Self::default()
        };
    }

    pub fn set_suspended(&self, suspended: bool) {
        self.suspended.set(suspended);
        if suspended {
            self.cancel_drag();
        }
    }

    pub fn viewport_bounds(&self) -> Bounds<Pixels> {
        let mut bounds = self.frame.get();
        if self.vertical_track.get().is_some() {
            bounds.size.width = (bounds.size.width - px(SCROLLBAR)).max(px(0.));
        }
        if self.horizontal_track.get().is_some() {
            bounds.size.height = (bounds.size.height - px(SCROLLBAR)).max(px(0.));
        }
        bounds
    }

    pub fn is_visible_at(&self, position: Point<Pixels>) -> bool {
        self.frame.get().contains(&position)
            && self.visible_clip.get().is_none_or(|bounds| {
                bounds.size.width > px(0.)
                    && bounds.size.height > px(0.)
                    && bounds.contains(&position)
            })
    }

    pub fn is_scrollbar_at(&self, position: Point<Pixels>) -> bool {
        [self.vertical_track.get(), self.horizontal_track.get()]
            .into_iter()
            .flatten()
            .any(|b| b.contains(&position))
    }

    pub fn cancel_drag(&self) {
        self.drag.set(None);
        self.hovered.set(None);
    }

    fn base(&self, axis: ScrollAxis) -> ScrollHandle {
        match axis {
            ScrollAxis::Vertical => self.vertical.0.borrow().base_handle.clone(),
            ScrollAxis::Horizontal => self.horizontal.clone(),
        }
    }

    fn track(&self, axis: ScrollAxis) -> &Cell<Option<Bounds<Pixels>>> {
        match axis {
            ScrollAxis::Vertical => &self.vertical_track,
            ScrollAxis::Horizontal => &self.horizontal_track,
        }
    }

    fn hide(&self, axis: ScrollAxis) {
        self.track(axis).set(None);
        if self.hovered.get() == Some(axis) {
            self.hovered.set(None);
        }
        if self.drag.get().is_some_and(|drag| drag.axis == axis) {
            self.cancel_drag();
        }
    }

    fn thumb(&self, axis: ScrollAxis) -> Option<(Bounds<Pixels>, Thumb)> {
        let bounds = self.track(axis).get()?;
        let base = self.base(axis);
        let maximum = base.max_offset();
        let offset = base.offset();
        let (length, maximum, offset) = match axis {
            ScrollAxis::Vertical => (bounds.size.height, maximum.height, -offset.y),
            ScrollAxis::Horizontal => (bounds.size.width, maximum.width, -offset.x),
        };
        Some((
            bounds,
            Thumb::new(length.into(), maximum.into(), offset.into()),
        ))
    }

    fn local(axis: ScrollAxis, bounds: Bounds<Pixels>, position: Point<Pixels>) -> f32 {
        match axis {
            ScrollAxis::Vertical => (position.y - bounds.origin.y).into(),
            ScrollAxis::Horizontal => (position.x - bounds.origin.x).into(),
        }
    }

    fn begin_drag(&self, axis: ScrollAxis, position: Point<Pixels>) -> bool {
        if self.suspended.get() {
            return false;
        }
        let Some((bounds, thumb)) = self.thumb(axis) else {
            return false;
        };
        let Some(drag) = ScrollDrag::start(axis, thumb, Self::local(axis, bounds, position)) else {
            return false;
        };
        self.drag.set(Some(drag));
        self.drag_to(position);
        true
    }

    fn drag_to(&self, position: Point<Pixels>) -> bool {
        let Some(drag) = self.drag.get() else {
            return false;
        };
        let Some((bounds, thumb)) = self.thumb(drag.axis) else {
            self.cancel_drag();
            return false;
        };
        let Some(value) = drag.offset(thumb, Self::local(drag.axis, bounds, position)) else {
            return false;
        };
        let base = self.base(drag.axis);
        let mut offset = base.offset();
        match drag.axis {
            ScrollAxis::Vertical => offset.y = px(-value),
            ScrollAxis::Horizontal => offset.x = px(-value),
        }
        if offset == base.offset() {
            return false;
        }
        base.set_offset(offset);
        true
    }

    fn wheel(&self, event: &ScrollWheelEvent, horizontal: bool) -> bool {
        if self.suspended.get() {
            return false;
        }
        let vertical = self.base(ScrollAxis::Vertical);
        let old = [
            f32::from(self.horizontal.offset().x),
            f32::from(vertical.offset().y),
        ];
        let delta = event.delta.pixel_delta(px(ROW_HEIGHT));
        let updated = wheel_offsets(
            old,
            [
                self.horizontal.max_offset().width.into(),
                vertical.max_offset().height.into(),
            ],
            [delta.x.into(), delta.y.into()],
            event.modifiers.shift,
            horizontal,
        );
        if old == updated {
            return false;
        }
        self.horizontal.set_offset(point(px(updated[0]), px(0.)));
        vertical.set_offset(point(px(0.), px(updated[1])));
        true
    }

    fn register_drag_events(&self, window: &mut Window) {
        let state = self.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture || state.drag.get().is_none() {
                return;
            }
            if event.dragging() {
                state.drag_to(event.position);
            } else {
                state.cancel_drag();
            }
            cx.stop_propagation();
            window.refresh();
        });
        let state = self.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture
                || event.button != MouseButton::Left
                || state.drag.get().is_none()
            {
                return;
            }
            state.drag_to(event.position);
            let hovered = state.drag.get().and_then(|drag| {
                state
                    .track(drag.axis)
                    .get()
                    .filter(|bounds| bounds.contains(&event.position))
                    .map(|_| drag.axis)
            });
            state.drag.set(None);
            state.hovered.set(hovered);
            cx.stop_propagation();
            window.refresh();
        });
    }

    fn register_events(&self, window: &mut Window, horizontal: bool) {
        self.register_drag_events(window);
        let state = self.clone();
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture
                || state.suspended.get()
                || !state.is_visible_at(event.position)
            {
                return;
            }
            if state.wheel(event, horizontal) {
                window.refresh();
                cx.stop_propagation();
            }
            // 경계에서는 부모 스크롤 영역이 이어받아 여러 파일 사이를 이동한다.
        });
    }

    fn scrollbar(
        &self,
        axis: ScrollAxis,
        header_height: f32,
        visible: Visibility,
        horizontal_inset: f32,
    ) -> gpui::Stateful<gpui::Div> {
        let record = self.clone();
        let paint = self.clone();
        let press = self.clone();
        let hover = self.clone();
        div()
            .id(match axis {
                ScrollAxis::Vertical => "vertical-scroll",
                ScrollAxis::Horizontal => "horizontal-scroll",
            })
            .absolute()
            .when(visible.vertical || visible.horizontal, |bar| {
                bar.bg(rgb(BASE))
            })
            .cursor_pointer()
            .occlude()
            .when(axis == ScrollAxis::Vertical, |bar| {
                bar.right_0()
                    .top(px(header_height))
                    .bottom(px(if visible.horizontal { SCROLLBAR } else { 0. }))
                    .w(px(SCROLLBAR))
            })
            .when(axis == ScrollAxis::Horizontal, |bar| {
                bar.left(px(horizontal_inset))
                    .bottom_0()
                    .right(px(if visible.vertical { SCROLLBAR } else { 0. }))
                    .h(px(SCROLLBAR))
            })
            .child(
                gpui::canvas(
                    move |bounds, _, _| record.track(axis).set(Some(bounds)),
                    move |_, _, window, _| {
                        let Some((bounds, thumb)) = paint.thumb(axis) else {
                            return;
                        };
                        if thumb.maximum <= 0. {
                            return;
                        }
                        let rectangle = match axis {
                            ScrollAxis::Vertical => Bounds::new(
                                bounds.origin + point(px(2.), px(thumb.start)),
                                size(px(8.), px(thumb.length)),
                            ),
                            ScrollAxis::Horizontal => Bounds::new(
                                bounds.origin + point(px(thumb.start), px(2.)),
                                size(px(thumb.length), px(8.)),
                            ),
                        };
                        let color = if paint.drag.get().is_some_and(|drag| drag.axis == axis) {
                            ACCENT
                        } else if paint.hovered.get() == Some(axis) {
                            TEXT
                        } else {
                            MUTED
                        };
                        window.paint_quad(gpui::quad(
                            rectangle,
                            px(4.),
                            rgb(color),
                            px(0.),
                            rgb(LINE),
                            Default::default(),
                        ));
                    },
                )
                .absolute()
                .size_full(),
            )
            .on_hover(move |hovered, window, _| {
                if *hovered {
                    hover.hovered.set(Some(axis));
                } else if hover.hovered.get() == Some(axis) {
                    hover.hovered.set(None);
                }
                window.refresh();
            })
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if press.begin_drag(axis, event.position) {
                    cx.stop_propagation();
                    window.refresh();
                }
            })
    }
}

/// 목록별로 보존하며 요청 시작·결과 적용·무효화에 연결하는 지연 로딩 상태입니다.
#[derive(Clone, Default)]
pub struct ListLoading(Rc<RefCell<ListLoadingState>>);

#[derive(Default)]
struct ListLoadingState {
    loading: crate::loading::DelayedLoading,
    revision: Option<u64>,
    completed: Arc<AtomicBool>,
    timer: Option<gpui::Task<()>>,
}

impl ListLoading {
    pub fn begin(&self, revision: u64, now: Instant) {
        self.begin_retaining(revision);
        self.0.borrow_mut().loading.begin(revision, now);
    }

    /// 기존 내용을 보존하는 목록에서 안내 없이 입력만 차단합니다.
    pub fn begin_retaining(&self, revision: u64) {
        self.reset();
        self.0.borrow_mut().revision = Some(revision);
    }

    pub fn reset(&self) {
        // 이전 Task를 취소하고 이전 백엔드와 완료 신호를 분리한다.
        let mut state = self.0.borrow_mut();
        state.loading.reset();
        state.revision = None;
        state.completed = Arc::new(AtomicBool::new(false));
        state.timer = None;
    }

    pub fn completion(&self) -> Arc<AtomicBool> {
        self.0.borrow().completed.clone()
    }

    pub fn is_pending(&self) -> bool {
        self.0.borrow().revision.is_some()
    }

    pub fn is_visible(&self, revision: u64) -> bool {
        let state = self.0.borrow();
        !state.completed.load(Ordering::Acquire) && state.loading.is_visible(revision)
    }

    fn replaces_content(&self, keep_content: bool) -> bool {
        self.is_pending()
            && (!keep_content
                || self
                    .0
                    .borrow()
                    .revision
                    .is_some_and(|revision| self.is_visible(revision)))
    }

    pub(crate) fn reveal(&self, revision: u64, now: Instant) -> bool {
        let mut state = self.0.borrow_mut();
        !state.completed.load(Ordering::Acquire) && state.loading.reveal(revision, now)
    }

    fn schedule_with(
        &self,
        now: Instant,
        schedule: impl FnOnce(u64, Duration, Weak<RefCell<ListLoadingState>>) -> gpui::Task<()>,
    ) {
        let mut state = self.0.borrow_mut();
        let Some(revision) = state.revision else {
            return;
        };
        if state.timer.is_some() || state.completed.load(Ordering::Acquire) {
            return;
        }
        let Some(delay) = state.loading.remaining(now) else {
            return;
        };
        state.timer = Some(schedule(revision, delay, Rc::downgrade(&self.0)));
    }

    fn schedule(&self, window: &mut Window, cx: &mut App) {
        self.schedule_with(Instant::now(), |revision, delay, weak| {
            let timer = cx.background_executor().timer(delay);
            window.spawn(cx, async move |cx| {
                timer.await;
                let Some(state) = weak.upgrade() else {
                    return;
                };
                let loading = ListLoading(state);
                let _ = cx.update(|window, _| {
                    if loading.reveal(revision, Instant::now()) {
                        window.refresh();
                    }
                });
            })
        });
    }
}

#[derive(gpui::IntoElement)]
pub struct VirtualList {
    id: ElementId,
    list: UniformList,
    count: usize,
    scroll: Option<ListScrollHandle>,
    header: Option<AnyElement>,
    header_height: f32,
    minimum_width: f32,
    fixed_left_width: f32,
    loading: Option<ListLoading>,
    keep_loading_content: bool,
    keyboard: Option<(Option<usize>, Rc<ListSelect>)>,
}

type ListSelect = dyn Fn(usize, &MouseDownEvent, &mut Window, &mut App);

pub fn arrow_direction(key: &gpui::Keystroke) -> Option<bool> {
    let modifiers = key.modifiers;
    if modifiers.control
        || modifiers.alt
        || modifiers.shift
        || modifiers.platform
        || modifiers.function
    {
        return None;
    }
    match key.key.as_str() {
        "up" => Some(false),
        "down" => Some(true),
        _ => None,
    }
}

pub fn adjacent_selection(selected: Option<usize>, count: usize, down: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match selected.filter(|index| *index < count) {
        None => Some(0),
        Some(index) => Some(if down {
            index.saturating_add(1).min(count - 1)
        } else {
            index.saturating_sub(1)
        }),
    }
}

impl VirtualList {
    pub fn keyboard_selected(mut self, selected: Option<usize>) -> Self {
        if let Some((current, _)) = &mut self.keyboard {
            *current = selected;
        }
        self
    }

    pub fn keyboard_selection(mut self, enabled: bool) -> Self {
        if !enabled {
            self.keyboard = None;
        }
        self
    }

    /// 요청 상태를 연결하면 목록이 150ms 지연 안내와 타이머를 관리합니다.
    pub fn loading(mut self, loading: ListLoading) -> Self {
        self.loading = Some(loading);
        self
    }

    pub fn keep_content_while_loading(mut self, keep: bool) -> Self {
        self.keep_loading_content = keep;
        self
    }

    pub fn scroll_handle(mut self, scroll: ListScrollHandle) -> Self {
        self.scroll = Some(scroll);
        self
    }
    pub fn header(mut self, header: impl IntoElement, height: f32) -> Self {
        self.header = Some(header.into_any_element());
        self.header_height = height;
        self
    }
    /// 고정 모드에서는 호출자가 본문에만 horizontal 오프셋을 적용해야 합니다.
    pub fn fixed_left_width(mut self, width: f32) -> Self {
        self.fixed_left_width = width.max(0.);
        self
    }

    pub fn min_content_width(mut self, width: f32) -> Self {
        self.minimum_width = width.max(0.);
        self
    }
}

/// 패널 본문의 상태 안내를 같은 크기로 가로·세로 중앙에 표시합니다.
pub(crate) fn panel_message(message: impl Into<gpui::SharedString>) -> gpui::Div {
    div()
        .flex()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .w_full()
        .items_center()
        .justify_center()
        .text_center()
        .px_3()
        .py_2()
        .text_size(px(12.))
        .text_color(rgb(MUTED))
        .child(message.into())
}

impl RenderOnce for VirtualList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll = self.scroll.unwrap_or_default();
        if let Some(loading) = &self.loading {
            loading.schedule(window, cx);
        }
        if let Some(loading) = &self.loading
            && loading.replaces_content(self.keep_loading_content)
        {
            scroll.hide(ScrollAxis::Vertical);
            scroll.hide(ScrollAxis::Horizontal);
            scroll.cancel_drag();
            let visible = loading
                .0
                .borrow()
                .revision
                .is_some_and(|revision| loading.is_visible(revision));
            return div()
                .id(self.id)
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .w_full()
                .overflow_hidden()
                .when_some(self.header, |column, header| column.child(header))
                .when(visible, |column| {
                    column.child(panel_message("조회 중 입니다.."))
                });
        }
        let focus = scroll
            .focus
            .borrow_mut()
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let mouse_focus = focus.clone();
        let key_focus = focus.clone();
        let key_scroll = scroll.clone();
        let pending = self.loading.as_ref().is_some_and(ListLoading::is_pending);
        let keyboard = self.keyboard.filter(|_| !pending);
        let keyboard_enabled = keyboard.is_some();
        let count = self.count;
        let frame = scroll.frame.get();
        let horizontal = self.minimum_width > 0.;
        let visible = Visibility::new(
            frame.size.width.into(),
            f32::from(frame.size.height) - self.header_height,
            self.minimum_width,
            self.count as f32 * ROW_HEIGHT,
            horizontal,
            SCROLLBAR,
        );
        if !visible.vertical {
            scroll.hide(ScrollAxis::Vertical);
        }
        if !visible.horizontal {
            scroll.hide(ScrollAxis::Horizontal);
        }
        let fixed = self.fixed_left_width > 0.;
        let [inset, body_width] = fixed_left_geometry(
            f32::from(frame.size.width) - if visible.vertical { SCROLLBAR } else { 0. },
            self.minimum_width,
            self.fixed_left_width,
        );
        let record = scroll.clone();
        let events = scroll.clone();
        div()
            .id(self.id)
            .track_focus(&focus)
            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                if keyboard_enabled {
                    mouse_focus.focus(window);
                }
            })
            .on_key_down(move |event, window, cx| {
                if !key_focus.is_focused(window)
                    || key_scroll.suspended.get()
                    || !window.is_window_active()
                {
                    return;
                }
                let Some(down) = arrow_direction(&event.keystroke) else {
                    return;
                };
                let Some((selected, on_select)) = &keyboard else {
                    return;
                };
                if let Some(index) = adjacent_selection(*selected, count, down) {
                    if Some(index) != *selected {
                        on_select(
                            index,
                            &MouseDownEvent {
                                button: MouseButton::Left,
                                click_count: 1,
                                ..Default::default()
                            },
                            window,
                            cx,
                        );
                    }
                    key_scroll
                        .vertical
                        .scroll_to_item(index, gpui::ScrollStrategy::Top);
                    key_focus.focus(window);
                    window.refresh();
                }
                cx.stop_propagation();
            })
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .w_full()
            .overflow_hidden()
            .child(
                gpui::canvas(
                    move |bounds, window, _| {
                        if record.frame.get() != bounds {
                            record.frame.set(bounds);
                            window.on_next_frame(|window, _| window.refresh());
                        }
                    },
                    move |bounds, _, window, _| {
                        events
                            .visible_clip
                            .set(Some(bounds.intersect(&window.content_mask().bounds)));
                        events.register_events(window, horizontal);
                    },
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .pr(px(if visible.vertical { SCROLLBAR } else { 0. }))
                    .pb(px(if visible.horizontal { SCROLLBAR } else { 0. }))
                    .when(fixed, |frame| {
                        // 범위 계산은 기존 핸들에 맡기고 실제 목록의 위치는 고정한다.
                        frame.child(
                            div()
                                .id("list-horizontal-extent")
                                .absolute()
                                .left(px(inset))
                                .right(px(if visible.vertical { SCROLLBAR } else { 0. }))
                                .top_0()
                                .bottom(px(if visible.horizontal { SCROLLBAR } else { 0. }))
                                .overflow_x_scroll()
                                .map(|mut viewport| {
                                    viewport.style().restrict_scroll_to_axis = Some(true);
                                    viewport
                                })
                                .track_scroll(&scroll.horizontal)
                                .child(div().w(px(body_width)).h(px(1.)).flex_shrink_0()),
                        )
                    })
                    .child(
                        div()
                            .id("list-viewport")
                            .relative()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_hidden()
                            .when(!fixed, |viewport| {
                                viewport
                                    .when(horizontal, |mut viewport| {
                                        viewport.style().restrict_scroll_to_axis = Some(true);
                                        viewport.overflow_x_scroll()
                                    })
                                    .track_scroll(&scroll.horizontal)
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_h_0()
                                    .w_full()
                                    .min_w(px(if fixed { 0. } else { self.minimum_width }))
                                    .overflow_hidden()
                                    .when_some(self.header, |column, header| column.child(header))
                                    .child(self.list.track_scroll(scroll.vertical.clone())),
                            ),
                    ),
            )
            .when(visible.vertical, |frame| {
                frame.child(scroll.scrollbar(ScrollAxis::Vertical, self.header_height, visible, 0.))
            })
            .when(visible.horizontal, |frame| {
                frame.child(scroll.scrollbar(
                    ScrollAxis::Horizontal,
                    self.header_height,
                    visible,
                    inset,
                ))
            })
            .when(pending, |frame| {
                frame.child(
                    div()
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation()),
                )
            })
    }
}

fn fixed_left_geometry(viewport: f32, content: f32, fixed_left: f32) -> [f32; 2] {
    let inset = fixed_left.max(0.).min(viewport.max(0.));
    [inset, (content - inset).max(0.)]
}

/// 가시 범위 렌더링·공통 행 선택·스크롤바와 마우스 조작을 제공하는 목록입니다.
/// 같은 목록의 스크롤 핸들을 프레임 사이에 보존해야 합니다.
pub fn virtual_list<R: IntoElement>(
    id: impl Into<ElementId>,
    item_count: usize,
    selected: Option<usize>,
    background: u32,
    render: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
    on_select: impl 'static + Fn(usize, &mut Window, &mut App),
) -> VirtualList {
    multi_virtual_list(
        id,
        item_count,
        selected.into_iter().collect(),
        background,
        render,
        move |index, _, window, cx| on_select(index, window, cx),
    )
}

pub fn multi_virtual_list<R: IntoElement>(
    id: impl Into<ElementId>,
    item_count: usize,
    selected: std::collections::BTreeSet<usize>,
    background: u32,
    render: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
    on_select: impl 'static + Fn(usize, &MouseDownEvent, &mut Window, &mut App),
) -> VirtualList {
    let id = id.into();
    let on_select = Rc::new(on_select);
    let selected: std::collections::BTreeSet<_> = selected
        .into_iter()
        .filter(|index| *index < item_count)
        .collect();
    let keyboard = Some((
        selected.iter().next_back().copied(),
        on_select.clone() as Rc<ListSelect>,
    ));
    let list = gpui::uniform_list(id.clone(), item_count, move |range, window, cx| {
        let start = range.start;
        render_visible(item_count, range, |range| render(range, window, cx))
            .into_iter()
            .enumerate()
            .map(|(offset, content)| {
                let index = start + offset;
                let on_select = on_select.clone();
                selectable_row(
                    index,
                    selected.contains(&index),
                    background,
                    content,
                    move |event, window, cx| on_select(index, event, window, cx),
                )
            })
            .collect::<Vec<_>>()
    })
    .with_horizontal_sizing_behavior(gpui::ListHorizontalSizingBehavior::FitList)
    .w_full()
    .flex_1()
    .min_h_0();
    VirtualList {
        id,
        list,
        count: item_count,
        scroll: None,
        header: None,
        header_height: 0.,
        minimum_width: 0.,
        fixed_left_width: 0.,
        loading: None,
        keep_loading_content: true,
        keyboard,
    }
}

pub fn selectable_row<R: IntoElement>(
    id: impl Into<ElementId>,
    selected: bool,
    background: u32,
    content: R,
    on_select: impl 'static + Fn(&MouseDownEvent, &mut Window, &mut App),
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w_full()
        .h(px(ROW_HEIGHT))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(rgb(if selected { SELECTED } else { background }))
        .cursor_pointer()
        .when(!selected, |row| row.hover(|style| style.bg(rgb(RAISED))))
        .child(content)
        .on_mouse_down(MouseButton::Left, on_select)
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct MultiSelection {
    indices: std::collections::BTreeSet<usize>,
    anchor: Option<usize>,
    active: Option<usize>,
}

impl MultiSelection {
    pub fn indices(&self, count: usize) -> Vec<usize> {
        self.indices
            .iter()
            .copied()
            .filter(|index| *index < count)
            .collect()
    }
    pub fn selected(&self, count: usize) -> Option<usize> {
        self.active
            .filter(|index| *index < count && self.indices.contains(index))
            .or_else(|| self.indices.range(..count).next_back().copied())
    }
    pub fn select(&mut self, index: usize, count: usize) -> bool {
        if index >= count {
            return false;
        }
        let changed = self.indices != [index].into()
            || self.anchor != Some(index)
            || self.active != Some(index);
        self.indices = [index].into();
        self.anchor = Some(index);
        self.active = Some(index);
        changed
    }
    pub fn click(&mut self, index: usize, count: usize, shift: bool, control: bool) -> bool {
        if index >= count {
            return false;
        }
        if !shift && !control {
            return self.select(index, count);
        }
        let before = self.clone();
        self.indices.retain(|row| *row < count);
        if shift {
            let anchor = self
                .anchor
                .filter(|row| *row < count)
                .or(self.selected(count))
                .unwrap_or(index);
            if !control {
                self.indices.clear();
            }
            self.indices.extend(anchor.min(index)..=anchor.max(index));
            self.anchor = Some(anchor);
        } else {
            if !self.indices.insert(index) {
                self.indices.remove(&index);
            }
            self.anchor = Some(index);
        }
        self.active = Some(index);
        *self != before
    }
    pub fn restore(&mut self, rows: impl IntoIterator<Item = usize>, count: usize) {
        self.indices = rows.into_iter().filter(|row| *row < count).collect();
        self.anchor = self.indices.first().copied();
        self.active = self.indices.last().copied();
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Default)]
pub struct SingleSelection {
    index: Option<usize>,
}

impl SingleSelection {
    pub fn selected(&self, item_count: usize) -> Option<usize> {
        self.index.filter(|index| *index < item_count)
    }

    pub fn select(&mut self, index: usize, item_count: usize) -> bool {
        if index >= item_count || self.index == Some(index) {
            return false;
        }
        self.index = Some(index);
        true
    }

    pub fn clear(&mut self) {
        self.index = None;
    }
}

fn render_visible<R>(
    item_count: usize,
    range: Range<usize>,
    render: impl FnOnce(Range<usize>) -> Vec<R>,
) -> Vec<R> {
    let start = range.start.min(item_count);
    let end = range.end.min(item_count);
    if start >= end {
        Vec::new()
    } else {
        render(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn multi_selection_handles_click_ranges_control_and_restore() {
        let mut selection = MultiSelection::default();
        assert!(selection.click(2, 10, false, false));
        assert_eq!(selection.indices(10), [2]);
        selection.click(5, 10, true, false);
        assert_eq!(selection.indices(10), [2, 3, 4, 5]);
        selection.click(0, 10, true, false);
        assert_eq!(selection.indices(10), [0, 1, 2]);
        selection.click(7, 10, false, true);
        assert_eq!(selection.indices(10), [0, 1, 2, 7]);
        selection.click(9, 10, true, true);
        assert_eq!(selection.indices(10), [0, 1, 2, 7, 8, 9]);
        selection.click(1, 10, false, true);
        assert_eq!(selection.indices(10), [0, 2, 7, 8, 9]);
        let before = selection.clone();
        assert!(!selection.click(99, 10, true, true));
        assert_eq!(selection, before);
        selection.click(4, 10, false, false);
        assert_eq!(selection.indices(10), [4]);
        selection.click(4, 10, false, true);
        assert!(selection.indices(10).is_empty());
        assert_eq!(selection.selected(10), None);
        selection.clear();
        selection.click(3, 10, true, false);
        assert_eq!(selection.indices(10), [3]);
        selection.restore([8, 2, 2, 99], 10);
        assert_eq!(selection.indices(10), [2, 8]);
        assert_eq!(selection.selected(10), Some(8));
        assert_eq!(selection.indices(3), [2]);
        selection.clear();
        assert_eq!(selection.selected(10), None);
        assert!(!selection.click(0, 0, false, false));
    }

    #[test]
    fn renders_only_requested_rows_in_large_list() {
        let calls = Cell::new(0);
        let rows = render_visible(100_000, 50_000..50_020, |range| {
            range
                .inspect(|_| {
                    calls.set(calls.get() + 1);
                })
                .collect()
        });
        assert_eq!(calls.get(), 20);
        assert_eq!(rows, (50_000..50_020).collect::<Vec<_>>());
    }

    #[test]
    fn skips_empty_and_outdated_ranges() {
        for (count, range) in [
            (0, 0..20),
            (10, 4..4),
            (2, 50..60),
            (10, Range { start: 8, end: 3 }),
        ] {
            let rows: Vec<usize> =
                render_visible(count, range, |_| panic!("빈 범위는 렌더링하면 안 됩니다"));
            assert!(rows.is_empty());
        }
    }

    #[test]
    fn clamps_last_page_and_single_item() {
        assert_eq!(
            render_visible(1, 0..20, |range| range.collect::<Vec<_>>()),
            vec![0]
        );
        assert_eq!(
            render_visible(100_000, 99_998..usize::MAX, |range| range
                .collect::<Vec<_>>()),
            vec![99_998, 99_999]
        );
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    #[test]
    fn single_selection_moves_repeats_and_clears() {
        let mut selection = SingleSelection::default();
        assert_eq!(selection.selected(3), None);
        assert!(selection.select(1, 3));
        assert_eq!(selection.selected(3), Some(1));
        assert!(!selection.select(1, 3));
        assert!(selection.select(2, 3));
        assert_eq!(selection.selected(3), Some(2));
        selection.clear();
        assert_eq!(selection.selected(3), None);
    }

    #[test]
    fn invalid_selection_and_shrinking_data_are_safe() {
        let mut selection = SingleSelection::default();
        assert!(!selection.select(0, 0));
        assert!(!selection.select(3, 3));
        assert!(selection.select(2, 3));
        assert_eq!(selection.selected(2), None);
        assert!(!selection.select(usize::MAX, 3));
        assert_eq!(selection.selected(3), Some(2));
    }
}

#[cfg(test)]
mod scroll_handle_tests {
    use super::*;

    #[test]
    fn content_reset_keeps_measured_viewport_without_a_scrollbarless_frame() {
        let mut scroll = ListScrollHandle::default();
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(400.), px(200.)));
        scroll.frame.set(bounds);
        scroll.horizontal.set_offset(point(px(-70.), px(0.)));
        scroll
            .vertical
            .scroll_to_item(12, gpui::ScrollStrategy::Top);
        scroll.vertical_track.set(Some(bounds));
        scroll.horizontal_track.set(Some(bounds));
        scroll.hovered.set(Some(ScrollAxis::Horizontal));
        scroll.drag.set(ScrollDrag::start(
            ScrollAxis::Horizontal,
            Thumb::new(200., 1000., 0.),
            3.,
        ));
        let old_horizontal = scroll.horizontal.clone();
        let old_vertical = scroll.vertical.clone();
        scroll.reset_for_content();
        assert_eq!(scroll.frame.get(), bounds);
        assert_eq!(old_horizontal.offset(), point(px(-70.), px(0.)));
        assert!(!Rc::ptr_eq(&scroll.vertical.0, &old_vertical.0));
        assert_eq!(scroll.horizontal.offset(), point(px(0.), px(0.)));
        assert!(scroll.vertical.0.borrow().deferred_scroll_to_item.is_none());
        assert!(scroll.vertical_track.get().is_none());
        assert!(scroll.horizontal_track.get().is_none());
        assert!(scroll.drag.get().is_none());
        assert!(scroll.hovered.get().is_none());
        let frame = scroll.frame.get();
        assert_eq!(
            Visibility::new(
                frame.size.width.into(),
                frame.size.height.into(),
                800.,
                600.,
                true,
                SCROLLBAR
            ),
            Visibility {
                vertical: true,
                horizontal: true
            }
        );
        assert_eq!(
            Visibility::new(
                frame.size.width.into(),
                frame.size.height.into(),
                100.,
                100.,
                true,
                SCROLLBAR
            ),
            Visibility::default()
        );
        let mut unmeasured = ListScrollHandle::default();
        unmeasured.reset_for_content();
        assert_eq!(unmeasured.frame.get(), Bounds::default());
    }

    #[test]
    fn clipped_list_rejects_hidden_input_without_changing_row_coordinates() {
        let scroll = ListScrollHandle::default();
        let frame = Bounds::new(point(px(10.), px(20.)), size(px(100.), px(200.)));
        scroll.frame.set(frame);
        assert!(scroll.is_visible_at(point(px(20.), px(25.))));
        scroll.visible_clip.set(Some(Bounds::new(
            point(px(10.), px(40.)),
            size(px(100.), px(80.)),
        )));
        assert!(!scroll.is_visible_at(point(px(20.), px(25.))));
        assert!(scroll.is_visible_at(point(px(20.), px(80.))));
        assert!(!scroll.is_visible_at(point(px(20.), px(150.))));
        assert_eq!(scroll.viewport_bounds(), frame);
        scroll.visible_clip.set(Some(Bounds::new(
            point(px(10.), px(40.)),
            size(px(0.), px(80.)),
        )));
        assert!(!scroll.is_visible_at(point(px(10.), px(80.))));
    }

    #[test]
    fn fixed_gutter_preserves_scroll_range_and_limits_bar_to_editor() {
        for gutter in [0., 116., 140.] {
            let [inset, content] = fixed_left_geometry(500., 1200., gutter);
            let editor = 500. - inset;
            assert_eq!(inset, gutter);
            assert_eq!(content - editor, 700.);
            let thumb = Thumb::new(editor, content - editor, 350.);
            assert_eq!(thumb.maximum, 700.);
            assert!((thumb.offset(thumb.start) - 350.).abs() < 0.001);
        }
        assert_eq!(fixed_left_geometry(50., 1000., 140.), [50., 950.]);
        assert_eq!(fixed_left_geometry(0., 1000., 140.), [0., 1000.]);
        assert_eq!(fixed_left_geometry(500., 20., 140.), [140., 0.]);
    }

    #[test]
    fn clones_share_state_but_different_lists_are_independent() {
        let first = ListScrollHandle::default();
        let clone = first.clone();
        let other = ListScrollHandle::default();
        assert!(Rc::ptr_eq(&first.vertical.0, &clone.vertical.0));
        assert!(!Rc::ptr_eq(&first.vertical.0, &other.vertical.0));
        assert!(Rc::ptr_eq(&first.frame, &clone.frame));
        assert!(!Rc::ptr_eq(&first.frame, &other.frame));
        first.horizontal.set_offset(point(px(-80.), px(0.)));
        assert_eq!(clone.horizontal.offset().x, px(-80.));
        assert_eq!(other.horizontal.offset().x, px(0.));
    }

    #[test]
    fn cancellation_and_disappearing_axis_clear_only_active_drag() {
        let state = ListScrollHandle::default();
        state.vertical_track.set(Some(Bounds::default()));
        state.horizontal_track.set(Some(Bounds::default()));
        state.drag.set(ScrollDrag::start(
            ScrollAxis::Vertical,
            Thumb::new(300., 900., 0.),
            10.,
        ));
        state.hovered.set(Some(ScrollAxis::Vertical));
        state.hide(ScrollAxis::Horizontal);
        assert!(state.drag.get().is_some());
        state.hide(ScrollAxis::Vertical);
        assert!(state.drag.get().is_none());
        assert_eq!(state.hovered.get(), None);
        state.drag.set(ScrollDrag::start(
            ScrollAxis::Horizontal,
            Thumb::new(300., 900., 0.),
            10.,
        ));
        state.cancel_drag();
        assert!(state.drag.get().is_none());
        state.hovered.set(Some(ScrollAxis::Horizontal));
        state.hide(ScrollAxis::Horizontal);
        assert_eq!(state.hovered.get(), None);
    }
    #[test]
    fn overlay_suspension_is_shared_cancels_drag_and_preserves_scroll() {
        let handle = ListScrollHandle::default();
        let clone = handle.clone();
        handle.horizontal.set_offset(point(px(-40.), px(0.)));
        handle.drag.set(ScrollDrag::start(
            ScrollAxis::Vertical,
            Thumb::new(300., 900., 0.),
            10.,
        ));
        handle.set_suspended(true);
        assert!(clone.suspended.get());
        assert!(clone.drag.get().is_none());
        assert!(!handle.begin_drag(ScrollAxis::Vertical, Point::default()));
        assert_eq!(handle.horizontal.offset().x, px(-40.));
        clone.set_suspended(false);
        assert!(!handle.suspended.get());
    }
}

#[cfg(test)]
mod loading_tests {
    use super::*;
    use crate::loading::LOADING_DELAY;
    use std::{
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };

    #[test]
    fn repeated_renders_schedule_once_and_new_request_resets_timer() {
        let loading = ListLoading::default();
        let start = Instant::now();
        loading.schedule_with(start, |_, _, _| panic!("대기 요청 없음"));
        loading.begin(1, start);
        let scheduled = Cell::new(0);
        for _ in 0..10 {
            loading.schedule_with(
                start + Duration::from_millis(50),
                |revision, delay, weak| {
                    assert_eq!(revision, 1);
                    assert_eq!(delay, Duration::from_millis(100));
                    assert!(weak.upgrade().is_some());
                    scheduled.set(scheduled.get() + 1);
                    gpui::Task::ready(())
                },
            );
        }
        assert_eq!(scheduled.get(), 1);
        loading.begin(2, start);
        loading.schedule_with(start, |revision, delay, _| {
            assert_eq!(revision, 2);
            assert_eq!(delay, LOADING_DELAY);
            scheduled.set(scheduled.get() + 1);
            gpui::Task::ready(())
        });
        assert_eq!(scheduled.get(), 2);
        loading.reset();
        assert!(loading.0.borrow().timer.is_none());
        loading.schedule_with(start, |_, _, _| panic!("초기화 뒤 예약됨"));
        loading.begin(3, start);
        loading.completion().store(true, Ordering::Release);
        loading.schedule_with(start, |_, _, _| panic!("완료 뒤 예약됨"));
        let weak = Rc::downgrade(&loading.0);
        drop(loading);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn retained_refresh_blocks_selection_without_replacing_content() {
        let loading = ListLoading::default();
        let now = Instant::now();
        loading.begin_retaining(1);
        assert!(loading.is_pending());
        assert!(!loading.reveal(1, now + crate::loading::LOADING_DELAY));
        assert!(!loading.replaces_content(true));
        loading.reset();
        assert!(!loading.is_pending());
    }

    #[test]
    fn retained_content_is_replaced_only_when_loading_message_is_visible() {
        let loading = ListLoading::default();
        let start = Instant::now();
        assert!(!loading.replaces_content(true));
        loading.begin(1, start);
        assert!(loading.replaces_content(false));
        assert!(!loading.replaces_content(true));
        assert!(!loading.reveal(1, start + Duration::from_millis(149)));
        assert!(!loading.replaces_content(true));
        assert!(loading.reveal(1, start + LOADING_DELAY));
        assert!(loading.replaces_content(true));
        loading.completion().store(true, Ordering::Release);
        assert!(!loading.replaces_content(true));
        loading.reset();
        assert!(!loading.replaces_content(true));
        loading.begin(2, start);
        assert!(!loading.replaces_content(true));
    }

    #[test]
    fn list_loading_delays_message_and_resets_after_completion() {
        let loading = ListLoading::default();
        let start = Instant::now();
        assert!(!loading.is_pending());
        loading.begin(1, start);
        assert!(loading.is_pending());
        assert!(!loading.reveal(1, start + Duration::from_millis(149)));
        assert!(!loading.is_visible(1));
        assert!(loading.reveal(1, start + LOADING_DELAY));
        assert!(loading.is_visible(1));
        loading.completion().store(true, Ordering::Release);
        assert!(!loading.is_visible(1));
        loading.reset();
        assert!(!loading.is_pending());
        assert!(!loading.reveal(1, start + LOADING_DELAY));
        assert!(!loading.is_visible(1));
    }

    #[test]
    fn clones_share_but_lists_and_previous_request_completions_are_independent() {
        let first = ListLoading::default();
        let clone = first.clone();
        let other = ListLoading::default();
        let start = Instant::now();
        first.begin(1, start);
        other.begin(1, start);
        let old_completion = first.completion();
        first.begin(2, start + LOADING_DELAY);
        old_completion.store(true, Ordering::Release);
        assert!(!clone.reveal(1, start + LOADING_DELAY + LOADING_DELAY));
        assert!(clone.reveal(2, start + LOADING_DELAY + LOADING_DELAY));
        assert!(first.is_visible(2));
        assert!(!other.is_visible(1));
        assert!(other.reveal(1, start + LOADING_DELAY));
        first.reset();
        assert!(!clone.is_pending());
        assert!(other.is_visible(1));
    }

    #[test]
    fn fast_backend_completion_prevents_message_before_ui_applies_result() {
        let loading = ListLoading::default();
        let start = Instant::now();
        loading.begin(3, start);
        loading.completion().store(true, Ordering::Release);
        assert!(!loading.reveal(3, start + LOADING_DELAY));
        assert!(!loading.is_visible(3));
        // UI 결과 적용 전에는 빈 결과나 이전 행을 표시하지 않는다.
        assert!(loading.is_pending());
        loading.reset();
        loading.begin(4, start);
        assert!(!loading.completion().load(Ordering::Acquire));
        assert!(loading.reveal(4, start + LOADING_DELAY));
    }
}

#[cfg(test)]
mod keyboard_tests {
    use super::*;
    #[test]
    fn arrows_require_plain_keys_and_selection_stops_at_boundaries() {
        assert_eq!(
            arrow_direction(&gpui::Keystroke::parse("up").unwrap()),
            Some(false)
        );
        assert_eq!(
            arrow_direction(&gpui::Keystroke::parse("down").unwrap()),
            Some(true)
        );
        for key in ["shift-up", "ctrl-down", "alt-up", "enter"] {
            assert_eq!(arrow_direction(&gpui::Keystroke::parse(key).unwrap()), None);
        }
        assert_eq!(adjacent_selection(None, 0, true), None);
        assert_eq!(adjacent_selection(None, 3, false), Some(0));
        assert_eq!(adjacent_selection(Some(0), 3, false), Some(0));
        assert_eq!(adjacent_selection(Some(2), 3, true), Some(2));
        assert_eq!(adjacent_selection(Some(1), 3, true), Some(2));
        assert_eq!(adjacent_selection(Some(1), 3, false), Some(0));
        assert_eq!(adjacent_selection(Some(99), 3, true), Some(0));
    }
}
