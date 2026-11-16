use crate::{text_input::TextInput, theme::*};
use gpui::{prelude::*, *};

pub fn input(editor: Entity<TextInput>) -> Div {
    div()
        .w_full()
        .min_w_0()
        .h(px(34.))
        .min_h(px(34.))
        .border_1()
        .border_color(rgb(LINE))
        .rounded_md()
        .overflow_hidden()
        .child(editor)
}

pub fn labeled_input(label: &str, editor: Entity<TextInput>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .min_w_0()
        .child(
            div()
                .text_size(px(13.))
                .text_align(TextAlign::Left)
                .child(label.to_owned()),
        )
        .child(input(editor))
}

pub fn checkbox_click(event: &ClickEvent) -> bool {
    matches!(event, ClickEvent::Mouse(_))
}

pub fn checkbox(
    id: impl Into<ElementId>,
    checked: bool,
    label: &str,
    enabled: bool,
) -> Stateful<Div> {
    let mut row = div()
        .id(id)
        .flex_shrink_0()
        .h(px(32.))
        .flex()
        .items_center()
        .justify_start()
        .gap_2()
        .text_align(TextAlign::Left)
        .text_size(px(13.))
        .text_color(rgb(TEXT))
        .when(enabled, |row| row.cursor_pointer())
        .when(!enabled, |row| row.opacity(0.45))
        .child(checkbox_mark(checked))
        .child(
            div()
                .min_w_0()
                .text_align(TextAlign::Left)
                .child(label.to_owned()),
        );
    row.style().align_self = Some(AlignSelf::FlexStart);
    row
}

pub fn checkbox_mark(checked: bool) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            window.paint_quad(quad(
                bounds,
                px(2.),
                rgb(if checked { SELECTED } else { PANEL }),
                px(1.),
                rgb(MUTED),
                Default::default(),
            ));
            if checked {
                let mut path = PathBuilder::stroke(px(1.6));
                path.move_to(bounds.origin + point(px(3.), px(7.)));
                path.line_to(bounds.origin + point(px(6.), px(10.)));
                path.line_to(bounds.origin + point(px(11.), px(4.)));
                if let Ok(path) = path.build() {
                    window.paint_path(path, rgb(ACCENT));
                }
            }
        },
    )
    .size(px(14.))
    .flex_shrink_0()
}

pub fn trash_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.5));
            let origin = bounds.origin;
            path.move_to(origin + point(px(2.), px(4.)));
            path.line_to(origin + point(px(14.), px(4.)));
            path.move_to(origin + point(px(6.), px(4.)));
            path.line_to(origin + point(px(6.), px(2.)));
            path.line_to(origin + point(px(10.), px(2.)));
            path.line_to(origin + point(px(10.), px(4.)));
            path.move_to(origin + point(px(4.), px(6.)));
            path.line_to(origin + point(px(5.), px(14.)));
            path.line_to(origin + point(px(11.), px(14.)));
            path.line_to(origin + point(px(12.), px(6.)));
            for x in [7., 9.] {
                path.move_to(origin + point(px(x), px(7.)));
                path.line_to(origin + point(px(x), px(12.)));
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
        },
    )
    .size(px(16.))
    .flex_shrink_0()
}

pub fn list_row(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .w_full()
        .flex_shrink_0()
        .min_w_0()
        .p_3()
        .gap_3()
        .rounded_md()
}
pub fn list_title(label: impl Into<SharedString>) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .min_h(px(20.))
        .line_height(px(20.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT))
        .truncate()
        .child(label.into())
}
pub fn list_detail(label: impl Into<SharedString>) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .min_h(px(18.))
        .line_height(px(18.))
        .text_size(px(11.))
        .text_color(rgb(MUTED))
        .truncate()
        .child(label.into())
}
pub fn list_delete(id: impl Into<ElementId>, enabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(30.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .when(enabled, |button| {
            button.cursor_pointer().hover(|style| style.bg(rgb(RAISED)))
        })
        .when(!enabled, |button| button.opacity(0.45))
        .child(trash_icon())
}
pub fn folder_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.5));
            let p = |x, y| bounds.origin + point(px(x), px(y));
            path.move_to(p(1., 13.));
            path.line_to(p(1., 3.));
            path.line_to(p(6., 3.));
            path.line_to(p(8., 5.));
            path.line_to(p(15., 5.));
            path.line_to(p(15., 13.));
            path.close();
            path.move_to(p(1., 7.));
            path.line_to(p(15., 7.));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
        },
    )
    .size(px(16.))
    .flex_shrink_0()
}

pub fn text_area(editor: Entity<TextInput>) -> Div {
    div()
        .w_full()
        .min_w_0()
        .h(px(180.))
        .min_h(px(180.))
        .border_1()
        .border_color(rgb(LINE))
        .rounded_md()
        .overflow_hidden()
        .child(editor)
}

pub fn navigable_list_row(
    id: impl Into<ElementId>,
    index: usize,
    handles: Vec<FocusHandle>,
    window: &Window,
    enabled: bool,
    scroll: ScrollHandle,
) -> Stateful<Div> {
    let focus = handles[index].clone();
    let click_focus = focus.clone();
    list_row(id)
        .track_focus(&focus)
        .relative()
        .when(focus.is_focused(window), |row| {
            row.child(
                gpui::canvas(
                    move |bounds, window, _| {
                        let viewport = scroll.bounds();
                        if viewport.size.height <= px(0.) {
                            return;
                        }
                        let delta = if bounds.top() < viewport.top() {
                            viewport.top() - bounds.top()
                        } else if bounds.bottom() > viewport.bottom() {
                            viewport.bottom() - bounds.bottom()
                        } else {
                            px(0.)
                        };
                        if delta != px(0.) {
                            let mut offset = scroll.offset();
                            offset.y += delta;
                            scroll.set_offset(offset);
                            window.on_next_frame(|window, _| window.refresh());
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
        })
        .when(focus.is_focused(window), |row| {
            row.bg(rgb(crate::theme::SELECTED))
        })
        .on_mouse_down(MouseButton::Left, move |_, window, _| {
            if enabled {
                click_focus.focus(window);
                window.refresh();
            }
        })
        .on_key_down(move |event, window, cx| {
            if !enabled || !focus.contains_focused(window, cx) {
                return;
            }
            if let Some(down) = crate::virtual_list::arrow_direction(&event.keystroke) {
                if let Some(next) =
                    crate::virtual_list::adjacent_selection(Some(index), handles.len(), down)
                {
                    handles[next].focus(window);
                    window.refresh();
                }
                cx.stop_propagation();
            }
        })
}
