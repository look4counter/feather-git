use crate::{dialog::Dialog, repository::ResetMode, theme::*};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionOptions {
    Confirm,
    Revert(bool),
    Reset(ResetMode),
}
impl ActionOptions {
    fn key(&mut self, key: &str) {
        match self {
            Self::Reset(mode) if matches!(key, "up" | "down") => {
                let index = ResetMode::ALL
                    .iter()
                    .position(|value| value == mode)
                    .unwrap_or(1);
                *mode = ResetMode::ALL[(index + if key == "up" { 2 } else { 1 }) % 3];
            }
            Self::Revert(value) if key == "space" => *value = !*value,
            _ => {}
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitActionEvent {
    Confirm(ActionOptions),
    Cancel,
}
pub struct GitActionDialog {
    title: String,
    description: String,
    options: ActionOptions,
    focus: FocusHandle,
    controls: [FocusHandle; 3],
}
impl EventEmitter<GitActionEvent> for GitActionDialog {}
impl GitActionDialog {
    pub fn new(
        title: String,
        description: String,
        options: ActionOptions,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title,
            description,
            options,
            focus: cx.focus_handle(),
            controls: std::array::from_fn(|_| cx.focus_handle()),
        }
    }
    pub fn focus(&self, window: &mut Window) {
        self.controls[2].focus(window);
    }
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        match event.keystroke.key.as_str() {
            "escape" => cx.emit(GitActionEvent::Cancel),
            "enter" => {
                if self.controls[1].is_focused(window) {
                    cx.emit(GitActionEvent::Confirm(self.options));
                } else if self.controls[2].is_focused(window) {
                    cx.emit(GitActionEvent::Cancel);
                }
            }
            "tab" => {
                let current = self
                    .controls
                    .iter()
                    .position(|focus| focus.is_focused(window))
                    .unwrap_or(2);
                let next = (current
                    + if event.keystroke.modifiers.shift {
                        2
                    } else {
                        1
                    })
                    % 3;
                self.controls[next].focus(window);
            }
            key if self.controls[0].is_focused(window) => self.options.key(key),
            _ => {}
        }
        cx.notify();
    }
}
impl Render for GitActionDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let options_focus = self.controls[0].clone();
        let mut options = div().flex().flex_col().gap_1().track_focus(&options_focus);
        match self.options {
            ActionOptions::Confirm => {}
            ActionOptions::Revert(checked) => {
                options = options.child(
                    crate::form_controls::checkbox(
                        "revert-no-commit",
                        checked,
                        "no-commit (커밋 없이 적용)",
                        true,
                    )
                    .on_click(cx.listener(|this, event, window, cx| {
                        if crate::form_controls::checkbox_click(event) {
                            this.controls[0].focus(window);
                            if let ActionOptions::Revert(value) = &mut this.options {
                                *value = !*value;
                            }
                            cx.notify();
                        }
                    })),
                );
            }
            ActionOptions::Reset(mode) => {
                options = options
                    .children(
                        ResetMode::ALL
                            .into_iter()
                            .enumerate()
                            .map(|(index, value)| {
                                div()
                                    .id(("reset-option", index))
                                    .h(px(28.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .when(mode == value, |row| row.bg(rgb(SELECTED)))
                                    .hover(|style| style.bg(rgb(RAISED)))
                                    .child(div().w(px(16.)).child(if mode == value {
                                        "✓"
                                    } else {
                                        ""
                                    }))
                                    .child(value.name())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.controls[0].focus(window);
                                        this.options = ActionOptions::Reset(value);
                                        cx.notify();
                                    }))
                            }),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child(mode.description()),
                    );
            }
        }
        Dialog::new("git-action-dialog", self.title.clone())
            .width(px(480.))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .body(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.description.clone())
                    .child(options),
            )
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        crate::dialog::button("git-action-confirm", "확인", true, true)
                            .track_focus(&self.controls[1])
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(GitActionEvent::Confirm(this.options));
                            })),
                    )
                    .child(
                        crate::dialog::button("git-action-cancel", "취소", false, true)
                            .track_focus(&self.controls[2])
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(GitActionEvent::Cancel);
                            })),
                    ),
            )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn options_are_explicit_and_reset_keyboard_matches_encoding_choice() {
        let mut reset = ActionOptions::Reset(ResetMode::default());
        assert_eq!(reset, ActionOptions::Reset(ResetMode::Mixed));
        reset.key("down");
        assert_eq!(reset, ActionOptions::Reset(ResetMode::Hard));
        reset.key("down");
        assert_eq!(reset, ActionOptions::Reset(ResetMode::Soft));
        reset.key("up");
        assert_eq!(reset, ActionOptions::Reset(ResetMode::Hard));
        let mut revert = ActionOptions::Revert(false);
        revert.key("space");
        assert_eq!(revert, ActionOptions::Revert(true));
        revert.key("escape");
        assert_eq!(revert, ActionOptions::Revert(true));
    }
}
