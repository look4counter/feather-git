use crate::dialog::Dialog;
use crate::repository::{Operation, Request, suggested_name};
use crate::text_input::{Changed, TextInput};
use gpui::{prelude::*, *};

#[derive(Clone)]
pub enum DialogEvent {
    Browse,
    Confirm(Request),
    Cancel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogActivity {
    #[default]
    Idle,
    Browsing,
}
impl DialogActivity {
    fn is_busy(self) -> bool {
        self != Self::Idle
    }
}

pub struct RepositoryDialog {
    pub operation: Operation,
    pub source: Entity<TextInput>,
    pub path: Entity<TextInput>,
    pub name: Entity<TextInput>,
    activity: DialogActivity,
    focus: FocusHandle,
    _path_subscription: Subscription,
}
impl EventEmitter<DialogEvent> for RepositoryDialog {}
impl RepositoryDialog {
    pub fn new(operation: Operation, cx: &mut Context<Self>) -> Self {
        let source = cx.new(TextInput::new);
        let path = cx.new(TextInput::new);
        let name: Entity<TextInput> = cx.new(TextInput::new);
        let subscription = cx.subscribe(&path, |this: &mut Self, _, event: &Changed, cx| {
            this.name
                .update(cx, |name, cx| name.set_text(&suggested_name(this.operation, &event.0), cx));
            cx.notify();
        });
        Self {
            operation,
            source,
            path,
            name,
            activity: DialogActivity::Idle,
            focus: cx.focus_handle(),
            _path_subscription: subscription,
        }
    }
    pub fn focus_first(&self, window: &mut Window, cx: &App) {
        let input = if self.operation == Operation::Clone {
            &self.source
        } else {
            &self.path
        };
        input.read(cx).focus_handle.focus(window);
    }
    pub fn is_busy(&self) -> bool {
        self.activity.is_busy()
    }
    pub fn set_activity(
        &mut self,
        activity: DialogActivity,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activity = activity;
        if self.is_busy() {
            self.focus.focus(window);
        } else {
            self.focus_first(window, cx);
        }
        cx.notify();
    }
    fn request(&self, cx: &App) -> Request {
        Request {
            operation: self.operation,
            source: self.source.read(cx).text(),
            path: self.path.read(cx).text(),
            name: self.name.read(cx).text(),
        }
    }
    fn confirm(&mut self, cx: &mut Context<Self>) {
        if !self.is_busy() {
            cx.emit(DialogEvent::Confirm(self.request(cx)));
        }
    }
    fn input(&self, input: Entity<TextInput>) -> Div {
        crate::form_controls::input(input).flex_1()
    }
}
impl Render for RepositoryDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .when(self.operation == Operation::Clone, |el| {
                el.child(div().flex_shrink_0().child("소스 경로")).child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .child(self.input(self.source.clone())),
                )
            })
            .child(div().flex_shrink_0().child("Git 디렉터리 경로"))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .gap_2()
                    .child(self.input(self.path.clone()))
                    .child(
                        crate::dialog::button("repository-browse", "", false, !self.is_busy())
                            .w(px(34.))
                            .px_0()
                            .child(crate::form_controls::folder_icon())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.is_busy() {
                                    cx.emit(DialogEvent::Browse);
                                }
                            })),
                    ),
            )
            .child(div().flex_shrink_0().child("저장소 이름"))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .child(self.input(self.name.clone())),
            );
        let footer = div()
            .flex()
            .flex_shrink_0()
            .gap_2()
            .mt_3()
            .child(
                crate::dialog::button("repository-confirm", "확인", true, !self.is_busy())
                    .on_click(cx.listener(|this, _, _, cx| this.confirm(cx))),
            )
            .child(
                crate::dialog::button("repository-cancel", "취소", false, !self.is_busy())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.is_busy() {
                            cx.emit(DialogEvent::Cancel);
                        }
                    })),
            );
        Dialog::new("repository-overlay", self.operation.title())
            .width(px(520.))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.is_busy() {
                    cx.stop_propagation();
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" => {
                        cx.stop_propagation();
                        cx.emit(DialogEvent::Cancel);
                    }
                    "enter" => {
                        if ![&this.source, &this.path, &this.name]
                            .iter()
                            .any(|i| i.read(cx).is_composing())
                        {
                            cx.stop_propagation();
                            this.confirm(cx);
                        }
                    }
                    "tab" => {
                        cx.stop_propagation();
                        let mut fields = vec![&this.path, &this.name];
                        if this.operation == Operation::Clone {
                            fields.insert(0, &this.source);
                        }
                        let index = fields
                            .iter()
                            .position(|i| i.read(cx).focus_handle.is_focused(window))
                            .unwrap_or(0);
                        let next = if event.keystroke.modifiers.shift {
                            (index + fields.len() - 1) % fields.len()
                        } else {
                            (index + 1) % fields.len()
                        };
                        fields[next].read(cx).focus_handle.focus(window);
                    }
                    _ => {}
                }
            }))
            .body(body)
            .footer(footer)
    }
}

#[cfg(test)]
mod tests {
    use super::DialogActivity;

    #[test]
    fn selected_paths_and_names_follow_operation_without_changing_manual_paths() {
        use crate::repository::{Operation, selected_repository_path, suggested_name};
        use std::path::Path;
        for operation in [Operation::Create, Operation::Clone] {
            assert_eq!(selected_repository_path(operation, Path::new("project")), Path::new("project/.git"));
            for path in ["D:/work/my-app/.git", "D:/work/my-app/.git2", "D:/work/my-app/custom"] {
                assert_eq!(suggested_name(operation, path), "my-app");
            }
        }
        assert_eq!(selected_repository_path(Operation::Open, Path::new("project/.git2")), Path::new("project/.git2"));
        assert_eq!(suggested_name(Operation::Open, "D:/work/my-app/.git2"), "my-app");
        assert_eq!(suggested_name(Operation::Open, "D:/work/my-app"), "my-app");
        assert_eq!(suggested_name(Operation::Open, "D:/work/bare.git"), "bare.git");
        assert_eq!(suggested_name(Operation::Clone, "D:/"), "");
    }

    #[test]
    fn browsing_blocks_duplicate_actions_until_the_folder_prompt_returns() {
        assert!(!DialogActivity::Idle.is_busy());
        assert!(DialogActivity::Browsing.is_busy());
    }
}
