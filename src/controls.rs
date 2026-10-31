use crate::theme;
use gpui::{prelude::*, *};

pub const CHECKBOX_PADDING: f32 = 6.;

pub fn checkbox_click(event: &ClickEvent) -> bool {
    matches!(event, ClickEvent::Mouse(_))
}

pub fn checkbox_key_repeat(key: &Keystroke, is_held: bool, checkbox_focused: bool) -> bool {
    checkbox_focused && is_held && key.key == "space"
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn one_space_press_toggles_once_despite_repeat_and_keyboard_click_on_release() {
        let key = Keystroke::parse("space").unwrap();
        let mut checked = false;
        for is_held in [false, true, true] {
            if !checkbox_key_repeat(&key, is_held, true) {
                checked = !checked;
            }
        }
        let release = ClickEvent::Keyboard(KeyboardClickEvent {
            button: KeyboardButton::Space,
            ..Default::default()
        });
        if checkbox_click(&release) {
            checked = !checked;
        }
        assert!(checked);
        if !checkbox_key_repeat(&key, false, true) {
            checked = !checked;
        }
        assert!(!checked);
        if checkbox_click(&ClickEvent::Mouse(MouseClickEvent::default())) {
            checked = !checked;
        }
        assert!(checked);
        assert!(!checkbox_click(&ClickEvent::Keyboard(KeyboardClickEvent {
            button: KeyboardButton::Enter,
            ..Default::default()
        })));
        assert!(!checkbox_key_repeat(&key, true, false));
        assert!(!checkbox_key_repeat(
            &Keystroke::parse("down").unwrap(),
            true,
            true
        ));
    }
}
