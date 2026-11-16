use crate::theme::*;
use gpui::{prelude::*, *};

type KeyHandler = Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    title: SharedString,
    width: Pixels,
    body: Option<AnyElement>,
    footer: Option<AnyElement>,
    overlays: Vec<AnyElement>,
    focus: Option<FocusHandle>,
    key_handler: Option<KeyHandler>,
}

impl Dialog {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            width: px(400.),
            body: None,
            footer: None,
            overlays: Vec::new(),
            focus: None,
            key_handler: None,
        }
    }
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }
    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
    pub fn overlay(mut self, overlay: impl IntoElement) -> Self {
        self.overlays.push(overlay.into_any_element());
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
    pub fn on_key_down(
        mut self,
        handler: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.key_handler = Some(Box::new(handler));
        self
    }
    pub fn build(self) -> Stateful<Div> {
        let card = div()
            .id("dialog-card")
            .relative()
            .w(self.width)
            .max_w_full()
            .max_h_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .overflow_y_scroll()
            .bg(rgb(PANEL))
            .text_color(rgb(TEXT))
            .border_1()
            .border_color(rgb(LINE))
            .rounded_lg()
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title),
            )
            .when_some(self.body, |el, body| el.child(body))
            .when_some(self.footer, |el, footer| {
                el.child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .justify_end()
                        .gap_2()
                        .child(footer),
                )
            })
            .children(self.overlays);
        div()
            .id(self.id)
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_5()
            .bg(rgba(0x00000099))
            .occlude()
            .when_some(self.focus, |el, focus| el.track_focus(&focus))
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .when_some(self.key_handler, |el, handler| el.on_key_down(handler))
            .child(card)
    }
}
impl RenderOnce for Dialog {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.build()
    }
}

/// Shared action styling for all application dialogs.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    enabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(32.))
        .px_4()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.))
        .text_color(rgb(TEXT))
        .when(primary, |button| button.bg(rgb(SELECTED)))
        .border_1()
        .border_color(rgb(if primary { ACCENT } else { LINE }))
        .rounded_md()
        .when(enabled, |button| {
            button.cursor_pointer().hover(|style| style.bg(rgb(RAISED)))
        })
        .when(!enabled, |button| button.opacity(0.45))
        .child(label.into())
}
