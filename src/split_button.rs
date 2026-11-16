use crate::{
    context_menu::{self, MenuItem},
    theme::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

type Execute = Rc<dyn Fn(&mut Window, &mut App)>;
type Select = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Main action and option selection are separate interactions.
#[derive(IntoElement)]
pub struct SplitButton {
    id: ElementId,
    label: SharedString,
    icon: AnyElement,
    options: Vec<SharedString>,
    selected: usize,
    enabled: bool,
    busy: bool,
    execute: Option<Execute>,
    select: Option<Select>,
}
impl SplitButton {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        icon: impl IntoElement,
        options: Vec<SharedString>,
        selected: usize,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into_any_element(),
            options,
            selected,
            enabled: true,
            busy: false,
            execute: None,
            select: None,
        }
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
    pub fn on_execute(mut self, action: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.execute = Some(Rc::new(action));
        self
    }
    pub fn on_select(mut self, select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.select = Some(Rc::new(select));
        self
    }
}
impl RenderOnce for SplitButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let enabled = self.enabled && !self.busy;
        let can_execute = enabled && self.execute.is_some();
        let can_select = enabled
            && self.select.is_some()
            && !self.options.is_empty()
            && self.selected < self.options.len();
        let anchor = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let painted_anchor = anchor.clone();
        let execute = self.execute;
        let select = self.select;
        let options = self.options;
        let selected = self.selected;
        div()
            .id(self.id)
            .relative()
            .h(px(52.))
            .flex()
            .items_center()
            .rounded_md()
            .overflow_hidden()
            .text_color(rgb(TEXT))
            .when(!enabled, |button| button.opacity(0.45))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, _, _| painted_anchor.set(Some(bounds)),
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(
                toolbar_action("split-button-execute", self.label, self.icon, can_execute)
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        if can_execute && let Some(execute) = &execute {
                            execute(window, cx);
                        }
                    }),
            )
            .child(
                div()
                    .id("split-button-options")
                    .h_full()
                    .w(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_l_1()
                    .border_color(rgb(LINE))
                    .when(can_select, |button| {
                        button.cursor_pointer().hover(|style| style.bg(rgb(LINE)))
                    })
                    .child("▾")
                    .on_click(move |event: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        if !can_select {
                            return;
                        }
                        let items = options
                            .iter()
                            .enumerate()
                            .map(|(index, label)| {
                                let select = select.clone();
                                MenuItem::new(label.clone(), "", true, move |window, cx| {
                                    if let Some(select) = &select {
                                        select(index, window, cx);
                                    }
                                })
                                .checked(index == selected)
                            })
                            .collect();
                        context_menu::open(
                            anchor
                                .get()
                                .map_or(event.position(), |bounds| bounds.bottom_left()),
                            items,
                            window,
                            cx,
                        );
                    }),
            )
    }
}

pub fn pull_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.7));
            let p = |x, y| bounds.origin + point(px(x), px(y));
            path.move_to(p(10., 1.));
            path.line_to(p(10., 12.));
            path.move_to(p(5., 7.));
            path.line_to(p(10., 12.));
            path.line_to(p(15., 7.));
            path.move_to(p(2., 12.));
            path.line_to(p(2., 18.));
            path.line_to(p(18., 18.));
            path.line_to(p(18., 12.));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
        },
    )
    .size(px(20.))
    .flex_shrink_0()
}

pub fn toolbar_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: impl IntoElement,
    enabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(52.))
        .w(px(56.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .when(enabled, |button| {
            button.cursor_pointer().hover(|style| style.bg(rgb(LINE)))
        })
        .child(icon)
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .child(label.into()),
        )
}
pub fn commit_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.7));
            let p = |x, y| bounds.origin + point(px(x), px(y));
            path.move_to(p(1., 10.));
            path.line_to(p(6., 10.));
            path.move_to(p(14., 10.));
            path.line_to(p(19., 10.));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
            window.paint_quad(quad(
                Bounds::new(p(6., 6.), size(px(8.), px(8.))),
                px(4.),
                rgba(0),
                px(1.7),
                rgb(TEXT),
                Default::default(),
            ));
        },
    )
    .size(px(20.))
    .flex_shrink_0()
}

pub fn push_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let p = |x, y| bounds.origin + point(px(x), px(y));
            let mut path = PathBuilder::stroke(px(1.7));
            path.move_to(p(10., 16.));
            path.line_to(p(10., 3.));
            path.move_to(p(5., 8.));
            path.line_to(p(10., 3.));
            path.line_to(p(15., 8.));
            path.move_to(p(3., 14.));
            path.line_to(p(3., 19.));
            path.line_to(p(17., 19.));
            path.line_to(p(17., 14.));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
        },
    )
    .size(px(20.))
    .flex_shrink_0()
}
pub fn branch_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let p = |x, y| bounds.origin + point(px(x), px(y));
            let mut path = PathBuilder::stroke(px(1.7));
            path.move_to(p(5., 5.));
            path.line_to(p(5., 15.));
            path.move_to(p(5., 13.));
            path.line_to(p(15., 8.));
            path.line_to(p(15., 5.));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
            for (x, y) in [(3., 1.), (3., 15.), (13., 1.)] {
                window.paint_quad(quad(
                    Bounds::new(p(x, y), size(px(4.), px(4.))),
                    px(2.),
                    rgb(PANEL),
                    px(1.5),
                    rgb(TEXT),
                    Default::default(),
                ));
            }
        },
    )
    .size(px(20.))
    .flex_shrink_0()
}

pub fn stash_icon() -> impl IntoElement {
    canvas(
        |_, _, _| (),
        |bounds, _, window, _| {
            let p = |x, y| bounds.origin + point(px(x), px(y));
            let mut path = PathBuilder::stroke(px(1.7));
            path.move_to(p(10., 1.));
            path.line_to(p(10., 11.));
            path.move_to(p(6., 7.));
            path.line_to(p(10., 11.));
            path.line_to(p(14., 7.));
            path.move_to(p(2., 10.));
            path.line_to(p(2., 18.));
            path.line_to(p(18., 18.));
            path.line_to(p(18., 10.));
            path.move_to(p(2., 13.));
            path.line_to(p(7., 13.));
            path.line_to(p(8., 15.));
            path.line_to(p(12., 15.));
            path.line_to(p(13., 13.));
            path.line_to(p(18., 13.));
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(TEXT));
            }
        },
    )
    .size(px(20.))
    .flex_shrink_0()
}
