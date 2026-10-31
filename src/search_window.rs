use crate::{
    commit_editor::{CommitEditor, read_only_key},
    history_view,
    shortcut::Bindings,
    theme,
};
use feather_git::git_search;
use gpui::{prelude::*, *};
use std::path::PathBuf;

pub struct FileHistoryWindow {
    root: PathBuf,
    path: String,
    shortcuts: Bindings,
    commits: Vec<git_search::FileCommit>,
    selected: Option<String>,
    selected_index: usize,
    list_focus: FocusHandle,
    list_scroll: UniformListScrollHandle,
    list_height: f32,
    height_drag: Option<(f32, f32)>,
    column_widths: [f32; 3],
    column_drag: Option<(usize, f32, [f32; 3], f32)>,
    diff_editor: Entity<CommitEditor>,
    error: Option<String>,
}

const DEFAULT_COLUMN_WIDTHS: [f32; 3] = [180., 142., 86.];
const MIN_COLUMN_WIDTHS: [f32; 4] = [72., 80., 80., 60.];

fn resize_grid_columns(widths: [f32; 3], boundary: usize, delta: f32, total: f32) -> [f32; 3] {
    if boundary >= 3 || !delta.is_finite() || !total.is_finite() {
        return widths;
    }
    let cells = [
        (total - widths.iter().sum::<f32>()).max(0.),
        widths[0],
        widths[1],
        widths[2],
    ];
    let minimum_left = MIN_COLUMN_WIDTHS[boundary].min(cells[boundary]);
    let minimum_right = MIN_COLUMN_WIDTHS[boundary + 1].min(cells[boundary + 1]);
    let delta = delta.clamp(
        minimum_left - cells[boundary],
        cells[boundary + 1] - minimum_right,
    );
    let mut resized = widths;
    if boundary == 0 {
        resized[0] -= delta;
    } else {
        resized[boundary - 1] += delta;
        resized[boundary] -= delta;
    }
    resized
}

fn commit_navigation(bindings: &Bindings, key: &Keystroke) -> Option<bool> {
    let mapped = bindings.resolve("history", key)?;
    if mapped.modifiers.shift
        || mapped.modifiers.control
        || mapped.modifiers.alt
        || mapped.modifiers.platform
    {
        return None;
    }
    match mapped.key.as_str() {
        "up" | "left" => Some(false),
        "down" | "right" => Some(true),
        _ => None,
    }
}

fn viewer_shortcut(bindings: &Bindings, key: &Keystroke) -> Option<Keystroke> {
    bindings.resolve("viewer", key).filter(read_only_key)
}

fn numbered_lines(diff: &str) -> Vec<(Option<usize>, Option<usize>, String)> {
    let mut old = 0;
    let mut new = 0;
    diff.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("@@ ") {
                let mut parts = rest.split_whitespace();
                old = parts
                    .next()
                    .and_then(|part| part.strip_prefix('-'))
                    .and_then(|part| part.split(',').next())
                    .and_then(|part| part.parse().ok())
                    .unwrap_or(0);
                new = parts
                    .next()
                    .and_then(|part| part.strip_prefix('+'))
                    .and_then(|part| part.split(',').next())
                    .and_then(|part| part.parse().ok())
                    .unwrap_or(0);
                return (None, None, line.to_owned());
            }
            match line.chars().next() {
                Some('-') if !line.starts_with("--- ") => {
                    let number = old;
                    old += 1;
                    (Some(number), None, line.to_owned())
                }
                Some('+') if !line.starts_with("+++ ") => {
                    let number = new;
                    new += 1;
                    (None, Some(number), line.to_owned())
                }
                Some(' ') => {
                    let pair = (Some(old), Some(new), line.to_owned());
                    old += 1;
                    new += 1;
                    pair
                }
                _ => (None, None, line.to_owned()),
            }
        })
        .collect()
}

fn numbered_text(diff: &str) -> String {
    numbered_lines(diff)
        .into_iter()
        .map(|(old, new, line)| {
            let (marker, body) = if line.starts_with("@@") {
                ("@", line.as_str())
            } else if line.starts_with("--- ") || line.starts_with("+++ ") {
                (" ", line.as_str())
            } else {
                line.split_at(line.chars().next().map_or(0, char::len_utf8))
            };
            format!(
                "{:>5} {:>5}  {:^3}{}",
                old.map(|n| n.to_string()).unwrap_or_default(),
                new.map(|n| n.to_string()).unwrap_or_default(),
                marker,
                body
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn grid_cell(text: String, width: Option<f32>) -> Div {
    div()
        .h(px(24.))
        .min_w_0()
        .overflow_hidden()
        .px_2()
        .flex()
        .items_center()
        .text_xs()
        .border_r_1()
        .border_color(rgb(theme::LINE))
        .when_some(width, |cell, width| cell.w(px(width)).flex_shrink_0())
        .when(width.is_none(), |cell| cell.flex_1())
        .child(div().min_w_0().text_ellipsis().child(text))
}

fn grid_guide(width: Option<f32>) -> Div {
    div()
        .h_full()
        .min_w_0()
        .border_r_1()
        .border_color(rgb(theme::LINE))
        .when_some(width, |cell, width| cell.w(px(width)).flex_shrink_0())
        .when(width.is_none(), |cell| cell.flex_1())
}

impl FileHistoryWindow {
    fn history_row(&self, index: usize, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let commit = &self.commits[index];
        let selected = self.selected.as_deref() == Some(commit.id.as_str());
        let target = commit.id.clone();
        div()
            .id(("file-commit", index))
            .w_full()
            .min_w_0()
            .h(px(24.))
            .flex()
            .items_center()
            .text_xs()
            .cursor_pointer()
            .bg(rgb(if selected {
                if self.list_focus.is_focused(window) {
                    theme::SELECT
                } else {
                    theme::SELECT_INACTIVE
                }
            } else {
                theme::PANEL
            }))
            .when(!selected, |row| {
                row.hover(|row| row.bg(rgb(theme::HISTORY_HOVER)))
            })
            .child(grid_cell(commit.description.clone(), None))
            .child(grid_cell(
                format!("{} <{}>", commit.author, commit.author_email),
                Some(self.column_widths[0]),
            ))
            .child(grid_cell(
                history_view::format_time(commit.time, commit.time_offset),
                Some(self.column_widths[1]),
            ))
            .child(grid_cell(
                commit.id.chars().take(8).collect(),
                Some(self.column_widths[2]),
            ))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.list_focus.focus(window);
                this.selected_index = index;
                this.load_diff(target.clone(), cx);
                cx.notify();
            }))
    }

    pub fn new(root: PathBuf, path: String, shortcuts: Bindings, cx: &mut Context<Self>) -> Self {
        let diff_editor = cx.new(|cx| {
            let mut editor = CommitEditor::diff_read_only("", cx);
            editor.text_size = 12.;
            editor
        });
        let task_root = root.clone();
        let task_path = path.clone();
        let task = cx
            .background_executor()
            .spawn(async move { git_search::file_history(&task_root, &task_path) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(commits) => {
                        if let Some(commit) = commits.first() {
                            this.load_diff(commit.id.clone(), cx);
                        }
                        this.commits = commits;
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        Self {
            root,
            path,
            shortcuts,
            commits: Vec::new(),
            selected: None,
            selected_index: 0,
            list_focus: cx.focus_handle(),
            list_scroll: UniformListScrollHandle::new(),
            list_height: 280.,
            height_drag: None,
            column_widths: DEFAULT_COLUMN_WIDTHS,
            column_drag: None,
            diff_editor,
            error: None,
        }
    }

    fn load_diff(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected = Some(id.clone());
        self.diff_editor
            .update(cx, |editor, cx| editor.set_text("", cx));
        let root = self.root.clone();
        let path = self.path.clone();
        let request_id = id.clone();
        let task = cx
            .background_executor()
            .spawn(async move { git_search::file_diff(&root, &path, &request_id) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.selected.as_deref() == Some(id.as_str()) {
                    match result {
                        Ok(diff) => this
                            .diff_editor
                            .update(cx, |editor, cx| editor.set_text(&numbered_text(&diff), cx)),
                        Err(error) => this.error = Some(error.to_string()),
                    }
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if !key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform {
            match key.key.as_str() {
                "escape" => {
                    window.remove_window();
                    cx.stop_propagation();
                    return;
                }
                "1" => {
                    self.list_focus.focus(window);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "2" => {
                    self.diff_editor.read(cx).focus.focus(window);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }
        if self.diff_editor.read(cx).focus.is_focused(window) {
            if let Some(mapped) = viewer_shortcut(&self.shortcuts, key) {
                self.diff_editor.update(cx, |editor, cx| {
                    editor.apply_shortcut_key(&mapped, cx);
                });
                cx.stop_propagation();
                return;
            }
            if read_only_key(key) {
                cx.stop_propagation();
                return;
            }
        }
        if self.list_focus.is_focused(window)
            && !self.commits.is_empty()
            && let Some(down) = commit_navigation(&self.shortcuts, key)
        {
            let next = if !down {
                self.selected_index.saturating_sub(1)
            } else {
                (self.selected_index + 1).min(self.commits.len() - 1)
            };
            if next != self.selected_index || self.selected.is_none() {
                self.selected_index = next;
                self.list_scroll
                    .scroll_to_item(next, ScrollStrategy::Center);
                self.load_diff(self.commits[next].id.clone(), cx);
            }
            cx.stop_propagation();
            cx.notify();
        }
    }
}

impl Render for FileHistoryWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if window.focused(cx).is_none() {
            self.list_focus.focus(window);
        }
        let list_height = self
            .list_height
            .min((f32::from(window.viewport_size().height) - 210.).max(120.));
        let column_widths = self.column_widths;
        let history_rows = uniform_list(
            "file-history-commits",
            self.commits.len(),
            cx.processor(|this, range: std::ops::Range<usize>, window, cx| {
                range
                    .map(|index| this.history_row(index, window, cx))
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(self.list_scroll.clone())
        .flex_1()
        .min_h_0()
        .w_full();
        let history = div()
            .id("file-history-list")
            .relative()
            .text_sm()
            .flex()
            .flex_col()
            .w_full()
            .flex_shrink_0()
            .h(px(list_height))
            .min_h_0()
            .bg(rgb(theme::PANEL))
            .track_focus(&self.list_focus)
            .child(
                div()
                    .absolute()
                    .top(px(24.))
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .flex()
                    .child(grid_guide(None))
                    .child(grid_guide(Some(column_widths[0])))
                    .child(grid_guide(Some(column_widths[1])))
                    .child(grid_guide(Some(column_widths[2]))),
            )
            .child({
                let mut header = div()
                    .w_full()
                    .h(px(24.))
                    .flex_shrink_0()
                    .flex()
                    .border_b_1()
                    .border_color(rgb(theme::LINE))
                    .bg(rgb(theme::BG))
                    .text_color(rgb(theme::MUTED));
                for (index, (label, width)) in [
                    ("Description", None),
                    ("Author", Some(column_widths[0])),
                    ("Date", Some(column_widths[1])),
                    ("Hash", Some(column_widths[2])),
                ]
                .into_iter()
                .enumerate()
                {
                    let cell = grid_cell(label.into(), width)
                        .relative()
                        .when(index < 3, |cell| {
                            cell.child(
                                div()
                                    .id(("file-column-resize", index))
                                    .absolute()
                                    .right_0()
                                    .top_0()
                                    .w(px(7.))
                                    .h_full()
                                    .cursor(CursorStyle::ResizeLeftRight)
                                    .occlude()
                                    .hover(|style| style.bg(rgb(theme::ACCENT)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this, event: &MouseDownEvent, window, cx| {
                                                this.height_drag = None;
                                                this.column_drag = Some((
                                                    index,
                                                    f32::from(event.position.x),
                                                    this.column_widths,
                                                    f32::from(window.viewport_size().width),
                                                ));
                                                cx.stop_propagation();
                                            },
                                        ),
                                    ),
                            )
                        });
                    header = header.child(cell);
                }
                header
            })
            .child(history_rows);
        let diff = div()
            .id("file-history-diff")
            .text_xs()
            .flex_1()
            .min_w_0()
            .h_full()
            .min_h_0()
            .bg(rgb(theme::PANEL))
            .child(self.diff_editor.clone());
        let splitter = div()
            .id("file-history-splitter")
            .absolute()
            .left_0()
            .top(px(list_height - 3.))
            .w_full()
            .h(px(7.))
            .cursor(CursorStyle::ResizeUpDown)
            .occlude()
            .flex()
            .items_center()
            .child(div().h(px(1.)).w_full().bg(rgb(theme::LINE)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.column_drag = None;
                    this.height_drag = Some((f32::from(event.position.y), list_height));
                    cx.stop_propagation();
                }),
            );
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::PANEL))
            .text_color(rgb(theme::TEXT))
            .text_sm()
            .child(
                div()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .px_3()
                    .text_xs()
                    .border_b_1()
                    .border_color(rgb(theme::LINE))
                    .child(self.path.clone()),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(div().px_4().text_color(rgb(theme::ERROR)).child(error))
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .child(history)
                    .child(diff)
                    .child(splitter),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if let Some((boundary, start, widths, total)) = this.column_drag {
                    if event.pressed_button == Some(MouseButton::Left) {
                        this.column_widths = resize_grid_columns(
                            widths,
                            boundary,
                            f32::from(event.position.x) - start,
                            total,
                        );
                        cx.stop_propagation();
                        cx.notify();
                    } else {
                        this.column_drag = None;
                    }
                }
                if let Some((start, height)) = this.height_drag {
                    if event.pressed_button == Some(MouseButton::Left) {
                        let max_height =
                            (f32::from(window.viewport_size().height) - 210.).max(120.);
                        this.list_height =
                            (height + f32::from(event.position.y) - start).clamp(120., max_height);
                        cx.stop_propagation();
                        cx.notify();
                    } else {
                        this.height_drag = None;
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let height_dragged = this.height_drag.take().is_some();
                    let column_dragged = this.column_drag.take().is_some();
                    if height_dragged || column_dragged {
                        cx.stop_propagation();
                    }
                }),
            )
            .capture_key_down(
                cx.listener(|this, event, window, cx| this.key_down(event, window, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        commit_navigation, numbered_lines, numbered_text, resize_grid_columns, viewer_shortcut,
    };
    use crate::shortcut::Bindings;
    use gpui::Keystroke;

    #[test]
    fn grid_column_resize_moves_adjacent_boundaries_and_honors_minimums() {
        let initial = [180., 142., 86.];
        assert_eq!(
            resize_grid_columns(initial, 0, 50., 960.),
            [130., 142., 86.]
        );
        assert_eq!(
            resize_grid_columns(initial, 1, 30., 960.),
            [210., 112., 86.]
        );
        assert_eq!(
            resize_grid_columns(initial, 2, 20., 960.),
            [180., 162., 66.]
        );
        assert_eq!(
            resize_grid_columns(initial, 0, 500., 960.),
            [80., 142., 86.]
        );
        assert_eq!(
            resize_grid_columns(initial, 1, 500., 960.),
            [242., 80., 86.]
        );
        assert_eq!(
            resize_grid_columns(initial, 2, 500., 960.),
            [180., 168., 60.]
        );
        assert_eq!(
            resize_grid_columns(initial, 0, -500., 960.),
            [660., 142., 86.]
        );
        assert_eq!(resize_grid_columns(initial, 3, 50., 960.), initial);
    }

    #[test]
    fn file_history_navigation_uses_common_direction_bindings() {
        let mut bindings = Bindings::default();
        for (key, direction) in [
            ("up", false),
            ("i", false),
            ("left", false),
            ("j", false),
            ("down", true),
            ("k", true),
            ("right", true),
            ("l", true),
        ] {
            assert_eq!(
                commit_navigation(&bindings, &Keystroke::parse(key).unwrap()),
                Some(direction)
            );
        }
        bindings.set("nav-up", 0, "alt-u".into()).unwrap();
        bindings.set("nav-right", 0, "alt-r".into()).unwrap();
        assert_eq!(
            commit_navigation(&bindings, &Keystroke::parse("alt-u").unwrap()),
            Some(false)
        );
        assert_eq!(
            commit_navigation(&bindings, &Keystroke::parse("alt-r").unwrap()),
            Some(true)
        );
        assert_eq!(
            commit_navigation(&bindings, &Keystroke::parse("up").unwrap()),
            None
        );
        assert_eq!(
            commit_navigation(&bindings, &Keystroke::parse("right").unwrap()),
            None
        );
    }

    #[test]
    fn file_history_diff_uses_viewer_shortcuts_and_remaps() {
        let mut bindings = Bindings::default();
        for (input, target) in [
            ("i", "up"),
            ("j", "left"),
            ("k", "down"),
            ("l", "right"),
            ("shift-i", "shift-up"),
            ("shift-l", "shift-right"),
            ("home", "home"),
            ("shift-end", "shift-end"),
            ("ctrl-a", "ctrl-a"),
            ("ctrl-c", "ctrl-c"),
        ] {
            assert_eq!(
                crate::shortcut::chord(
                    &viewer_shortcut(&bindings, &Keystroke::parse(input).unwrap()).unwrap()
                ),
                target
            );
        }
        bindings.set("nav-up", 0, "alt-u".into()).unwrap();
        bindings.set("viewer-copy", 0, "alt-y".into()).unwrap();
        assert_eq!(
            viewer_shortcut(&bindings, &Keystroke::parse("alt-u").unwrap())
                .unwrap()
                .key,
            "up"
        );
        assert_eq!(
            crate::shortcut::chord(
                &viewer_shortcut(&bindings, &Keystroke::parse("alt-y").unwrap()).unwrap()
            ),
            "ctrl-c"
        );
        assert!(viewer_shortcut(&bindings, &Keystroke::parse("up").unwrap()).is_none());
        assert!(viewer_shortcut(&bindings, &Keystroke::parse("ctrl-c").unwrap()).is_none());
    }

    #[test]
    fn diff_numbers_follow_hunks_and_added_deleted_lines() {
        let rows = numbered_lines("--- a/f\n+++ b/f\n@@ -4,2 +4,3 @@\n same\n-old\n+new\n+added\n");
        assert_eq!(rows[0].0, None);
        assert_eq!((rows[3].0, rows[3].1), (Some(4), Some(4)));
        assert_eq!((rows[4].0, rows[4].1), (Some(5), None));
        assert_eq!((rows[5].0, rows[5].1), (None, Some(5)));
        assert_eq!((rows[6].0, rows[6].1), (None, Some(6)));
    }

    #[test]
    fn diff_text_keeps_numbers_marker_and_content_in_separate_columns() {
        let text = numbered_text("@@ -1 +1 @@\n-old\n+new\n");
        let rows: Vec<_> = text.lines().collect();
        assert_eq!(&rows[1][13..16], " - ");
        assert_eq!(&rows[1][16..], "old");
        assert_eq!(&rows[2][13..16], " + ");
        assert_eq!(&rows[2][16..], "new");
    }
}
