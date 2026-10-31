use crate::theme;
use gpui::{prelude::*, *};
use std::ops::Range;

fn text_vertical_offset(single_line: bool, height: f32) -> f32 {
    if single_line {
        ((height - 22.) / 2.).max(0.)
    } else {
        0.
    }
}

fn normalized_input(text: &str, single_line: bool) -> String {
    if single_line {
        text.replace(['\r', '\n', '\t'], "")
    } else {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }
}

fn masked_text(text: &str) -> String {
    "*".repeat(text.len())
}
fn source_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(index, _)| index + 1))
        .collect()
}

fn visible_rows(offset: f32, viewport: f32, count: usize) -> Range<usize> {
    let first = ((-offset).max(0.) / 22.).floor() as usize;
    let visible = (viewport.max(0.) / 22.).ceil() as usize;
    first.saturating_sub(2).min(count)..(first + visible + 3).min(count)
}

fn diff_body_start(start: usize, line_len: usize) -> usize {
    start + 16.min(line_len)
}

pub fn read_only_key(key: &Keystroke) -> bool {
    if key.modifiers.alt || key.modifiers.platform {
        return false;
    }
    if key.modifiers.control {
        return matches!(key.key.as_str(), "a" | "c");
    }
    matches!(
        key.key.as_str(),
        "left" | "right" | "up" | "down" | "home" | "end"
    )
}

fn wrapped_ranges(text: &str, width: f32, x_for_index: impl Fn(usize) -> f32) -> Vec<Range<usize>> {
    let mut rows = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        let end = index + character.len_utf8();
        if index > start && x_for_index(end) - x_for_index(start) > width.max(1.) {
            rows.push(start..index);
            start = index;
        }
    }
    rows.push(start..text.len());
    rows
}

#[derive(Default)]
struct Buffer {
    text: String,
    anchor: usize,
    cursor: usize,
    marked: Option<Range<usize>>,
}

impl Buffer {
    fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut count = 0;
        for (index, character) in self.text.char_indices() {
            if count >= offset {
                return index;
            }
            count += character.len_utf16();
        }
        self.text.len()
    }
    fn to_utf16(&self, offset: usize) -> usize {
        self.text[..offset].encode_utf16().count()
    }
    fn utf8_range(&self, range: Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }
    fn utf16_range(&self, range: Range<usize>) -> Range<usize> {
        self.to_utf16(range.start)..self.to_utf16(range.end)
    }
    fn replace(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        marked: bool,
        selected: Option<Range<usize>>,
    ) {
        let range = range
            .map(|range| self.utf8_range(range))
            .or(self.marked.clone())
            .unwrap_or_else(|| self.selection());
        self.text.replace_range(range.clone(), text);
        self.marked = (marked && !text.is_empty()).then_some(range.start..range.start + text.len());
        let relative = selected
            .map(|range| {
                let temporary = Buffer {
                    text: text.to_owned(),
                    ..Buffer::default()
                };
                temporary.utf8_range(range)
            })
            .unwrap_or(text.len()..text.len());
        self.anchor = range.start + relative.start;
        self.cursor = range.start + relative.end;
    }
    fn previous(&self, offset: usize) -> usize {
        self.text[..offset]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index)
    }
    fn next(&self, offset: usize) -> usize {
        self.text[offset..]
            .chars()
            .next()
            .map_or(offset, |character| offset + character.len_utf8())
    }
    fn move_to(&mut self, offset: usize, extend: bool) {
        self.cursor = offset;
        if !extend {
            self.anchor = offset;
        }
        self.marked = None;
    }
}

pub struct CommitEditor {
    buffer: Buffer,
    single_line: bool,
    read_only: bool,
    diff_coloring: bool,
    secret: bool,
    pub focus: FocusHandle,
    pub enabled: bool,
    pub text_size: f32,
    scroll: ScrollHandle,
    bounds: Option<Bounds<Pixels>>,
    lines: Vec<(usize, ShapedLine)>,
    visible_row_start: usize,
    diff_line_starts: Vec<usize>,
    diff_max_columns: usize,
    dragging: bool,
    reveal_cursor: bool,
    viewport: Size<Pixels>,
    tracks: [Bounds<Pixels>; 2],
    scroll_drag: Option<(bool, f32)>,
}

impl CommitEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            buffer: Buffer::default(),
            single_line: false,
            read_only: false,
            diff_coloring: false,
            secret: false,
            focus: cx.focus_handle(),
            enabled: true,
            text_size: 14.,
            scroll: ScrollHandle::new(),
            bounds: None,
            lines: Vec::new(),
            visible_row_start: 0,
            diff_line_starts: Vec::new(),
            diff_max_columns: 0,
            dragging: false,
            reveal_cursor: false,
            viewport: Size::default(),
            tracks: [Bounds::default(); 2],
            scroll_drag: None,
        }
    }
    pub fn message(&self) -> &str {
        &self.buffer.text
    }

    pub fn read_only(text: &str, cx: &mut Context<Self>) -> Self {
        let mut editor = Self::new(cx);
        editor.buffer.text = text.to_owned();
        editor.read_only = true;
        editor.text_size = 13.;
        editor
    }

    pub fn diff_read_only(text: &str, cx: &mut Context<Self>) -> Self {
        let mut editor = Self::read_only(text, cx);
        editor.diff_coloring = true;
        editor.diff_line_starts = line_starts(&editor.buffer.text);
        editor.diff_max_columns = editor
            .buffer
            .text
            .split('\n')
            .map(|line| {
                line.chars()
                    .map(|character| usize::from(!character.is_ascii()) + 1)
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0);
        editor
    }

    fn editable(&self) -> bool {
        self.enabled && !self.read_only
    }

    pub fn background_color(&self) -> u32 {
        if self.read_only {
            theme::PANEL
        } else {
            theme::INPUT
        }
    }

    pub fn set_read_only(&mut self, read_only: bool, cx: &mut Context<Self>) {
        self.read_only = read_only;
        cx.notify();
    }
    pub fn single_line(text: &str, cx: &mut Context<Self>) -> Self {
        let mut editor = Self::new(cx);
        editor.single_line = true;
        editor.set_text(text, cx);
        editor
    }
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.secret {
            unsafe {
                self.buffer.text.as_bytes_mut().fill(0);
            }
        }
        self.buffer = Buffer {
            text: self.input_text(text),
            ..Buffer::default()
        };
        if self.diff_coloring {
            self.diff_line_starts = line_starts(&self.buffer.text);
            self.diff_max_columns = self
                .buffer
                .text
                .split('\n')
                .map(|line| {
                    line.chars()
                        .map(|character| usize::from(!character.is_ascii()) + 1)
                        .sum::<usize>()
                })
                .max()
                .unwrap_or(0);
            self.lines.clear();
            self.visible_row_start = 0;
            self.scroll.set_offset(point(px(0.), px(0.)));
        }
        self.changed(cx);
    }
    fn input_text(&self, text: &str) -> String {
        normalized_input(text, self.single_line)
    }
    pub fn password(cx: &mut Context<Self>) -> Self {
        let mut editor = Self::single_line("", cx);
        editor.secret = true;
        editor
    }
    pub fn take_secret(&mut self, cx: &mut Context<Self>) -> feather_git::auth_profiles::Secret {
        let text = std::mem::take(&mut self.buffer.text);
        self.buffer = Buffer::default();
        self.changed(cx);
        feather_git::auth_profiles::Secret::new(text)
    }
    pub fn is_composing(&self) -> bool {
        self.buffer.marked.is_some()
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.reveal_cursor = true;
        cx.notify();
    }
    fn index_at(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.bounds else {
            return 0;
        };
        let row = (f32::from(position.y - bounds.origin.y) / 22.).max(0.) as usize;
        let local_row = if self.diff_coloring {
            row.saturating_sub(self.visible_row_start)
        } else {
            row
        };
        let Some((start, line)) = self
            .lines
            .get(local_row.min(self.lines.len().saturating_sub(1)))
        else {
            return 0;
        };
        let index = source_boundary(
            &self.buffer.text,
            start + line.closest_index_for_x((position.x - bounds.origin.x).max(px(0.))),
        );
        if self.diff_coloring {
            index.max(diff_body_start(*start, line.len()))
        } else {
            index
        }
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.apply_shortcut_key(&event.keystroke, cx);
    }

    pub fn apply_shortcut_key(&mut self, key: &Keystroke, cx: &mut Context<Self>) {
        if !self.enabled {
            cx.stop_propagation();
            return;
        }
        if self.read_only && !read_only_key(key) {
            cx.stop_propagation();
            return;
        }
        if self.buffer.marked.is_some() && !key.modifiers.control {
            return;
        }
        if key.modifiers.control && ["a", "c", "x", "v"].contains(&key.key.as_str()) {
            match key.key.as_str() {
                "a" => {
                    self.buffer.anchor = 0;
                    self.buffer.cursor = self.buffer.text.len();
                }
                "c" | "x" => {
                    let range = self.buffer.selection();
                    if !self.secret && !range.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.buffer.text[range].to_owned(),
                        ));
                        if key.key == "x" {
                            self.buffer.replace(None, "", false, None);
                        }
                    }
                }
                "v" => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        self.buffer
                            .replace(None, &self.input_text(&text), false, None);
                    }
                }
                _ => {}
            }
        } else if !key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform {
            match key.key.as_str() {
                "enter" if !self.single_line => self.buffer.replace(None, "\n", false, None),
                "tab" if !self.single_line => self.buffer.replace(None, "    ", false, None),
                "backspace" | "delete" => {
                    if self.buffer.selection().is_empty() {
                        self.buffer.anchor = if key.key == "backspace" {
                            self.buffer.previous(self.buffer.cursor)
                        } else {
                            self.buffer.next(self.buffer.cursor)
                        };
                    }
                    self.buffer.replace(None, "", false, None);
                }
                "left" | "right" | "up" | "down" | "home" | "end" => {
                    let cursor = self.buffer.cursor;
                    let start = self.buffer.text[..cursor]
                        .rfind('\n')
                        .map_or(0, |index| index + 1);
                    let end = self.buffer.text[cursor..]
                        .find('\n')
                        .map_or(self.buffer.text.len(), |index| index + cursor);
                    let column = self.buffer.text[start..cursor].chars().count();
                    let next = match key.key.as_str() {
                        "left" => {
                            if !key.modifiers.shift && !self.buffer.selection().is_empty() {
                                self.buffer.selection().start
                            } else {
                                self.buffer.previous(cursor)
                            }
                        }
                        "right" => {
                            if !key.modifiers.shift && !self.buffer.selection().is_empty() {
                                self.buffer.selection().end
                            } else {
                                self.buffer.next(cursor)
                            }
                        }
                        "home" => start,
                        "end" => end,
                        "up" | "down" => {
                            if self.diff_coloring && !self.diff_line_starts.is_empty() {
                                let row = self
                                    .diff_line_starts
                                    .partition_point(|start| *start <= cursor)
                                    .saturating_sub(1);
                                let target = if key.key == "up" {
                                    row.saturating_sub(1)
                                } else {
                                    (row + 1).min(self.diff_line_starts.len() - 1)
                                };
                                let column = self.buffer.text[self.diff_line_starts[row]..cursor]
                                    .chars()
                                    .count()
                                    .max(16);
                                let target_start = self.diff_line_starts[target];
                                let target_end = self
                                    .diff_line_starts
                                    .get(target + 1)
                                    .map_or(self.buffer.text.len(), |next| next - 1);
                                self.buffer.text[target_start..target_end]
                                    .char_indices()
                                    .nth(column)
                                    .map_or(target_end, |(index, _)| target_start + index)
                            } else if self.read_only && !self.lines.is_empty() {
                                let row = self
                                    .lines
                                    .iter()
                                    .rposition(|(start, _)| *start <= cursor)
                                    .unwrap_or(0);
                                let column =
                                    self.buffer.text[self.lines[row].0..cursor].chars().count();
                                let target = if key.key == "up" {
                                    row.saturating_sub(1)
                                } else {
                                    (row + 1).min(self.lines.len() - 1)
                                };
                                let (start, line) = &self.lines[target];
                                let text = &self.buffer.text[*start..*start + line.len()];
                                text.char_indices()
                                    .nth(column)
                                    .map_or(*start + text.len(), |(index, _)| *start + index)
                            } else {
                                let target = if key.key == "up" {
                                    if start == 0 {
                                        0
                                    } else {
                                        self.buffer.text[..start - 1]
                                            .rfind('\n')
                                            .map_or(0, |index| index + 1)
                                    }
                                } else {
                                    (end + 1).min(self.buffer.text.len())
                                };
                                self.buffer.text[target..]
                                    .split('\n')
                                    .next()
                                    .unwrap_or("")
                                    .char_indices()
                                    .nth(column)
                                    .map_or_else(
                                        || {
                                            target
                                                + self.buffer.text[target..]
                                                    .split('\n')
                                                    .next()
                                                    .unwrap_or("")
                                                    .len()
                                        },
                                        |(index, _)| target + index,
                                    )
                            }
                        }
                        _ => cursor,
                    };
                    let next = if self.diff_coloring {
                        let row = self
                            .diff_line_starts
                            .partition_point(|start| *start <= next)
                            .saturating_sub(1);
                        let start = self.diff_line_starts.get(row).copied().unwrap_or(0);
                        let end = self
                            .diff_line_starts
                            .get(row + 1)
                            .map_or(self.buffer.text.len(), |next| next - 1);
                        let minimum = diff_body_start(start, end.saturating_sub(start));
                        if next < minimum && key.key == "left" && cursor == minimum && row > 0 {
                            start.saturating_sub(1)
                        } else {
                            next.max(minimum)
                        }
                    } else {
                        next
                    };
                    self.buffer.move_to(next, key.modifiers.shift);
                }
                _ => return,
            }
        } else {
            return;
        }
        self.changed(cx);
        cx.stop_propagation();
    }

    fn drag_scrollbar(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((horizontal, grab)) = self.scroll_drag else {
            return;
        };
        let bounds = self.tracks[usize::from(horizontal)];
        let maximum = self.scroll.max_offset();
        let mut offset = self.scroll.offset();
        let (track, maximum, local) = if horizontal {
            (
                bounds.size.width,
                maximum.width,
                position.x - bounds.origin.x,
            )
        } else {
            (
                bounds.size.height,
                maximum.height,
                position.y - bounds.origin.y,
            )
        };
        let value = crate::scrolling::Thumb::new(track.into(), maximum.into(), 0.)
            .offset(f32::from(local) - grab);
        if horizontal {
            offset.x = px(-value);
        } else {
            offset.y = px(-value);
        }
        self.scroll.set_offset(offset);
        cx.notify();
    }

    fn scrollbar(&self, horizontal: bool, cx: &Context<Self>) -> Stateful<Div> {
        let handle = self.scroll.clone();
        let view = cx.weak_entity();
        div()
            .id(if horizontal {
                "message-horizontal-scroll"
            } else {
                "message-vertical-scroll"
            })
            .flex_shrink_0()
            .bg(rgb(self.background_color()))
            .cursor_pointer()
            .occlude()
            .when(horizontal, |bar| bar.h(px(12.)).w_full())
            .when(!horizontal, |bar| bar.w(px(12.)).h_full())
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let maximum = handle.max_offset();
                        let offset = handle.offset();
                        let (track, maximum, current) = if horizontal {
                            (bounds.size.width, maximum.width, -offset.x)
                        } else {
                            (bounds.size.height, maximum.height, -offset.y)
                        };
                        let thumb = crate::scrolling::Thumb::new(
                            track.into(),
                            maximum.into(),
                            current.into(),
                        );
                        let rect = if horizontal {
                            Bounds::new(
                                bounds.origin + point(px(thumb.start), px(3.)),
                                size(px(thumb.length), px(6.)),
                            )
                        } else {
                            Bounds::new(
                                bounds.origin + point(px(3.), px(thumb.start)),
                                size(px(6.), px(thumb.length)),
                            )
                        };
                        window.paint_quad(quad(
                            rect,
                            px(3.),
                            rgb(theme::MUTED),
                            px(0.),
                            rgb(theme::MUTED),
                            Default::default(),
                        ));
                        view.update(cx, |this, _| this.tracks[usize::from(horizontal)] = bounds)
                            .ok();
                        let down = view.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture
                                || event.button != MouseButton::Left
                                || !bounds.contains(&event.position)
                            {
                                return;
                            }
                            down.update(cx, |this, cx| {
                                if !this.enabled {
                                    return;
                                }
                                let local = f32::from(if horizontal {
                                    event.position.x - bounds.origin.x
                                } else {
                                    event.position.y - bounds.origin.y
                                });
                                let grab = if local >= thumb.start
                                    && local <= thumb.start + thumb.length
                                {
                                    local - thumb.start
                                } else {
                                    thumb.length / 2.
                                };
                                this.dragging = false;
                                this.scroll_drag = Some((horizontal, grab));
                                this.focus.focus(window);
                                this.drag_scrollbar(event.position, cx);
                            })
                            .ok();
                        });
                        let moving = view.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase != DispatchPhase::Capture {
                                return;
                            }
                            moving
                                .update(cx, |this, cx| {
                                    if this.scroll_drag.is_some_and(|(axis, _)| axis == horizontal)
                                    {
                                        if event.pressed_button == Some(MouseButton::Left)
                                            && this.enabled
                                        {
                                            this.drag_scrollbar(event.position, cx);
                                        } else {
                                            this.scroll_drag = None;
                                        }
                                    }
                                })
                                .ok();
                        });
                        let up = view.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                up.update(cx, |this, _| this.scroll_drag = None).ok();
                            }
                        });
                    },
                )
                .size_full(),
            )
    }
}

impl Drop for CommitEditor {
    fn drop(&mut self) {
        if self.secret {
            unsafe {
                self.buffer.text.as_bytes_mut().fill(0);
            }
        }
    }
}

impl EntityInputHandler for CommitEditor {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        if self.secret {
            return None;
        }
        let range = self.buffer.utf8_range(range);
        *actual = Some(self.buffer.utf16_range(range.clone()));
        Some(self.buffer.text[range].to_owned())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        self.enabled.then(|| UTF16Selection {
            range: self.buffer.utf16_range(self.buffer.selection()),
            reversed: self.buffer.cursor < self.buffer.anchor,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.buffer
            .marked
            .clone()
            .map(|range| self.buffer.utf16_range(range))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editable() {
            self.buffer
                .replace(range, &self.input_text(text), false, None);
            self.changed(cx);
        }
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editable() {
            self.buffer
                .replace(range, &self.input_text(text), true, selected);
            self.changed(cx);
        }
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let bounds = self.bounds?;
        let range = self.buffer.utf8_range(range);
        let row = self
            .lines
            .iter()
            .rposition(|(start, _)| *start <= range.start)?;
        let (start, line) = &self.lines[row];
        if range.start > start + line.len() {
            return None;
        }
        Some(Bounds::new(
            bounds.origin
                + point(
                    line.x_for_index(range.start.saturating_sub(*start).min(line.len())),
                    px((row + self.visible_row_start) as f32 * 22.),
                ),
            size(px(2.), px(22.)),
        ))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.buffer.to_utf16(self.index_at(position)))
    }
}

impl Render for CommitEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let wrap = self.read_only && !self.single_line && !self.diff_coloring;
        let mut start = 0;
        let wrap_width = if self.viewport.width > px(0.) {
            f32::from(self.viewport.width) - 20.
        } else {
            560.
        }
        .max(1.);
        let secret = self.secret;
        let diff_coloring = self.diff_coloring;
        let text_size = self.text_size;
        let shape = |text: &str, color: u32| {
            let display = if secret {
                masked_text(text)
            } else {
                text.to_owned()
            };
            let prefix = if diff_coloring {
                display.len().min(13)
            } else {
                0
            };
            let runs = if prefix > 0 {
                vec![
                    TextRun {
                        len: prefix,
                        font: font("Cascadia Mono"),
                        color: rgb(theme::MUTED).into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    },
                    TextRun {
                        len: display.len() - prefix,
                        font: font("Cascadia Mono"),
                        color: rgb(color).into(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    },
                ]
            } else {
                vec![TextRun {
                    len: display.len(),
                    font: font("Cascadia Mono"),
                    color: rgb(color).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }]
            };
            window
                .text_system()
                .shape_line(display.into(), px(text_size), &runs, None)
        };
        let mut lines = Vec::new();
        let mut backgrounds = Vec::new();
        let row_range = if diff_coloring {
            visible_rows(
                f32::from(self.scroll.offset().y),
                if self.viewport.height > px(0.) {
                    f32::from(self.viewport.height)
                } else {
                    660.
                },
                self.diff_line_starts.len(),
            )
        } else {
            0..0
        };
        if diff_coloring {
            for row in row_range.clone() {
                let start = self.diff_line_starts[row];
                let end = self
                    .diff_line_starts
                    .get(row + 1)
                    .map_or(self.buffer.text.len(), |next| next - 1);
                let text = &self.buffer.text[start..end];
                let marker = text.as_bytes().get(14).copied();
                let (background, foreground) = match marker {
                    Some(b'+') => (theme::DIFF_ADD_BG, theme::DIFF_ADD_TEXT),
                    Some(b'-') => (theme::DIFF_DELETE_BG, theme::DIFF_DELETE_TEXT),
                    Some(b'@') | Some(b'\\') => (theme::DIFF_HEADER_BG, theme::DIFF_HEADER_TEXT),
                    _ => (theme::PANEL, theme::TEXT),
                };
                lines.push((start, shape(text, foreground)));
                backgrounds.push(background);
            }
        } else {
            for text in self.buffer.text.split('\n') {
                let line = shape(text, theme::TEXT);
                if wrap {
                    for range in
                        wrapped_ranges(text, wrap_width, |index| line.x_for_index(index).into())
                    {
                        lines.push((start + range.start, shape(&text[range], theme::TEXT)));
                        backgrounds.push(theme::PANEL);
                    }
                } else {
                    lines.push((start, line));
                    backgrounds.push(theme::PANEL);
                }
                start += text.len() + 1;
            }
        }
        let width = if wrap {
            (f32::from(self.viewport.width) - 12.).max(0.)
        } else if diff_coloring {
            let cell = f32::from(shape("0", theme::TEXT).width);
            self.diff_max_columns as f32 * cell + 12.
        } else {
            lines
                .iter()
                .map(|(_, line)| f32::from(line.width))
                .fold(0., f32::max)
                + 8.
        };
        let height = if diff_coloring {
            self.diff_line_starts.len()
        } else {
            lines.len()
        } as f32
            * 22.;
        let visible = if self.single_line {
            crate::scrolling::Visibility::default()
        } else {
            crate::scrolling::Visibility::new(
                self.viewport.width.into(),
                self.viewport.height.into(),
                width,
                height,
                !self.read_only || self.diff_coloring,
                12.,
            )
        };
        if self.scroll_drag.is_some_and(|(horizontal, _)| {
            if horizontal {
                !visible.horizontal
            } else {
                !visible.vertical
            }
        }) {
            self.scroll_drag = None;
        }
        if wrap && self.scroll.offset().x != px(0.) {
            self.scroll
                .set_offset(point(px(0.), self.scroll.offset().y));
        }
        let selection = self.buffer.selection();
        let cursor = self.buffer.cursor;
        let marked = self.buffer.marked.clone();
        let first_visible_row = row_range.start;
        let gutter_edges = if diff_coloring {
            let measure = shape("0000000000000000", theme::TEXT);
            Some((measure.x_for_index(13), measure.x_for_index(16)))
        } else {
            None
        };
        let view = cx.entity();
        let focus = self.focus.clone();
        let editor = div()
            .id("commit-message-editor")
            .track_focus(&self.focus)
            .h_full()
            .flex_1()
            .min_w_0()
            .overflow_scroll()
            .when(wrap, |editor| {
                editor.overflow_x_hidden().overflow_y_scroll()
            })
            .track_scroll(&self.scroll)
            .bg(rgb(self.background_color()))
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if !this.enabled {
                        return;
                    }
                    this.focus.focus(window);
                    this.scroll_drag = None;
                    let index = this.index_at(event.position);
                    this.buffer.move_to(index, event.modifiers.shift);
                    this.dragging = true;
                    this.changed(cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let single_line = view.read(cx).single_line;
                        let offset =
                            px(text_vertical_offset(single_line, bounds.size.height.into()));
                        let text_bounds = Bounds::new(
                            bounds.origin + point(px(0.), offset),
                            size(bounds.size.width, bounds.size.height - offset),
                        );
                        let move_view = view.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture {
                                return;
                            }
                            move_view.update(cx, |this, cx| {
                                if !this.dragging {
                                    return;
                                }
                                if !this.enabled
                                    || !this.focus.is_focused(window)
                                    || event.pressed_button != Some(MouseButton::Left)
                                {
                                    this.dragging = false;
                                    return;
                                }
                                let index = this.index_at(event.position);
                                this.buffer.move_to(index, true);
                                this.changed(cx);
                            });
                        });
                        let up_view = view.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                up_view.update(cx, |this, _| this.dragging = false);
                            }
                        });
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(text_bounds, view.clone()),
                            cx,
                        );
                        if let Some((numbers_end, marker_end)) = gutter_edges {
                            window.paint_quad(fill(
                                Bounds::new(
                                    text_bounds.origin,
                                    size(numbers_end, text_bounds.size.height),
                                ),
                                rgb(theme::HISTORY_HOVER),
                            ));
                            for edge in [numbers_end, marker_end] {
                                window.paint_quad(fill(
                                    Bounds::new(
                                        text_bounds.origin + point(edge, px(0.)),
                                        size(px(1.), text_bounds.size.height),
                                    ),
                                    rgb(theme::LINE),
                                ));
                            }
                        }
                        for (row, (start, line)) in lines.iter().enumerate() {
                            let absolute_row = if diff_coloring {
                                first_visible_row + row
                            } else {
                                row
                            };
                            let origin =
                                text_bounds.origin + point(px(0.), px(absolute_row as f32 * 22.));
                            if let Some(background) = backgrounds.get(row)
                                && *background != theme::PANEL
                            {
                                window.paint_quad(fill(
                                    Bounds::new(origin, size(text_bounds.size.width, px(22.))),
                                    rgb(*background),
                                ));
                            }
                            if diff_coloring {
                                let numbers_end = line.x_for_index(13.min(line.len()));
                                let marker_end = line.x_for_index(16.min(line.len()));
                                window.paint_quad(fill(
                                    Bounds::new(origin, size(numbers_end, px(22.))),
                                    rgb(theme::HISTORY_HOVER),
                                ));
                                window.paint_quad(fill(
                                    Bounds::new(
                                        origin + point(numbers_end, px(0.)),
                                        size(px(1.), px(22.)),
                                    ),
                                    rgb(theme::LINE),
                                ));
                                window.paint_quad(fill(
                                    Bounds::new(
                                        origin + point(marker_end, px(0.)),
                                        size(px(1.), px(22.)),
                                    ),
                                    rgb(theme::LINE),
                                ));
                            }
                            let end = start + line.len();
                            let body_start = if diff_coloring {
                                diff_body_start(0, line.len())
                            } else {
                                0
                            };
                            if selection.start <= end && selection.end > start + body_start {
                                let left = line.x_for_index(
                                    selection
                                        .start
                                        .saturating_sub(*start)
                                        .clamp(body_start, line.len()),
                                );
                                let right = line.x_for_index(
                                    selection.end.saturating_sub(*start).min(line.len()),
                                ) + if selection.end > end { px(7.) } else { px(0.) };
                                window.paint_quad(fill(
                                    Bounds::new(
                                        origin + point(left, px(0.)),
                                        size((right - left).max(px(0.)), px(22.)),
                                    ),
                                    rgb(theme::TEXT_SELECTION),
                                ));
                            }
                            line.paint(origin, px(22.), window, cx).ok();
                            if cursor >= start + body_start
                                && cursor <= end
                                && focus.is_focused(window)
                            {
                                window.paint_quad(fill(
                                    Bounds::new(
                                        origin + point(line.x_for_index(cursor - start), px(1.)),
                                        size(px(1.), px(20.)),
                                    ),
                                    rgb(theme::ACCENT),
                                ));
                            }
                            if let Some(marked) = &marked
                                && marked.start <= end
                                && marked.end > *start
                            {
                                let left = line.x_for_index(
                                    marked.start.saturating_sub(*start).min(line.len()),
                                );
                                let right = line
                                    .x_for_index(marked.end.saturating_sub(*start).min(line.len()));
                                window.paint_quad(fill(
                                    Bounds::new(
                                        origin + point(left, px(21.)),
                                        size(right - left, px(1.)),
                                    ),
                                    rgb(theme::ACCENT),
                                ));
                            }
                        }
                        view.update(cx, |this, cx| {
                            this.bounds = Some(text_bounds);
                            this.lines = lines.clone();
                            this.visible_row_start = first_visible_row;
                            if this.reveal_cursor {
                                this.reveal_cursor = false;
                                let cursor_row = if diff_coloring {
                                    this.diff_line_starts
                                        .partition_point(|start| *start <= cursor)
                                        .saturating_sub(1)
                                } else {
                                    lines
                                        .iter()
                                        .rposition(|(start, _)| *start <= cursor)
                                        .unwrap_or(0)
                                };
                                if let Some((start, line)) =
                                    lines.get(cursor_row.saturating_sub(first_visible_row))
                                {
                                    let caret = point(
                                        line.x_for_index(
                                            cursor.saturating_sub(*start).min(line.len()),
                                        ),
                                        px(cursor_row as f32 * 22.),
                                    );
                                    let viewport = this.scroll.bounds().size;
                                    let before = this.scroll.offset();
                                    let mut offset = before;
                                    if (!this.read_only || this.diff_coloring)
                                        && viewport.width > px(0.)
                                    {
                                        if caret.x < -offset.x {
                                            offset.x = -caret.x;
                                        } else if caret.x + px(3.) > -offset.x + viewport.width {
                                            offset.x = -(caret.x + px(3.) - viewport.width);
                                        }
                                    }
                                    if viewport.height > px(0.) {
                                        if caret.y < -offset.y {
                                            offset.y = -caret.y;
                                        } else if caret.y + px(22.) > -offset.y + viewport.height {
                                            offset.y = -(caret.y + px(22.) - viewport.height);
                                        }
                                    }
                                    if offset != before {
                                        this.scroll.set_offset(offset);
                                        cx.notify();
                                    }
                                } else if diff_coloring {
                                    let mut offset = this.scroll.offset();
                                    offset.y = px(-(cursor_row as f32 * 22.));
                                    this.scroll.set_offset(offset);
                                    cx.notify();
                                }
                            }
                        });
                    },
                )
                .w(px(width))
                .h(px(height))
                .min_w_full()
                .min_h_full(),
            );
        let measure = cx.weak_entity();
        div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .h(px(if self.single_line { 26. } else { 220. }))
            .when(self.read_only, |editor| editor.h_full())
            .flex_shrink_0()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        measure
                            .update(cx, |this, cx| {
                                if this.viewport != bounds.size {
                                    this.viewport = bounds.size;
                                    cx.notify();
                                }
                            })
                            .ok();
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(editor)
                    .when(visible.vertical, |row| row.child(self.scrollbar(false, cx))),
            )
            .when(visible.horizontal, |column| {
                column.child(
                    div()
                        .w_full()
                        .flex_shrink_0()
                        .pr(px(if visible.vertical { 12. } else { 0. }))
                        .child(self.scrollbar(true, cx)),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn diff_virtual_rows_and_body_boundary_cover_scrolling_and_short_lines() {
        assert_eq!(line_starts("one\ntwo\n"), vec![0, 4, 8]);
        assert_eq!(visible_rows(0., 220., 10_000), 0..13);
        assert_eq!(visible_rows(-2200., 220., 10_000), 98..113);
        assert_eq!(diff_body_start(100, 40), 116);
        assert_eq!(diff_body_start(100, 3), 103);
    }

    #[::core::prelude::v1::test]
    fn password_mask_preserves_byte_offsets_and_mouse_indexes_stay_on_source_boundaries() {
        for text in ["secret", "한글🙂password", ""] {
            let mask = masked_text(text);
            assert_eq!(mask.len(), text.len());
            assert!(mask.chars().all(|c| c == '*'));
            for index in 0..=mask.len() {
                assert!(text.is_char_boundary(source_boundary(text, index)));
            }
        }
    }

    #[::core::prelude::v1::test]
    fn wrapping_preserves_unicode_source_offsets_empty_lines_and_long_words() {
        for text in ["", "long/path/without/spaces", "한글🙂 오류  text", "a\t b"] {
            for width in [0., 1., 3., 80.] {
                let rows =
                    wrapped_ranges(text, width, |index| text[..index].chars().count() as f32);
                assert!(!rows.is_empty());
                assert_eq!(
                    rows.iter()
                        .map(|range| &text[range.clone()])
                        .collect::<String>(),
                    text
                );
                for (index, range) in rows.iter().enumerate() {
                    assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
                    assert!(text[range.clone()].chars().count() <= width.max(1.) as usize);
                    if index > 0 {
                        assert_eq!(rows[index - 1].end, range.start);
                    }
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn read_only_mode_accepts_selection_and_copy_but_rejects_editing_keys() {
        for key in [
            "ctrl-c",
            "ctrl-a",
            "left",
            "shift-right",
            "shift-down",
            "home",
            "end",
        ] {
            assert!(read_only_key(&Keystroke::parse(key).unwrap()), "{key}");
        }
        for key in [
            "ctrl-v",
            "ctrl-x",
            "backspace",
            "delete",
            "enter",
            "tab",
            "a",
            "alt-v",
        ] {
            assert!(!read_only_key(&Keystroke::parse(key).unwrap()), "{key}");
        }
    }
    #[::core::prelude::v1::test]
    fn single_line_text_is_centered_without_moving_multiline_text() {
        assert_eq!(text_vertical_offset(true, 26.), 2.);
        assert_eq!(text_vertical_offset(true, 22.), 0.);
        assert_eq!(text_vertical_offset(false, 220.), 0.);
    }

    #[::core::prelude::v1::test]
    fn settings_inputs_keep_unicode_on_one_line_and_commit_messages_keep_newlines() {
        assert_eq!(normalized_input("한글 😀\r\n이름\t", true), "한글 😀이름");
        assert_eq!(normalized_input("제목\r\n\r\n본문", false), "제목\n\n본문");
    }
    #[::core::prelude::v1::test]
    fn ime_composition_replaces_marked_text_and_preserves_utf16_selection() {
        let mut buffer = Buffer::default();
        buffer.replace(None, "제목 😀\n", false, None);
        buffer.replace(None, "ㅎ", true, Some(1..1));
        buffer.replace(None, "한", true, Some(1..1));
        buffer.replace(None, "한글", false, None);
        assert_eq!(buffer.text, "제목 😀\n한글");
        assert!(buffer.marked.is_none());
        for (index, _) in buffer.text.char_indices() {
            assert_eq!(buffer.offset_from_utf16(buffer.to_utf16(index)), index);
        }
        let range = buffer.utf16_range("제목 😀\n".len()..buffer.text.len());
        buffer.replace(Some(range), "본문\n\n둘째 줄", false, None);
        assert_eq!(buffer.text, "제목 😀\n본문\n\n둘째 줄");
        buffer.anchor = 0;
        buffer.cursor = buffer.text.len();
        buffer.replace(None, "", false, None);
        assert!(buffer.text.is_empty());
    }
}
