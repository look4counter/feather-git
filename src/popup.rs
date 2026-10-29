use crate::theme;
use gpui::{prelude::*, *};

fn next_input(current: Option<usize>, count: usize, backwards: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current {
        Some(index) if backwards => (index + count - 1) % count,
        Some(index) => (index + 1) % count,
        None if backwards => count - 1,
        None => 0,
    })
}

pub fn focus_next(inputs: &[FocusHandle], backwards: bool, window: &mut Window) {
    let current = inputs.iter().position(|input| input.is_focused(window));
    if let Some(index) = next_input(current, inputs.len(), backwards) {
        inputs[index].focus(window);
    }
}

pub fn frame(
    title: &str,
    width: f32,
    body: Div,
    footer: Option<AnyElement>,
    header_action: Option<AnyElement>,
    viewport_height: Pixels,
    fixed_height: Option<f32>,
) -> Stateful<Div> {
    div()
        .id("modal-card")
        .w(px(width))
        .max_w_full()
        .max_h((viewport_height - px(64.)).max(px(0.)))
        .when_some(fixed_height, |card, height| {
            card.h(px(height).min((viewport_height - px(64.)).max(px(0.))))
        })
        .p_5()
        .flex()
        .flex_col()
        .gap_3()
        .overflow_hidden()
        .bg(rgb(theme::PANEL))
        .border_1()
        .border_color(rgb(theme::LINE))
        .rounded_lg()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .flex_shrink_0()
                .pb_3()
                .border_b_1()
                .border_color(rgb(theme::LINE))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title.to_owned()),
                )
                .when_some(header_action, |header, action| header.child(action)),
        )
        .child(
            div()
                .id("popup-body")
                .min_w_0()
                .min_h_0()
                .when(fixed_height.is_some(), |body| body.flex_1())
                .overflow_y_scroll()
                .child(body),
        )
        .when_some(footer, |card, footer| {
            card.child(
                div()
                    .flex()
                    .justify_end()
                    .flex_shrink_0()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(theme::LINE))
                    .child(footer),
            )
        })
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, _, cx| cx.stop_propagation())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn input_focus_wraps_in_both_directions() {
        assert_eq!(next_input(Some(2), 3, false), Some(0));
        assert_eq!(next_input(Some(0), 3, true), Some(2));
        assert_eq!(next_input(None, 3, true), Some(2));
        assert_eq!(next_input(None, 1, false), Some(0));
        assert_eq!(next_input(None, 0, false), None);
    }
}
