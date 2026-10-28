use crate::theme;
use gpui::{prelude::*, *};

pub const CHECKBOX_PADDING: f32 = 6.;

pub fn radio(checked: bool, label: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .size(px(14.))
                .flex_shrink_0()
                .rounded_full()
                .border_1()
                .border_color(rgb(theme::MUTED))
                .bg(rgb(theme::PANEL))
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |circle| {
                    circle.child(div().size(px(8.)).rounded_full().bg(rgb(theme::ACCENT)))
                }),
        )
        .child(label)
}

pub fn checkbox(checked: bool, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    window.paint_quad(quad(
                        bounds,
                        px(2.),
                        rgb(if checked { theme::SELECT } else { theme::PANEL }),
                        px(1.),
                        rgb(theme::MUTED),
                        Default::default(),
                    ));
                    if checked {
                        let mut path = PathBuilder::stroke(px(1.6));
                        path.move_to(bounds.origin + point(px(3.), px(7.)));
                        path.line_to(bounds.origin + point(px(6.), px(10.)));
                        path.line_to(bounds.origin + point(px(11.), px(4.)));
                        if let Ok(path) = path.build() {
                            window.paint_path(path, rgb(theme::ACCENT));
                        }
                    }
                },
            )
            .size(px(14.))
            .flex_shrink_0(),
        )
        .child(label.to_owned())
}
