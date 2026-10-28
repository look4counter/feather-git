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
    pub focus: FocusHandle,
    pub enabled: bool,
    pub text_size: f32,
    scroll: ScrollHandle,
    bounds: Option<Bounds<Pixels>>,
    lines: Vec<(usize, ShapedLine)>,
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
            focus: cx.focus_handle(),
            enabled: true,
            text_size: 14.,
            scroll: ScrollHandle::new(),
            bounds: None,
            lines: Vec::new(),
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
    pub fn single_line(text: &str, cx: &mut Context<Self>) -> Self {
        let mut editor = Self::new(cx);
        editor.single_line = true;
        editor.set_text(text, cx);
        editor
    }
    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.buffer = Buffer {
            text: self.input_text(text),
            ..Buffer::default()
        };
        self.changed(cx);
    }
    fn input_text(&self, text: &str) -> String {
        normalized_input(text, self.single_line)
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
        let Some((start, line)) = self.lines.get(row.min(self.lines.len().saturating_sub(1)))
        else {
            return 0;
        };
        start + line.closest_index_for_x((position.x - bounds.origin.x).max(px(0.)))
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            cx.stop_propagation();
            return;
        }
        let key = &event.keystroke;
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
                    if !range.is_empty() {
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
                        _ => cursor,
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
            .bg(rgb(theme::INPUT))
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

impl EntityInputHandler for CommitEditor {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
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
        if self.enabled {
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
        if self.enabled {
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
        Some(Bounds::new(
            bounds.origin + point(line.x_for_index(range.start - start), px(row as f32 * 22.)),
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
        let mut start = 0;
        let lines: Vec<_> = self
            .buffer
            .text
            .split('\n')
            .map(|text| {
                let run = TextRun {
                    len: text.len(),
                    font: font("Cascadia Mono"),
                    color: rgb(theme::TEXT).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let line = window.text_system().shape_line(
                    text.to_owned().into(),
                    px(self.text_size),
                    &[run],
                    None,
                );
                let current = start;
                start += text.len() + 1;
                (current, line)
            })
            .collect();
        let width = lines
            .iter()
            .map(|(_, line)| f32::from(line.width))
            .fold(0., f32::max)
            + 8.;
        let height = lines.len() as f32 * 22.;
        let visible = if self.single_line {
            crate::scrolling::Visibility::default()
        } else {
            crate::scrolling::Visibility::new(
                self.viewport.width.into(),
                self.viewport.height.into(),
                width,
                height,
                true,
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
        let selection = self.buffer.selection();
        let cursor = self.buffer.cursor;
        let marked = self.buffer.marked.clone();
        let view = cx.entity();
        let focus = self.focus.clone();
        let editor = div()
            .id("commit-message-editor")
            .track_focus(&self.focus)
            .h_full()
            .flex_1()
            .min_w_0()
            .overflow_scroll()
            .track_scroll(&self.scroll)
            .bg(rgb(theme::INPUT))
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
                        for (row, (start, line)) in lines.iter().enumerate() {
                            let origin = text_bounds.origin + point(px(0.), px(row as f32 * 22.));
                            let end = start + line.len();
                            if selection.start <= end && selection.end > *start {
                                let left = line.x_for_index(
                                    selection.start.saturating_sub(*start).min(line.len()),
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
                            if cursor >= *start && cursor <= end && focus.is_focused(window) {
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
                            if this.reveal_cursor {
                                this.reveal_cursor = false;
                                if let Some(row) =
                                    lines.iter().rposition(|(start, _)| *start <= cursor)
                                {
                                    let (start, line) = &lines[row];
                                    let caret = point(
                                        line.x_for_index(cursor - start),
                                        px(row as f32 * 22.),
                                    );
                                    let viewport = this.scroll.bounds().size;
                                    let before = this.scroll.offset();
                                    let mut offset = before;
                                    if viewport.width > px(0.) {
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
