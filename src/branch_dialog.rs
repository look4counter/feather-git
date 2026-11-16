use crate::{
    dialog::{Dialog, button},
    form_controls,
    text_input::TextInput,
};
use gpui::{prelude::*, *};

pub enum BranchDialogEvent {
    Confirm {
        name: String,
        checkout: bool,
    },
    ConfirmTag {
        id: String,
        name: String,
        push: bool,
    },
    ConfirmStash {
        name: String,
        keep_index: bool,
    },
    Cancel,
}
pub struct BranchDialog {
    editor: Entity<TextInput>,
    checkout: bool,
    remote: Option<String>,
    tag_commit: Option<String>,
    push: bool,
    stash: bool,
    keep_index: bool,
    busy: bool,
    error: Option<String>,
    focus: FocusHandle,
    checkout_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
}
impl EventEmitter<BranchDialogEvent> for BranchDialog {}
impl BranchDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let editor = cx.new(TextInput::new);
        cx.observe(&editor, |_, _, cx| cx.notify()).detach();
        Self {
            editor,
            checkout: false,
            remote: None,
            tag_commit: None,
            push: false,
            stash: false,
            keep_index: false,
            busy: false,
            error: None,
            focus: cx.focus_handle(),
            checkout_focus: cx.focus_handle(),
            confirm_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
        }
    }
    pub fn configure_remote(&mut self, reference: String, name: String, cx: &mut Context<Self>) {
        self.remote = Some(reference);
        self.checkout = true;
        self.editor
            .update(cx, |editor, cx| editor.set_text(&name, cx));
        cx.notify();
    }
    pub fn configure_tag(&mut self, id: String, cx: &mut Context<Self>) {
        self.tag_commit = Some(id);
        self.remote = None;
        self.push = false;
        cx.notify();
    }
    pub fn configure_stash(&mut self, cx: &mut Context<Self>) {
        self.stash = true;
        self.tag_commit = None;
        self.remote = None;
        self.keep_index = false;
        cx.notify();
    }
    fn option_checked(&self) -> bool {
        if self.stash {
            self.keep_index
        } else if self.tag_commit.is_some() {
            self.push
        } else {
            self.checkout
        }
    }
    fn toggle_option(&mut self) {
        if self.stash {
            self.keep_index = !self.keep_index;
        } else if self.tag_commit.is_some() {
            self.push = !self.push;
        } else if self.remote.is_none() {
            self.checkout = !self.checkout;
        }
    }
    pub fn focus(&self, window: &mut Window, cx: &App) {
        self.editor.read(cx).focus_handle.focus(window);
    }
    pub fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        self.editor.update(cx, |input, cx| {
            input.enabled = !busy;
            cx.notify();
        });
        cx.notify();
    }
    pub fn fail(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        self.set_busy(false, cx);
    }
    fn can_confirm(&self, cx: &App) -> bool {
        !self.busy
            && !self.editor.read(cx).is_composing()
            && !self.editor.read(cx).text().trim().is_empty()
    }
    fn confirm(&self, cx: &mut Context<Self>) {
        if self.can_confirm(cx) {
            let name = self.editor.read(cx).text().trim().to_owned();
            if self.stash {
                cx.emit(BranchDialogEvent::ConfirmStash {
                    name,
                    keep_index: self.keep_index,
                });
            } else if let Some(id) = &self.tag_commit {
                cx.emit(BranchDialogEvent::ConfirmTag {
                    id: id.clone(),
                    name,
                    push: self.push,
                });
            } else {
                cx.emit(BranchDialogEvent::Confirm {
                    name,
                    checkout: self.checkout,
                });
            }
        }
    }
}
impl Render for BranchDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .when_some(self.remote.clone(), |body, remote| body.child(remote))
            .when_some(self.tag_commit.clone(), |body, id| body.child(id))
            .child(form_controls::labeled_input(
                if self.stash {
                    "스태시명"
                } else if self.tag_commit.is_some() {
                    "태그명"
                } else {
                    "브랜치명"
                },
                self.editor.clone(),
            ))
            .child(
                form_controls::checkbox(
                    "branch-checkout",
                    self.option_checked(),
                    if self.stash {
                        "스테이지 변경사항 유지"
                    } else if self.tag_commit.is_some() {
                        "원격에 Push"
                    } else {
                        "생성 후 체크아웃"
                    },
                    !self.busy && self.remote.is_none(),
                )
                .track_focus(&self.checkout_focus)
                .on_click(cx.listener(|this, event, window, cx| {
                    if !this.busy && this.remote.is_none() && form_controls::checkbox_click(event) {
                        this.checkout_focus.focus(window);
                        this.toggle_option();
                        cx.notify();
                    }
                })),
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
                button("branch-confirm", "확인", true, self.can_confirm(cx))
                    .track_focus(&self.confirm_focus)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm(cx))),
            )
            .child(
                button("branch-cancel", "취소", false, !self.busy)
                    .track_focus(&self.cancel_focus)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.busy {
                            cx.emit(BranchDialogEvent::Cancel);
                        }
                    })),
            );
        Dialog::new(
            "branch-overlay",
            if self.stash {
                "Stash"
            } else if self.tag_commit.is_some() {
                "태그 추가"
            } else {
                "New Branch"
            },
        )
        .width(px(440.))
        .track_focus(&self.focus)
        .body(body)
        .footer(footer)
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            if this.busy || this.editor.read(cx).is_composing() {
                return;
            }
            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    cx.emit(BranchDialogEvent::Cancel);
                }
                "enter" => {
                    cx.stop_propagation();
                    if this.cancel_focus.is_focused(window) {
                        cx.emit(BranchDialogEvent::Cancel);
                    } else if this.checkout_focus.is_focused(window) {
                        this.toggle_option();
                        cx.notify();
                    } else {
                        this.confirm(cx);
                    }
                }
                "space" if this.checkout_focus.is_focused(window) => {
                    cx.stop_propagation();
                    this.toggle_option();
                    cx.notify();
                }
                "tab" => {
                    let mut handles = vec![
                        this.editor.read(cx).focus_handle.clone(),
                        this.checkout_focus.clone(),
                    ];
                    if this.can_confirm(cx) {
                        handles.push(this.confirm_focus.clone());
                    }
                    handles.push(this.cancel_focus.clone());
                    let current = handles.iter().position(|h| h.is_focused(window));
                    let next = match current {
                        Some(i) if event.keystroke.modifiers.shift => {
                            (i + handles.len() - 1) % handles.len()
                        }
                        Some(i) => (i + 1) % handles.len(),
                        None => 0,
                    };
                    handles[next].focus(window);
                    cx.stop_propagation();
                }
                _ => {}
            }
        }))
    }
}
