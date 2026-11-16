use crate::theme::*;
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

type MenuAction = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone)]
pub struct MenuItem {
    label: SharedString,
    shortcut: SharedString,
    enabled: bool,
    checked: bool,
    separator_after: bool,
    action: MenuAction,
    children: Vec<MenuItem>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}
impl MenuItem {
    pub fn new(
        label: impl Into<SharedString>,
        shortcut: impl Into<SharedString>,
        enabled: bool,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            shortcut: shortcut.into(),
            enabled,
            checked: false,
            separator_after: false,
            action: Rc::new(action),
            children: Vec::new(),
            bounds: Rc::new(Cell::new(None)),
        }
    }
    pub fn submenu(label: impl Into<SharedString>, children: Vec<MenuItem>) -> Self {
        let mut item = Self::new(
            label,
            "",
            children.iter().any(|child| child.enabled),
            |_, _| {},
        );
        item.children = children;
        item
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub fn separator_after(mut self) -> Self {
        self.separator_after = true;
        self
    }
}

struct ActiveMenu(Option<Entity<ContextMenu>>);
impl Global for ActiveMenu {}
pub fn active(cx: &App) -> Option<Entity<ContextMenu>> {
    cx.try_global::<ActiveMenu>()
        .and_then(|menu| menu.0.clone())
}
pub fn dismiss(window: &mut Window, cx: &mut App) {
    if let Some(menu) = active(cx) {
        menu.update(cx, |menu, cx| menu.close(window, cx));
    }
}
fn initial_selection(items: &[MenuItem]) -> Option<usize> {
    items
        .iter()
        .position(|item| item.enabled && item.checked)
        .or_else(|| items.iter().position(|item| item.enabled))
}
pub fn open(position: Point<Pixels>, items: Vec<MenuItem>, window: &mut Window, cx: &mut App) {
    dismiss(window, cx);
    let previous_focus = window.focused(cx);
    let menu = cx.new(|cx| ContextMenu {
        position,
        selected: initial_selection(&items),
        items,
        focus: cx.focus_handle(),
        previous_focus,
        activation: None,
        submenus: Vec::new(),
        focused_level: 0,
    });
    menu.read(cx).focus.focus(window);
    cx.set_global(ActiveMenu(Some(menu)));
    window.refresh();
}
struct MenuLevel {
    position: Point<Pixels>,
    items: Vec<MenuItem>,
    selected: Option<usize>,
}
fn child_level(item: &MenuItem, position: Point<Pixels>) -> Option<MenuLevel> {
    (item.enabled && !item.children.is_empty()).then(|| MenuLevel {
        position,
        items: item.children.clone(),
        selected: initial_selection(&item.children),
    })
}
fn submenu_position(bounds: Bounds<Pixels>, viewport_width: Pixels) -> Point<Pixels> {
    let right = bounds.right() + px(4.);
    point(
        if right + px(360.) <= viewport_width {
            right
        } else {
            (bounds.left() - px(364.)).max(px(4.))
        },
        bounds.top(),
    )
}
pub struct ContextMenu {
    position: Point<Pixels>,
    items: Vec<MenuItem>,
    selected: Option<usize>,
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    activation: Option<Subscription>,
    submenus: Vec<MenuLevel>,
    focused_level: usize,
}
fn next_enabled(enabled: &[bool], selected: Option<usize>, forward: bool) -> Option<usize> {
    if enabled.is_empty() {
        return None;
    }
    let count = enabled.len();
    let start = selected.unwrap_or(if forward { count - 1 } else { 0 });
    (1..=count)
        .map(|step| {
            if forward {
                (start + step) % count
            } else {
                (start + count - step) % count
            }
        })
        .find(|&index| enabled[index])
}
fn shortcut_matches(label: &str, key: &Keystroke) -> bool {
    let binding = label.to_lowercase().replace(' ', "").replace('+', "-");
    !binding.is_empty()
        && Keystroke::parse(&binding)
            .is_ok_and(|binding| binding.key == key.key && binding.modifiers == key.modifiers)
}
impl ContextMenu {
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.set_global(ActiveMenu(None));
        if let Some(focus) = self.previous_focus.take() {
            focus.focus(window);
        } else {
            window.blur();
        }
        window.refresh();
    }
    fn items_at(&self, depth: usize) -> &[MenuItem] {
        if depth == 0 {
            &self.items
        } else {
            self.submenus
                .get(depth - 1)
                .map(|level| level.items.as_slice())
                .unwrap_or(&[])
        }
    }
    fn selection_at(&self, depth: usize) -> Option<usize> {
        if depth == 0 {
            self.selected
        } else {
            self.submenus
                .get(depth - 1)
                .and_then(|level| level.selected)
        }
    }
    fn select(&mut self, depth: usize, index: Option<usize>) {
        if depth > self.submenus.len() {
            return;
        }
        if depth == 0 {
            self.selected = index;
        } else {
            self.submenus[depth - 1].selected = index;
        }
        self.submenus.truncate(depth);
        self.focused_level = depth;
    }
    fn open_child(
        &mut self,
        depth: usize,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.items_at(depth).get(index) else {
            return;
        };
        let position = item
            .bounds
            .get()
            .map(|bounds| submenu_position(bounds, window.viewport_size().width))
            .unwrap_or_else(|| point(self.position.x + px(360.), self.position.y));
        let Some(child) = child_level(item, position) else {
            return;
        };
        self.select(depth, Some(index));
        self.submenus.push(child);
        self.focused_level = depth + 1;
        cx.notify();
    }
    fn activate(
        &mut self,
        depth: usize,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.items_at(depth).get(index).filter(|item| item.enabled) else {
            return;
        };
        if !item.children.is_empty() {
            self.open_child(depth, index, window, cx);
            return;
        }
        let action = item.action.clone();
        self.close(window, cx);
        action(window, cx);
    }
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            return;
        }
        let depth = self.focused_level.min(self.submenus.len());
        if let Some(index) = self
            .items_at(depth)
            .iter()
            .position(|item| shortcut_matches(&item.shortcut, &event.keystroke))
        {
            self.activate(depth, index, window, cx);
            cx.stop_propagation();
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" if depth == 0 => self.close(window, cx),
            "left" | "escape" if depth > 0 => {
                self.submenus.truncate(depth - 1);
                self.focused_level = depth - 1;
                cx.notify();
            }
            "enter" | "space" => {
                if let Some(index) = self.selection_at(depth) {
                    self.activate(depth, index, window, cx);
                }
            }
            "right" => {
                if let Some(index) = self.selection_at(depth) {
                    self.open_child(depth, index, window, cx);
                }
            }
            "up" | "down" => {
                let selected = next_enabled(
                    &self
                        .items_at(depth)
                        .iter()
                        .map(|item| item.enabled)
                        .collect::<Vec<_>>(),
                    self.selection_at(depth),
                    event.keystroke.key == "down",
                );
                self.select(depth, selected);
                cx.notify();
            }
            _ => {}
        }
        cx.stop_propagation();
    }
    fn card(
        &self,
        depth: usize,
        items: &[MenuItem],
        selected: Option<usize>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let has_checks = items.iter().any(|item| item.checked);
        div()
            .id(("context-menu-card", depth))
            .w(px(360.))
            .p_1()
            .bg(rgb(PANEL))
            .text_color(rgb(TEXT))
            .text_size(px(12.))
            .border_1()
            .border_color(rgb(LINE))
            .rounded_md()
            .shadow_lg()
            .occlude()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .children(items.iter().enumerate().map(|(index, item)| {
                let painted = item.bounds.clone();
                let enabled = item.enabled;
                let row = div()
                    .id(index)
                    .relative()
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_3()
                    .h(px(30.))
                    .rounded_sm()
                    .when(selected == Some(index), |row| row.bg(rgb(RAISED)))
                    .when(enabled, |row| {
                        row.cursor_pointer().hover(|style| style.bg(rgb(SELECTED)))
                    })
                    .when(!enabled, |row| row.opacity(0.4))
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, _, _| painted.set(Some(bounds)),
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    )
                    .when(has_checks, |row| {
                        row.child(
                            div()
                                .w(px(16.))
                                .flex_shrink_0()
                                .text_color(rgb(ACCENT))
                                .child(if item.checked { "✓" } else { "" }),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(item.label.clone()),
                    )
                    .when(!item.shortcut.is_empty(), |row| {
                        row.child(
                            div()
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .px_1()
                                .border_1()
                                .border_color(rgb(LINE))
                                .rounded_sm()
                                .text_size(px(11.))
                                .font_family("Cascadia Mono")
                                .child(item.shortcut.clone()),
                        )
                    })
                    .when(!item.children.is_empty(), |row| row.child("›"))
                    .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                        if !*hovered || !enabled {
                            return;
                        }
                        let Some(item) = this.items_at(depth).get(index) else {
                            return;
                        };
                        let submenu = !item.children.is_empty();
                        if submenu {
                            this.open_child(depth, index, window, cx);
                        } else {
                            this.select(depth, Some(index));
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.activate(depth, index, window, cx);
                            cx.stop_propagation();
                        }),
                    );
                div()
                    .child(row)
                    .when(item.separator_after && index + 1 < items.len(), |group| {
                        group.child(div().h(px(1.)).my_1().bg(rgb(LINE)))
                    })
            }))
    }
}
impl Render for ContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.activation.is_none() {
            self.activation = Some(cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.close(window, cx);
                }
            }));
        }
        let mut cards = vec![
            anchored()
                .position(self.position)
                .snap_to_window_with_margin(px(4.))
                .child(self.card(0, &self.items, self.selected, cx)),
        ];
        for (index, level) in self.submenus.iter().enumerate() {
            cards.push(
                anchored()
                    .position(level.position)
                    .snap_to_window_with_margin(px(4.))
                    .child(self.card(index + 1, &level.items, level.selected, cx)),
            );
        }
        deferred(
            div()
                .id("context-menu-overlay")
                .absolute()
                .inset_0()
                .occlude()
                .track_focus(&self.focus)
                .on_key_down(cx.listener(Self::key_down))
                .on_any_mouse_down(cx.listener(|this, _, window, cx| {
                    this.close(window, cx);
                    cx.stop_propagation();
                }))
                .on_scroll_wheel(cx.listener(|this, _, window, cx| {
                    this.close(window, cx);
                    cx.stop_propagation();
                }))
                .children(cards),
        )
        .with_priority(10)
    }
}

#[cfg(test)]
mod tests {
    use super::next_enabled;

    #[test]
    fn submenu_only_opens_enabled_children_and_preserves_child_selection() {
        let child = super::MenuItem::new("disabled", "", false, |_, _| {});
        let parent = super::MenuItem::submenu("rebase", vec![child]);
        assert!(super::child_level(&parent, gpui::point(gpui::px(0.), gpui::px(0.))).is_none());
        let parent = super::MenuItem::submenu(
            "rebase",
            vec![
                super::MenuItem::new("disabled", "", false, |_, _| {}),
                super::MenuItem::new("forward", "", true, |_, _| {}),
                super::MenuItem::new("reverse", "", true, |_, _| {}),
            ],
        );
        let level = super::child_level(&parent, gpui::point(gpui::px(12.), gpui::px(15.))).unwrap();
        assert_eq!(level.selected, Some(1));
        assert_eq!(level.items.len(), 3);
        assert_eq!(
            super::next_enabled(&[false, true, true], level.selected, true),
            Some(2)
        );
    }
    #[test]
    fn submenu_flips_at_the_right_window_edge() {
        use gpui::{Bounds, point, px, size};
        let left = Bounds::new(point(px(10.), px(20.)), size(px(350.), px(30.)));
        assert_eq!(
            super::submenu_position(left, px(1000.)),
            point(px(364.), px(20.))
        );
        let right = Bounds::new(point(px(600.), px(20.)), size(px(350.), px(30.)));
        assert_eq!(
            super::submenu_position(right, px(1000.)),
            point(px(236.), px(20.))
        );
    }
    #[test]
    fn option_menu_starts_at_checked_enabled_item_and_falls_back_safely() {
        let item = |enabled, checked| {
            super::MenuItem::new("option", "", enabled, |_, _| {}).checked(checked)
        };
        assert_eq!(
            super::initial_selection(&[item(true, false), item(true, true)]),
            Some(1)
        );
        assert_eq!(
            super::initial_selection(&[item(true, false), item(false, true)]),
            Some(0)
        );
        assert_eq!(super::initial_selection(&[item(false, true)]), None);
        assert_eq!(super::initial_selection(&[]), None);
    }
    #[test]
    fn displayed_shortcuts_match_only_the_exact_key_combination() {
        let key = gpui::Keystroke::parse("ctrl-d").unwrap();
        assert!(super::shortcut_matches("Ctrl + D", &key));
        assert!(!super::shortcut_matches("", &key));
        assert!(!super::shortcut_matches("Space", &key));
        assert!(!super::shortcut_matches(
            "Ctrl + D",
            &gpui::Keystroke::parse("ctrl-shift-d").unwrap()
        ));
        assert!(super::shortcut_matches(
            "Space",
            &gpui::Keystroke::parse("space").unwrap()
        ));
        assert!(super::shortcut_matches(
            "Ctrl + Shift + C",
            &gpui::Keystroke::parse("ctrl-shift-c").unwrap()
        ));
        assert!(!super::shortcut_matches(
            "Ctrl + Shift + C",
            &gpui::Keystroke::parse("ctrl-c").unwrap()
        ));
    }

    #[test]
    fn menu_navigation_skips_disabled_items_and_wraps() {
        let enabled = [true, false, true];
        assert_eq!(next_enabled(&enabled, None, true), Some(0));
        assert_eq!(next_enabled(&enabled, Some(0), true), Some(2));
        assert_eq!(next_enabled(&enabled, Some(2), true), Some(0));
        assert_eq!(next_enabled(&enabled, Some(0), false), Some(2));
        assert_eq!(next_enabled(&[false, false], Some(0), true), None);
        assert_eq!(next_enabled(&[], None, false), None);
    }
}
