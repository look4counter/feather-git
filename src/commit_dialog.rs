use crate::{
    dialog::{Dialog, button},
    form_controls,
    repository::{self, CommitDraft, CommitRequest},
    text_input::TextInput,
    theme::*,
};
use gpui::{prelude::*, *};
use std::path::PathBuf;

pub enum CommitDialogEvent {
    Confirm(CommitRequest),
    Cancel,
}
pub struct CommitDialog {
    editor: Entity<TextInput>,
    draft: Option<CommitDraft>,
    push: bool,
    amend: bool,
    normal_message: String,
    amend_message: String,
    loading: bool,
    busy: bool,
    error: Option<String>,
    focus: FocusHandle,
    push_focus: FocusHandle,
    amend_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
}
impl EventEmitter<CommitDialogEvent> for CommitDialog {}
impl CommitDialog {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(TextInput::text_area);
        editor.update(cx, |input, _| input.enabled = false);
        cx.observe(&editor, |_, _, cx| cx.notify()).detach();
        let task = crate::backend::spawn(cx.background_executor(), move || {
            repository::prepare_commit(&path)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(draft) => {
                        this.error = None;
                        this.normal_message = draft.default_message.clone();
                        this.amend_message = draft.last_message.clone();
                        this.editor.update(cx, |input, cx| {
                            input.enabled = true;
                            input.set_text(&draft.default_message, cx);
                        });
                        this.draft = Some(draft);
                    }
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        Self {
            editor,
            draft: None,
            push: false,
            amend: false,
            normal_message: String::new(),
            amend_message: String::new(),
            loading: true,
            busy: false,
            error: None,
            focus: cx.focus_handle(),
            push_focus: cx.focus_handle(),
            amend_focus: cx.focus_handle(),
            confirm_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
        }
    }
    pub fn focus(&self, window: &mut Window, cx: &App) {
        self.editor.read(cx).focus_handle.focus(window);
    }
    pub fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        self.editor.update(cx, |input, cx| {
            input.enabled = !busy && !self.loading;
            cx.notify();
        });
        cx.notify();
    }
    pub fn fail(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        self.set_busy(false, cx);
    }
    pub fn can_close(&self) -> bool {
        !self.busy
    }
    fn available(&self) -> bool {
        !self.loading && !self.busy && self.draft.is_some()
    }
    fn toggle_amend(&mut self, cx: &mut Context<Self>) {
        if !self.available() || !self.draft.as_ref().is_some_and(|draft| draft.can_amend) {
            return;
        }
        let message = switch_message(
            &mut self.amend,
            &mut self.normal_message,
            &mut self.amend_message,
            self.editor.read(cx).text(),
        );
        self.editor
            .update(cx, |input, cx| input.set_text(&message, cx));
        self.error = None;
        cx.notify();
    }
    fn confirm(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.editor.read(cx).is_composing() {
            return;
        }
        if self.loading {
            self.error = Some("커밋 상태를 읽고 있습니다. 잠시 후 다시 확인하세요.".into());
            cx.notify();
            return;
        }
        let Some(draft) = self.draft.as_ref() else {
            if self.error.is_none() {
                self.error = Some("커밋 상태를 읽지 못했습니다. 창을 다시 열어 확인하세요.".into());
            }
            cx.notify();
            return;
        };
        cx.emit(CommitDialogEvent::Confirm(CommitRequest {
            expected_head: draft.head.clone(),
            expected_reference: draft.reference.clone(),
            message: self.editor.read(cx).text(),
            push: self.push,
            amend: self.amend,
        }));
    }
}
impl Render for CommitDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let available = self.available();
        let can_amend = available && self.draft.as_ref().is_some_and(|draft| draft.can_amend);
        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child("커밋 메시지")
            .child(form_controls::text_area(self.editor.clone()))
            .child(
                form_controls::checkbox("commit-push", self.push, "바뀐 내용 즉시 푸시", available)
                    .track_focus(&self.push_focus)
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.available() {
                            this.push_focus.focus(window);
                            this.push = !this.push;
                            cx.notify();
                        }
                    })),
            )
            .child(
                form_controls::checkbox("commit-amend", self.amend, "마지막 커밋 정정", can_amend)
                    .track_focus(&self.amend_focus)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.amend_focus.focus(window);
                        this.toggle_amend(cx);
                    })),
            )
            .when(self.loading, |body| {
                body.child(
                    div()
                        .text_color(rgb(MUTED))
                        .child("커밋 상태를 읽고 있습니다…"),
                )
            })
            .when(
                !self.loading
                    && self
                        .draft
                        .as_ref()
                        .is_some_and(|draft| !draft.has_staged && !draft.is_merge)
                    && !self.amend,
                |body| {
                    body.child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child("스테이지된 변경사항이 없습니다."),
                    )
                },
            )
            .when_some(self.error.clone(), |body, error| {
                body.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(0xffa7b2))
                        .child(error),
                )
            });
        let footer = div()
            .flex()
            .gap_2()
            .child(
                button("commit-confirm", "확인", true, true)
                    .track_focus(&self.confirm_focus)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm(cx))),
            )
            .child(
                button("commit-cancel", "취소", false, !self.busy)
                    .track_focus(&self.cancel_focus)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.can_close() {
                            cx.emit(CommitDialogEvent::Cancel);
                        }
                    })),
            );
        Dialog::new("commit-overlay", "Commit")
            .width(px(600.))
            .track_focus(&self.focus)
            .body(body)
            .footer(footer)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.editor.read(cx).is_composing() {
                    return;
                }
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    if this.can_close() {
                        cx.emit(CommitDialogEvent::Cancel);
                    }
                } else if event.keystroke.key == "tab" && !this.busy {
                    let mut handles = Vec::new();
                    if this.available() {
                        handles.push(this.editor.read(cx).focus_handle.clone());
                        handles.push(this.push_focus.clone());
                    }
                    if this.available() && this.draft.as_ref().is_some_and(|draft| draft.can_amend)
                    {
                        handles.push(this.amend_focus.clone());
                    }
                    handles.push(this.confirm_focus.clone());
                    handles.push(this.cancel_focus.clone());
                    let current = handles.iter().position(|handle| handle.is_focused(window));
                    let next = match current {
                        Some(index) if event.keystroke.modifiers.shift => {
                            (index + handles.len() - 1) % handles.len()
                        }
                        Some(index) => (index + 1) % handles.len(),
                        None => 0,
                    };
                    handles[next].focus(window);
                    cx.stop_propagation();
                }
            }))
    }
}

fn switch_message(
    amend: &mut bool,
    normal: &mut String,
    amended: &mut String,
    current: String,
) -> String {
    if *amend {
        *amended = current;
    } else {
        *normal = current;
    }
    *amend = !*amend;
    if *amend {
        amended.clone()
    } else {
        normal.clone()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn amend_toggle_preserves_both_message_drafts() {
        let mut amend = false;
        let mut normal = String::new();
        let mut amended = "last commit".to_owned();
        assert_eq!(
            super::switch_message(&mut amend, &mut normal, &mut amended, "normal draft".into()),
            "last commit"
        );
        assert!(amend);
        assert_eq!(
            super::switch_message(&mut amend, &mut normal, &mut amended, "amend draft".into()),
            "normal draft"
        );
        assert!(!amend);
        assert_eq!(
            super::switch_message(
                &mut amend,
                &mut normal,
                &mut amended,
                "normal edited".into()
            ),
            "amend draft"
        );
        assert!(amend);
    }
}
