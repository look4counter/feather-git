use crate::{dialog::Dialog, text_encoding::TextEncoding, theme::*};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingDialogEvent {
    Select(TextEncoding),
    Cancel,
}

struct EncodingChoice {
    #[cfg(test)]
    current: TextEncoding,
    cursor: usize,
}
impl EncodingChoice {
    fn new(current: TextEncoding) -> Self {
        Self {
            #[cfg(test)]
            current,
            cursor: TextEncoding::ALL
                .iter()
                .position(|value| *value == current)
                .unwrap_or(0),
        }
    }
    fn select(&mut self, index: usize) {
        self.cursor = index;
    }
    fn confirm(&self) -> EncodingDialogEvent {
        EncodingDialogEvent::Select(TextEncoding::ALL[self.cursor])
    }
    fn key(&mut self, key: &str) -> Option<EncodingDialogEvent> {
        match key {
            "up" => {
                self.cursor = (self.cursor + TextEncoding::ALL.len() - 1) % TextEncoding::ALL.len()
            }
            "down" => self.cursor = (self.cursor + 1) % TextEncoding::ALL.len(),
            "escape" => return Some(EncodingDialogEvent::Cancel),
            _ => {}
        }
        None
    }
}

pub struct EncodingDialog {
    choice: EncodingChoice,
    focus: FocusHandle,
}
impl EventEmitter<EncodingDialogEvent> for EncodingDialog {}
impl EncodingDialog {
    pub fn new(current: TextEncoding, cx: &mut Context<Self>) -> Self {
        Self {
            choice: EncodingChoice::new(current),
            focus: cx.focus_handle(),
        }
    }
    pub fn focus(&self, window: &mut Window) {
        self.focus.focus(window);
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        if let Some(event) = self.choice.key(&event.keystroke.key) {
            cx.emit(event);
        }
        cx.notify();
    }
}
impl Render for EncodingDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Dialog::new("encoding-overlay", "인코딩 선택")
            .width(px(400.))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .body(
                div().flex().flex_col().gap_1().children(
                    TextEncoding::ALL
                        .into_iter()
                        .enumerate()
                        .map(|(index, encoding)| {
                            div()
                                .id(("encoding-option", index))
                                .h(px(28.))
                                .px_3()
                                .flex()
                                .items_center()
                                .gap_2()
                                .rounded_sm()
                                .cursor_pointer()
                                .when(index == self.choice.cursor, |el| el.bg(rgb(SELECTED)))
                                .hover(|style| style.bg(rgb(RAISED)))
                                .child(div().w(px(16.)).child(if index == self.choice.cursor {
                                    "✓"
                                } else {
                                    ""
                                }))
                                .child(encoding.name())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.choice.select(index);
                                    cx.notify();
                                }))
                        }),
                ),
            )
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        crate::dialog::button("encoding-confirm", "확인", true, true).on_click(
                            cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(this.choice.confirm());
                            }),
                        ),
                    )
                    .child(
                        crate::dialog::button("encoding-cancel", "취소", false, true).on_click(
                            cx.listener(|_, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(EncodingDialogEvent::Cancel);
                            }),
                        ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn starts_at_current_encoding_and_keyboard_wraps_without_changing_value() {
        let mut choice = EncodingChoice::new(TextEncoding::Windows949);
        assert_eq!(choice.cursor, 3);
        assert_eq!(choice.key("escape"), Some(EncodingDialogEvent::Cancel));
        assert_eq!(choice.current, TextEncoding::Windows949);
        assert_eq!(choice.key("down"), None);
        assert_eq!(
            choice.confirm(),
            EncodingDialogEvent::Select(TextEncoding::EucKr)
        );
        assert_eq!(choice.current, TextEncoding::Windows949);
        assert_eq!(choice.key("enter"), None);
        assert_eq!(choice.key("space"), None);
        choice.select(0);
        assert_eq!(choice.key("escape"), Some(EncodingDialogEvent::Cancel));
        assert_eq!(choice.current, TextEncoding::Windows949);
        assert_eq!(
            choice.confirm(),
            EncodingDialogEvent::Select(TextEncoding::Utf8)
        );
        let mut choice = EncodingChoice::new(TextEncoding::Utf8);
        choice.key("up");
        assert_eq!(choice.cursor, 10);
        choice.key("down");
        assert_eq!(choice.cursor, 0);
        assert_eq!(choice.key("tab"), None);
    }
}
