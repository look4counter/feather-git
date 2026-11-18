// Adapted from the GPUI 0.2.2 single-line input example.
use crate::theme::*;
use gpui::{prelude::*, *};
use std::ops::Range;

actions!(
    repository_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectHome,
        SelectEnd,
        SelectAll,
        Home,
        End,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Newline,
        Up,
        Down,
        SelectUp,
        SelectDown,
        DocumentHome,
        DocumentEnd,
    ]
);

pub struct TextInput {
    pub focus_handle: FocusHandle,
    pub enabled: bool,
    focus_subscriptions: Vec<Subscription>,
    read_only: bool,
    secret: bool,
    multiline: bool,
    compact: bool,
    text_area_scroll: ScrollHandle,
    last_lines: Vec<(usize, ShapedLine)>,
    reveal_cursor: bool,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    scroll_offset: Pixels,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            enabled: true,
            focus_subscriptions: Vec::new(),
            read_only: false,
            secret: false,
            multiline: false,
            compact: false,
            text_area_scroll: ScrollHandle::new(),
            last_lines: Vec::new(),
            reveal_cursor: false,
            content: "".into(),
            placeholder: "".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            scroll_offset: px(0.),
        }
    }
    pub fn compact(cx: &mut Context<Self>) -> Self {
        Self { compact: true, ..Self::new(cx) }
    }
    pub fn single_line(value: &str, cx: &mut Context<Self>) -> Self {
        let mut input = Self::new(cx);
        input.set_text(value, cx);
        input
    }
    pub fn text_area(cx: &mut Context<Self>) -> Self {
        let mut input = Self::new(cx);
        input.multiline = true;
        input
    }
    pub fn password(cx: &mut Context<Self>) -> Self {
        let mut input = Self::new(cx);
        input.secret = true;
        input
    }
    pub fn message(&self) -> &str {
        &self.content
    }
    pub fn background_color(&self) -> u32 {
        if self.read_only { PANEL } else { BASE }
    }
    pub fn set_read_only(&mut self, value: bool, cx: &mut Context<Self>) {
        self.read_only = value;
        cx.notify();
    }
    pub fn take_secret(&mut self, cx: &mut Context<Self>) -> crate::auth_profiles::Secret {
        let value = crate::auth_profiles::Secret::new(self.content.to_string());
        self.set_text("", cx);
        self.last_layout = None;
        self.last_bounds = None;
        value
    }
    fn editable(&self) -> bool {
        self.enabled && !self.read_only
    }
    fn display_text(&self) -> SharedString {
        if self.secret {
            "•".repeat(self.content.chars().count()).into()
        } else {
            self.content.clone()
        }
    }
    fn display_offset(&self, index: usize) -> usize {
        display_offset(&self.content, index, self.secret)
    }
    fn source_offset(&self, index: usize) -> usize {
        source_offset(&self.content, index, self.secret)
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.reveal_cursor = self.multiline;
        if !self.secret {
            cx.emit(Changed(self.text()));
        }
    }
    pub fn is_composing(&self) -> bool {
        self.marked_range.is_some()
    }
    pub fn text(&self) -> String {
        self.content.to_string()
    }
    pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
        self.content = normalized_input(value, self.multiline).into();
        self.selected_range = self.content.len()..self.content.len();
        self.marked_range = None;
        self.scroll_offset = px(0.);
        self.changed(cx);
        cx.notify();
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        clear_input_selection(
            &mut self.selected_range,
            &mut self.selection_reversed,
            &mut self.is_selecting,
        );
        cx.notify();
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        let offset = line_edge(&self.content, self.cursor_offset(), self.multiline, false);
        self.move_to(offset, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        let offset = line_edge(&self.content, self.cursor_offset(), self.multiline, true);
        self.move_to(offset, cx);
    }

    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        let offset = line_edge(&self.content, self.cursor_offset(), self.multiline, false);
        self.select_to(offset, cx);
    }
    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        let offset = line_edge(&self.content, self.cursor_offset(), self.multiline, true);
        self.select_to(offset, cx);
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline && self.editable() && !self.is_composing() {
            self.replace_text_in_range(None, "\n", window, cx);
        }
    }
    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if self.enabled {
            self.move_to(
                vertical_offset(&self.content, self.cursor_offset(), false),
                cx,
            );
        }
    }
    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if self.enabled {
            self.move_to(
                vertical_offset(&self.content, self.cursor_offset(), true),
                cx,
            );
        }
    }
    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        if self.enabled {
            self.select_to(
                vertical_offset(&self.content, self.cursor_offset(), false),
                cx,
            );
        }
    }
    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        if self.enabled {
            self.select_to(
                vertical_offset(&self.content, self.cursor_offset(), true),
                cx,
            );
        }
    }
    fn document_home(&mut self, _: &DocumentHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }
    fn document_end(&mut self, _: &DocumentEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.enabled {
            return;
        }
        self.focus_handle.focus(window);
        if event.click_count == 2 && self.marked_range.is_none() {
            let offset = self.index_for_position(event.position, true);
            self.selected_range = word_selection_range(&self.content, offset, self.secret);
            self.selection_reversed = false;
            self.is_selecting = false;
            cx.notify();
            return;
        }
        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        if self.editable() {
            window.show_character_palette();
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text
            } else {
                text.replace("\n", " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled || self.secret {
            return;
        }
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }
    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled || self.secret {
            return;
        }
        if !self.editable() {
            return;
        }
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        let offset = utf8_boundary(&self.content, offset);
        self.selected_range = offset..offset;
        self.reveal_cursor = self.multiline;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        self.index_for_position(position, false)
    }

    fn index_for_position(&self, position: Point<Pixels>, character_hit: bool) -> usize {
        if self.multiline {
            return self.multiline_index(position, character_hit).unwrap_or(0);
        }
        if self.content.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        let x = position.x - bounds.left() + self.scroll_offset;
        self.source_offset(if character_hit {
            line.index_for_x(x).unwrap_or(line.text.len())
        } else {
            line.closest_index_for_x(x)
        })
    }

    fn multiline_index(&self, position: Point<Pixels>, character_hit: bool) -> Option<usize> {
        let bounds = self.last_bounds?;
        let row = ((f32::from(position.y - bounds.top()) / 24.).max(0.) as usize)
            .min(self.last_lines.len().saturating_sub(1));
        let (start, line) = self.last_lines.get(row)?;
        let x = position.x - bounds.left();
        Some(
            (start
                + if character_hit {
                    line.index_for_x(x).unwrap_or(line.text.len())
                } else {
                    line.closest_index_for_x(x)
                })
            .min(self.content.len()),
        )
    }
    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        extend_selection(
            &self.content,
            &mut self.selected_range,
            &mut self.selection_reversed,
            offset,
        );
        self.reveal_cursor = self.multiline;
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        utf16_offset(&self.content, offset)
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .char_indices()
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .char_indices()
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.secret || !self.enabled {
            return None;
        }
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if !self.enabled {
            return None;
        }
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let new_text = normalized_input(new_text, self.multiline);
        replace_buffer(
            &mut self.content,
            &mut self.selected_range,
            &mut self.marked_range,
            range_utf16,
            &new_text,
            None,
            false,
        );
        self.selection_reversed = false;
        self.changed(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let new_text = normalized_input(new_text, self.multiline);
        replace_buffer(
            &mut self.content,
            &mut self.selected_range,
            &mut self.marked_range,
            range_utf16,
            &new_text,
            new_selected_range_utf16,
            true,
        );
        self.selection_reversed = false;
        self.changed(cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.multiline {
            let range = self.range_from_utf16(&range_utf16);
            let row = display_row(&self.last_lines, range.start);
            let (start, line) = self.last_lines.get(row)?;
            let left = bounds.left()
                + line.x_for_index(range.start.saturating_sub(*start).min(line.text.len()));
            let right = bounds.left()
                + line.x_for_index(range.end.saturating_sub(*start).min(line.text.len()));
            return Some(Bounds::new(
                point(left, bounds.top() + px(row as f32 * 24.)),
                size((right - left).max(px(1.)), px(24.)),
            ));
        }
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(self.display_offset(range.start))
                    - self.scroll_offset,
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(self.display_offset(range.end))
                    - self.scroll_offset,
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        self.last_bounds?.localize(&point)?;
        if self.multiline {
            return self
                .multiline_index(point, true)
                .map(|offset| self.offset_to_utf16(offset));
        }
        let last_layout = self.last_layout.as_ref()?;

        if last_layout.text != self.display_text() {
            return None;
        }
        let utf8_index =
            last_layout.index_for_x(point.x - self.last_bounds?.origin.x + self.scroll_offset)?;
        Some(self.offset_to_utf16(self.source_offset(utf8_index)))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    scroll_offset: Pixels,
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.display_text();
        let selected_range = input.display_offset(input.selected_range.start)
            ..input.display_offset(input.selected_range.end);
        let cursor = input.display_offset(input.cursor_offset());
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), rgb(MUTED).into())
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(source_range) = input.marked_range.as_ref() {
            let marked_range =
                input.display_offset(source_range.start)..input.display_offset(source_range.end);
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let scroll_offset = (line.x_for_index(cursor) - bounds.size.width + px(2.)).max(px(0.));
        let cursor_pos = line.x_for_index(cursor) - scroll_offset;
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    rgb(ACCENT),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start) - scroll_offset,
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end) - scroll_offset,
                            bounds.bottom(),
                        ),
                    ),
                    rgb(SELECTED),
                )),
                None,
            )
        };
        PrepaintState {
            scroll_offset,
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin - point(prepaint.scroll_offset, px(0.)),
            window.line_height(),
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.scroll_offset = prepaint.scroll_offset;
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions = vec![
                cx.on_blur(&self.focus_handle, window, |this, _, cx| {
                    this.clear_selection(cx)
                }),
                cx.observe_window_activation(window, |this, window, cx| {
                    if !window.is_window_active() {
                        this.clear_selection(cx);
                    }
                }),
            ];
        }
        div()
            .id("text-input")
            .flex()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .key_context(if self.multiline {
                "MultilineInput"
            } else {
                "RepositoryInput"
            })
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::document_home))
            .on_action(cx.listener(Self::document_end))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .bg(rgb(self.background_color()))
            .line_height(px(if self.compact { 20. } else { 24. }))
            .text_size(px(if self.compact { 12. } else { 13. }))
            .text_color(rgb(TEXT))
            .when(self.multiline, |input| {
                input
                    .h(px(178.))
                    .flex_col()
                    .overflow_scroll()
                    .track_scroll(&self.text_area_scroll)
            })
            .child(if self.multiline {
                div()
                    .flex_shrink_0()
                    .p(px(4.))
                    .child(TextAreaElement { input: cx.entity() })
                    .into_any_element()
            } else {
                div()
                    .h(px(if self.compact { 28. } else { 32. }))
                    .w_full()
                    .p(px(4.))
                    .bg(rgb(self.background_color()))
                    .child(TextElement { input: cx.entity() })
                    .into_any_element()
            })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
pub(crate) fn apply_edit(
    content: &mut SharedString,
    selection: &mut Range<usize>,
    marked: &mut Option<Range<usize>>,
    range_utf16: Option<Range<usize>>,
    text: &str,
    selected_utf16: Option<Range<usize>>,
    composing: bool,
) {
    let text = single_line(text);
    replace_buffer(
        content,
        selection,
        marked,
        range_utf16,
        &text,
        selected_utf16,
        composing,
    );
}

fn replace_buffer(
    content: &mut SharedString,
    selection: &mut Range<usize>,
    marked: &mut Option<Range<usize>>,
    range_utf16: Option<Range<usize>>,
    text: &str,
    selected_utf16: Option<Range<usize>>,
    composing: bool,
) {
    let range = range_utf16
        .map(|r| utf16_offset(content, r.start)..utf16_offset(content, r.end))
        .or_else(|| marked.clone())
        .unwrap_or_else(|| selection.clone());
    *content = (content[..range.start].to_owned() + text + &content[range.end..]).into();
    *marked = (composing && !text.is_empty()).then_some(range.start..range.start + text.len());
    *selection = selected_utf16
        .map(|r| {
            (range.start + utf16_offset(text, r.start))..(range.start + utf16_offset(text, r.end))
        })
        .unwrap_or(range.start + text.len()..range.start + text.len());
}

fn utf16_offset(text: &str, offset: usize) -> usize {
    let mut count = 0;
    for (index, character) in text.char_indices() {
        if count >= offset {
            return index;
        }
        count += character.len_utf16();
    }
    text.len()
}

fn line_edge(text: &str, cursor: usize, multiline: bool, end: bool) -> usize {
    let cursor = utf8_boundary(text, cursor);
    let bounds = if multiline {
        line_bounds(text, cursor)
    } else {
        0..text.len()
    };
    if end { bounds.end } else { bounds.start }
}
fn extend_selection(text: &str, selected: &mut Range<usize>, reversed: &mut bool, offset: usize) {
    let offset = utf8_boundary(text, offset);
    if *reversed {
        selected.start = offset;
    } else {
        selected.end = offset;
    }
    if selected.end < selected.start {
        *reversed = !*reversed;
        *selected = selected.end..selected.start;
    }
}

fn single_line(value: &str) -> String {
    value.replace(['\r', '\n', '\t'], "")
}
pub struct Changed(pub String);
impl EventEmitter<Changed> for TextInput {}

pub fn init(cx: &mut App) {
    for context in ["RepositoryInput", "MultilineInput"] {
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, Some(context)),
            KeyBinding::new("delete", Delete, Some(context)),
            KeyBinding::new("left", Left, Some(context)),
            KeyBinding::new("right", Right, Some(context)),
            KeyBinding::new("shift-left", SelectLeft, Some(context)),
            KeyBinding::new("shift-right", SelectRight, Some(context)),
            KeyBinding::new("ctrl-a", SelectAll, Some(context)),
            KeyBinding::new("ctrl-v", Paste, Some(context)),
            KeyBinding::new("ctrl-c", Copy, Some(context)),
            KeyBinding::new("ctrl-x", Cut, Some(context)),
            KeyBinding::new("home", Home, Some(context)),
            KeyBinding::new("end", End, Some(context)),
            KeyBinding::new("shift-home", SelectHome, Some(context)),
            KeyBinding::new("shift-end", SelectEnd, Some(context)),
        ]);
    }
    cx.bind_keys([
        KeyBinding::new("enter", Newline, Some("MultilineInput")),
        KeyBinding::new("up", Up, Some("MultilineInput")),
        KeyBinding::new("down", Down, Some("MultilineInput")),
        KeyBinding::new("shift-up", SelectUp, Some("MultilineInput")),
        KeyBinding::new("shift-down", SelectDown, Some("MultilineInput")),
        KeyBinding::new("ctrl-home", DocumentHome, Some("MultilineInput")),
        KeyBinding::new("ctrl-end", DocumentEnd, Some("MultilineInput")),
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn utf16_selection_handles_korean_surrogate_pairs_and_bounds() {
        let text = "a한😀z";
        for (offset, expected) in [(0, 0), (1, 1), (2, 4), (3, 8), (4, 8), (5, 9), (100, 9)] {
            let index = utf16_offset(text, offset);
            assert_eq!(index, expected);
            assert!(text.is_char_boundary(index));
        }
    }

    #[::core::prelude::v1::test]
    fn shift_line_edges_keep_anchor_through_unicode_and_selection_direction_changes() {
        let text = "한글😀abc";
        let anchor = "한글".len();
        let mut selected = anchor..anchor;
        let mut reversed = false;
        extend_selection(
            text,
            &mut selected,
            &mut reversed,
            line_edge(text, anchor, false, false),
        );
        assert_eq!(&text[selected.clone()], "한글");
        assert!(reversed);
        let offset = line_edge(text, selected.start, false, true);
        extend_selection(text, &mut selected, &mut reversed, offset);
        assert_eq!(&text[selected.clone()], "😀abc");
        assert!(!reversed);
        extend_selection(text, &mut selected, &mut reversed, 0);
        assert_eq!(selected, 0..anchor);
        assert!(reversed);
        let multiline = "first\n한글😀끝\nlast";
        let cursor = "first\n한글😀".len();
        let mut selected = cursor..cursor;
        let mut reversed = false;
        extend_selection(
            multiline,
            &mut selected,
            &mut reversed,
            line_edge(multiline, cursor, true, false),
        );
        assert_eq!(&multiline[selected.clone()], "한글😀");
        let offset = line_edge(multiline, selected.start, true, true);
        extend_selection(multiline, &mut selected, &mut reversed, offset);
        assert_eq!(&multiline[selected.clone()], "끝");
        assert!(!reversed);
        for text in ["", "\n", "한\n\n끝\n"] {
            for cursor in text
                .char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(text.len()))
            {
                for end in [false, true] {
                    let offset = line_edge(text, cursor, true, end);
                    assert!(text.is_char_boundary(offset));
                    let mut selected = cursor..cursor;
                    let mut reversed = false;
                    extend_selection(text, &mut selected, &mut reversed, offset);
                    assert!(
                        text.is_char_boundary(selected.start)
                            && text.is_char_boundary(selected.end)
                    );
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn single_line_preserves_unicode_and_strips_controls() {
        assert_eq!(single_line("D:\\한글\n\t😀\r"), "D:\\한글😀");
    }
}

fn display_offset(text: &str, index: usize, secret: bool) -> usize {
    if secret {
        text[..index].chars().count() * "•".len()
    } else {
        index
    }
}
fn source_offset(text: &str, index: usize, secret: bool) -> usize {
    if secret {
        text.char_indices()
            .nth(index / "•".len())
            .map_or(text.len(), |(i, _)| i)
    } else {
        index
    }
}
#[cfg(test)]
mod secret_tests {
    #[test]
    fn unicode_mask_positions_round_trip() {
        let text = "a한😀z";
        for i in text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
        {
            assert_eq!(
                super::source_offset(text, super::display_offset(text, i, true), true),
                i
            );
            assert_eq!(super::display_offset(text, i, false), i);
        }
        assert_eq!(super::display_offset(text, text.len(), true), 12);
    }
}

fn word_selection_range(text: &str, byte: usize, secret: bool) -> Range<usize> {
    if secret {
        return 0..text.len();
    }
    let byte = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .take_while(|index| *index <= byte)
        .last()
        .unwrap_or(0);
    let column = text[..byte].chars().count();
    let mut selection = crate::text_selection::Selection::default();
    selection.select_word(text, crate::text_selection::Position { row: 0, column });
    let offset = |column| {
        text.char_indices()
            .nth(column)
            .map_or(text.len(), |(index, _)| index)
    };
    offset(selection.anchor.column)..offset(selection.cursor.column)
}

#[cfg(test)]
mod word_selection_tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn double_click_selects_unicode_words_separators_and_exact_boundaries() {
        let text = "한글 foo_bar42, 😀 끝";
        for (needle, expected) in [
            ("한", "한글"),
            ("글", "한글"),
            ("foo", "foo_bar42"),
            ("42", "foo_bar42"),
            (",", ","),
            ("😀", "😀"),
            ("끝", "끝"),
        ] {
            let range = word_selection_range(text, text.find(needle).unwrap(), false);
            assert_eq!(&text[range], expected);
        }
        assert_eq!(&text[word_selection_range(text, 1, false)], "한글");
        assert_eq!(&"a  b"[word_selection_range("a  b", 1, false)], "  ");
        assert_eq!(
            word_selection_range(text, text.len(), false),
            text.len()..text.len()
        );
        assert_eq!(
            word_selection_range(text, usize::MAX, false),
            text.len()..text.len()
        );
        assert_eq!(word_selection_range("", 0, false), 0..0);
    }
    #[::core::prelude::v1::test]
    fn selected_word_replacement_uses_utf8_range_and_preserves_other_text() {
        let mut text: SharedString = "앞 한글_word42 뒤".into();
        let mut selected = word_selection_range(&text, "앞 한".len(), false);
        assert_eq!(&text[selected.clone()], "한글_word42");
        let mut marked = None;
        apply_edit(
            &mut text,
            &mut selected,
            &mut marked,
            None,
            "새값😀",
            None,
            false,
        );
        assert_eq!(text.to_string(), "앞 새값😀 뒤");
        assert_eq!(selected, "앞 새값😀".len().."앞 새값😀".len());
    }
    #[::core::prelude::v1::test]
    fn password_double_click_selects_all_without_exposing_word_boundaries() {
        let text = "비밀 word😀";
        for index in text
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(text.len()))
        {
            let mask = display_offset(text, index, true);
            let source = source_offset(text, mask, true);
            assert_eq!(word_selection_range(text, source, true), 0..text.len());
        }
        assert_eq!(word_selection_range("", 0, true), 0..0);
    }
}

fn clear_input_selection(selection: &mut Range<usize>, reversed: &mut bool, selecting: &mut bool) {
    let caret = if *reversed {
        selection.start
    } else {
        selection.end
    };
    *selection = caret..caret;
    *reversed = false;
    *selecting = false;
}
#[cfg(test)]
mod blur_selection_tests {
    #[test]
    fn losing_focus_collapses_forward_reverse_and_empty_ranges_at_the_caret() {
        for (range, reversed, expected) in [
            (0..9, false, 9),
            (3..15, true, 3),
            (8..8, false, 8),
            (0..0, true, 0),
        ] {
            let mut selected = range;
            let mut reversed = reversed;
            let mut dragging = true;
            super::clear_input_selection(&mut selected, &mut reversed, &mut dragging);
            assert_eq!(selected, expected..expected);
            assert!(!reversed);
            assert!(!dragging);
        }
    }
}

fn normalized_input(text: &str, multiline: bool) -> String {
    if multiline {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        single_line(text)
    }
}
fn line_bounds(text: &str, offset: usize) -> Range<usize> {
    let start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index);
    start..end
}
fn vertical_offset(text: &str, offset: usize, down: bool) -> usize {
    let row = line_bounds(text, offset);
    let column = text[row.start..offset].chars().count();
    let target = if down {
        if row.end == text.len() {
            return offset;
        }
        line_bounds(text, row.end + 1)
    } else {
        if row.start == 0 {
            return offset;
        }
        line_bounds(text, row.start - 1)
    };
    text[target.clone()]
        .char_indices()
        .nth(column)
        .map_or(target.end, |(index, _)| target.start + index)
}
fn display_row(lines: &[(usize, ShapedLine)], offset: usize) -> usize {
    lines
        .partition_point(|(start, _)| *start <= offset)
        .saturating_sub(1)
}

struct TextAreaElement {
    input: Entity<TextInput>,
}
struct TextAreaPaint {
    lines: Vec<(usize, ShapedLine)>,
    selection: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
}
impl IntoElement for TextAreaElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for TextAreaElement {
    type RequestLayoutState = Vec<(usize, ShapedLine)>;
    type PrepaintState = TextAreaPaint;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let mut start = 0;
        let mut lines = Vec::new();
        let mut width = px(0.);
        for text in input.content.split('\n') {
            let run = TextRun {
                len: text.len(),
                font: style.font(),
                color: style.color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = if let Some(marked) = &input.marked_range {
                let from = marked.start.saturating_sub(start).min(text.len());
                let to = marked.end.saturating_sub(start).min(text.len());
                [
                    TextRun {
                        len: from,
                        ..run.clone()
                    },
                    TextRun {
                        len: to.saturating_sub(from),
                        underline: Some(UnderlineStyle {
                            color: Some(style.color),
                            thickness: px(1.),
                            wavy: false,
                        }),
                        ..run.clone()
                    },
                    TextRun {
                        len: text.len() - to,
                        ..run
                    },
                ]
                .into_iter()
                .filter(|run| run.len > 0)
                .collect::<Vec<_>>()
            } else {
                vec![run]
            };
            let line =
                window
                    .text_system()
                    .shape_line(text.to_owned().into(), font_size, &runs, None);
            width = width.max(line.width);
            lines.push((start, line));
            start += text.len() + 1;
        }
        let mut layout = Style::default();
        layout.size.width = relative(1.).into();
        layout.min_size.width = (width + px(2.)).into();
        layout.size.height = px(lines.len() as f32 * 24.).into();
        (window.request_layout(layout, [], cx), lines)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        lines: &mut Self::RequestLayoutState,
        _: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let mut selection = Vec::new();
        for (row, (start, line)) in lines.iter().enumerate() {
            let end = start + line.text.len();
            let selected = &input.selected_range;
            if !selected.is_empty() && selected.start <= end && selected.end > *start {
                let from = selected.start.saturating_sub(*start).min(line.text.len());
                let to = selected.end.saturating_sub(*start).min(line.text.len());
                let left = line.x_for_index(from);
                let right = line.x_for_index(to) + if selected.end > end { px(6.) } else { px(0.) };
                if right > left {
                    selection.push(fill(
                        Bounds::new(
                            bounds.origin + point(left, px(row as f32 * 24.)),
                            size(right - left, px(24.)),
                        ),
                        rgb(SELECTED),
                    ));
                }
            }
        }
        let row = display_row(lines, input.cursor_offset());
        let cursor = lines.get(row).map(|(start, line)| {
            fill(
                Bounds::new(
                    bounds.origin
                        + point(
                            line.x_for_index(
                                input
                                    .cursor_offset()
                                    .saturating_sub(*start)
                                    .min(line.text.len()),
                            ),
                            px(row as f32 * 24.),
                        ),
                    size(px(2.), px(24.)),
                ),
                rgb(ACCENT),
            )
        });
        TextAreaPaint {
            lines: lines.clone(),
            selection,
            cursor,
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        paint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let input = self.input.read(cx);
        let focus = input.focus_handle.clone();
        let reveal = input.reveal_cursor;
        let caret = input.cursor_offset();
        let scroll = input.text_area_scroll.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for selection in paint.selection.drain(..) {
            window.paint_quad(selection);
        }
        let viewport = window.content_mask().bounds;
        for (row, (_, line)) in paint.lines.iter().enumerate() {
            let origin = bounds.origin + point(px(0.), px(row as f32 * 24.));
            if origin.y + px(24.) >= viewport.top() && origin.y <= viewport.bottom() {
                line.paint(origin, px(24.), window, cx).unwrap();
            }
        }
        if focus.is_focused(window)
            && let Some(cursor) = paint.cursor.take()
        {
            window.paint_quad(cursor);
        }
        let mut new_offset = scroll.offset();
        let old_offset = new_offset;
        if reveal && let Some((start, line)) = paint.lines.get(display_row(&paint.lines, caret)) {
            let row = display_row(&paint.lines, caret);
            let x =
                bounds.left() + line.x_for_index(caret.saturating_sub(*start).min(line.text.len()));
            let y = bounds.top() + px(row as f32 * 24.);
            if x < viewport.left() {
                new_offset.x += viewport.left() - x;
            } else if x + px(4.) > viewport.right() {
                new_offset.x -= x + px(4.) - viewport.right();
            }
            if y < viewport.top() {
                new_offset.y += viewport.top() - y;
            } else if y + px(24.) > viewport.bottom() {
                new_offset.y -= y + px(24.) - viewport.bottom();
            }
        }
        self.input.update(cx, |input, cx| {
            input.last_bounds = Some(bounds);
            input.last_lines = paint.lines.clone();
            input.last_layout = None;
            input.reveal_cursor = false;
            if new_offset != old_offset {
                scroll.set_offset(new_offset);
                cx.notify();
            }
        });
    }
}

#[cfg(test)]
mod multiline_tests {
    use super::{
        line_bounds, normalized_input, replace_buffer, vertical_offset, word_selection_range,
    };
    #[test]
    fn multiline_preserves_line_breaks_unicode_and_single_line_contract() {
        assert_eq!(
            normalized_input("제목\r\n\r\n본문😀\r끝\t", true),
            "제목\n\n본문😀\n끝\t"
        );
        assert_eq!(normalized_input("제목\r\n본문\t", false), "제목본문");
    }
    #[test]
    fn multiline_ime_and_selected_replacement_preserve_newlines() {
        let mut content: gpui::SharedString = "제목\n\n본문😀".into();
        let mut selection = word_selection_range(&content, "제목\n\n본".len(), false);
        let mut marked = None;
        replace_buffer(
            &mut content,
            &mut selection,
            &mut marked,
            None,
            "새 본문\n다음",
            None,
            false,
        );
        assert_eq!(content.to_string(), "제목\n\n새 본문\n다음😀");
        replace_buffer(
            &mut content,
            &mut selection,
            &mut marked,
            None,
            "하",
            Some(1..1),
            true,
        );
        assert!(marked.is_some());
        replace_buffer(
            &mut content,
            &mut selection,
            &mut marked,
            None,
            "한",
            None,
            false,
        );
        assert!(marked.is_none());
        assert!(content.contains("다음한😀"));
    }
    #[test]
    fn vertical_movement_and_line_edges_use_character_boundaries() {
        let text = "한😀글\na\n끝줄";
        assert_eq!(line_bounds(text, "한😀".len()), 0.."한😀글".len());
        let down = vertical_offset(text, "한😀".len(), true);
        assert_eq!(down, "한😀글\na".len());
        let next = vertical_offset(text, down, true);
        assert_eq!(next, "한😀글\na\n끝".len());
        let up = vertical_offset(text, next, false);
        assert_eq!(up, down);
        assert_eq!(vertical_offset(text, 0, false), 0);
        assert_eq!(vertical_offset(text, text.len(), true), text.len());
    }
}

fn utf8_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}
