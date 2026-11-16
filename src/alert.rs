use crate::dialog::Dialog;
use crate::theme::*;
use gpui::{prelude::*, *};
use std::rc::Rc;

const CONFLICT_PATHS_HEIGHT: f32 = 200.;

#[derive(Clone)]
pub struct Alert {
    pub title: SharedString,
    pub message: SharedString,
    pub confirm_label: SharedString,
    paths: Vec<String>,
}

fn is_dismiss_key(key: &str) -> bool {
    matches!(key, "enter" | "escape")
}

impl Alert {
    pub fn new(title: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            paths: Vec::new(),
            confirm_label: "확인".into(),
        }
    }

    pub fn set_paths(&mut self, mut paths: Vec<String>) {
        paths.sort();
        paths.dedup();
        self.paths = paths;
    }

    pub fn render(
        &self,
        focus: &FocusHandle,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Stateful<Div> {
        let on_dismiss = Rc::new(on_dismiss);
        let keyboard_dismiss = on_dismiss.clone();
        let confirm =
            crate::dialog::button("alert-confirm", self.confirm_label.clone(), true, true)
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    on_dismiss(window, cx);
                });
        Dialog::new("alert-overlay", self.title.clone())
            .track_focus(focus)
            .on_key_down(move |event, window, cx| {
                cx.stop_propagation();
                if is_dismiss_key(&event.keystroke.key) {
                    keyboard_dismiss(window, cx);
                }
            })
            .body(
                div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .min_w_0()
                    .text_size(px(13.))
                    .text_color(rgb(MUTED))
                    .child(self.message.clone())
                    .when(!self.paths.is_empty(), |body| {
                        body.child(format!("충돌 경로: {}", self.paths.len()))
                            .child(
                                div()
                                    .id("alert-conflict-paths")
                                    .h(px(CONFLICT_PATHS_HEIGHT))
                                    .flex_shrink_0()
                                    .w_full()
                                    .min_w_0()
                                    .overflow_y_scroll()
                                    .overflow_x_scroll()
                                    .border_1()
                                    .border_color(rgb(LINE))
                                    .bg(rgb(BASE))
                                    .p_2()
                                    .children(self.paths.iter().map(|path| {
                                        div()
                                            .min_w_0()
                                            .py_1()
                                            .text_size(px(12.))
                                            .whitespace_nowrap()
                                            .child(path.clone())
                                    })),
                            )
                    }),
            )
            .footer(confirm)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn alert_preserves_custom_content() {
        let message = "긴 알림 메시지\n".repeat(200);
        let alert = Alert::new("사용자 지정 제목", message.clone());
        assert_eq!(alert.title.as_ref(), "사용자 지정 제목");
        assert_eq!(alert.message.as_ref(), message);
        assert_eq!(alert.confirm_label.as_ref(), "확인");
    }

    #[::core::prelude::v1::test]
    fn conflict_paths_are_deduplicated_and_large_lists_are_preserved() {
        let mut alert = Alert::new("conflict", "details");
        assert!(alert.paths.is_empty());
        let mut paths = (0..2000)
            .rev()
            .map(|index| format!("folder/{index:04}.txt"))
            .collect::<Vec<_>>();
        paths.push("folder/0000.txt".into());
        alert.set_paths(paths);
        assert_eq!(alert.paths.len(), 2000);
        assert_eq!(alert.paths.first().unwrap(), "folder/0000.txt");
        assert_eq!(alert.paths.last().unwrap(), "folder/1999.txt");
        assert_eq!(CONFLICT_PATHS_HEIGHT, 200.);
        alert.set_paths(Vec::new());
        assert!(alert.paths.is_empty());
    }

    #[::core::prelude::v1::test]
    fn only_enter_and_escape_dismiss() {
        assert!(is_dismiss_key("enter"));
        assert!(is_dismiss_key("escape"));
        for key in ["tab", "space", "a", "backspace"] {
            assert!(!is_dismiss_key(key));
        }
    }
}
