#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Selection {
    pub anchor: Position,
    pub cursor: Position,
}

impl Selection {
    pub fn select_word(&mut self, line: &str, position: Position) {
        let chars: Vec<_> = line.chars().collect();
        let column = position.column.min(chars.len());
        let mut start = column;
        let mut end = column;
        if let Some(&character) = chars.get(column) {
            let word = |ch: char| ch.is_alphanumeric() || ch == '_';
            let same = |ch: char| {
                if word(character) {
                    word(ch)
                } else {
                    character.is_whitespace() && ch.is_whitespace()
                }
            };
            while start > 0 && same(chars[start - 1]) {
                start -= 1;
            }
            end += 1;
            while end < chars.len() && same(chars[end]) {
                end += 1;
            }
        }
        self.anchor = Position {
            row: position.row,
            column: start,
        };
        self.cursor = Position {
            row: position.row,
            column: end,
        };
    }

    pub fn rows(&self, count: usize) -> std::ops::Range<usize> {
        let (start, end) = self.range();
        let exclusive_end = end
            .row
            .saturating_add(usize::from(start.row == end.row || end.column > 0));
        start.row.min(count)..exclusive_end.min(count)
    }

    pub fn move_cursor_on_rows(
        &mut self,
        lines: &[String],
        rows: &[usize],
        key: &str,
        extend: bool,
    ) {
        let rows: Vec<_> = rows
            .iter()
            .copied()
            .filter(|row| *row < lines.len())
            .collect();
        if rows.is_empty() {
            return;
        }
        let index = rows
            .binary_search(&self.cursor.row)
            .unwrap_or_else(|index| index.min(rows.len() - 1));
        let position = Position {
            row: index,
            column: self.cursor.column,
        };
        let mut local = Selection {
            anchor: position,
            cursor: position,
        };
        let numbered: Vec<_> = rows.iter().map(|row| lines[*row].as_str()).collect();
        local.move_cursor(&numbered, key, false);
        self.cursor = Position {
            row: rows[local.cursor.row],
            column: local.cursor.column,
        };
        if !extend {
            self.anchor = self.cursor;
        }
    }

    pub fn range(&self) -> (Position, Position) {
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
    }

    pub fn text(&self, lines: &[String]) -> String {
        let (start, end) = self.range();
        (start.row..=end.row.min(lines.len().saturating_sub(1)))
            .filter_map(|row| {
                lines.get(row).map(|line| {
                    let from = if row == start.row { start.column } else { 0 };
                    let to = if row == end.row {
                        end.column
                    } else {
                        line.chars().count()
                    };
                    line.chars()
                        .skip(from)
                        .take(to.saturating_sub(from))
                        .collect::<String>()
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn move_cursor(&mut self, lines: &[impl AsRef<str>], key: &str, extend: bool) {
        if lines.is_empty() {
            return;
        }
        self.cursor.row = self.cursor.row.min(lines.len() - 1);
        let len = lines[self.cursor.row].as_ref().chars().count();
        match key {
            "left" if self.cursor.column > 0 => self.cursor.column -= 1,
            "left" if self.cursor.row > 0 => {
                self.cursor.row -= 1;
                self.cursor.column = lines[self.cursor.row].as_ref().chars().count();
            }
            "right" if self.cursor.column < len => self.cursor.column += 1,
            "right" if self.cursor.row + 1 < lines.len() => {
                self.cursor.row += 1;
                self.cursor.column = 0;
            }
            "up" => self.cursor.row = self.cursor.row.saturating_sub(1),
            "down" => self.cursor.row = (self.cursor.row + 1).min(lines.len() - 1),
            "home" => self.cursor.column = 0,
            "end" => self.cursor.column = len,
            _ => {}
        }
        self.cursor.column = self
            .cursor
            .column
            .min(lines[self.cursor.row].as_ref().chars().count());
        if !extend {
            self.anchor = self.cursor;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_click_word_selection_copies_identifiers_unicode_and_boundaries() {
        let mut selection = Selection::default();
        let raw = "한글 foo_bar42, 끝";
        for column in [3, 6, 11] {
            selection.select_word(raw, Position { row: 0, column });
            assert_eq!(
                selection.range(),
                (
                    Position { row: 0, column: 3 },
                    Position { row: 0, column: 12 }
                )
            );
            assert_eq!(selection.text(&[raw.into()]), "foo_bar42");
        }
        for (column, expected) in [
            (0, "한글"),
            (1, "한글"),
            (12, ","),
            (13, " "),
            (14, "끝"),
            (15, ""),
            (99, ""),
        ] {
            selection.select_word(raw, Position { row: 0, column });
            assert_eq!(selection.text(&[raw.into()]), expected);
        }
        selection.select_word("a\t  b", Position { row: 0, column: 1 });
        assert_eq!(selection.text(&["a\t  b".into()]), "\t  ");
        selection.select_word("a++b", Position { row: 0, column: 1 });
        assert_eq!(selection.text(&["a++b".into()]), "+");
        selection.select_word(
            "",
            Position {
                row: 2,
                column: 100,
            },
        );
        assert_eq!(selection.anchor, Position { row: 2, column: 0 });
        assert_eq!(selection.anchor, selection.cursor);
    }

    #[test]
    fn selected_rows_handle_reverse_ranges_zero_column_end_and_collapsed_cursor() {
        let mut selection = Selection {
            anchor: Position { row: 1, column: 2 },
            cursor: Position { row: 4, column: 0 },
        };
        assert_eq!(selection.rows(8), 1..4);
        std::mem::swap(&mut selection.anchor, &mut selection.cursor);
        assert_eq!(selection.rows(8), 1..4);
        selection.anchor.column = 1;
        assert_eq!(selection.rows(8), 1..5);
        selection.anchor = selection.cursor;
        assert_eq!(selection.rows(8), 1..2);
        assert_eq!(selection.rows(0), 0..0);
        selection.anchor = Position { row: 99, column: 0 };
        selection.cursor = selection.anchor;
        assert_eq!(selection.rows(8), 8..8);
    }

    #[test]
    fn filtered_cursor_moves_across_unnumbered_rows_and_preserves_selection_anchor() {
        let lines: Vec<String> = ["header", "한글", "header", "abc", "footer"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let first = Position { row: 1, column: 2 };
        let mut selection = Selection {
            anchor: first,
            cursor: first,
        };
        selection.move_cursor_on_rows(&lines, &[1, 3], "right", true);
        assert_eq!(selection.cursor, Position { row: 3, column: 0 });
        assert_eq!(selection.anchor, first);
        selection.move_cursor_on_rows(&lines, &[1, 3], "left", false);
        assert_eq!(selection.cursor, first);
        selection.move_cursor_on_rows(&lines, &[1, 3], "down", true);
        assert_eq!(selection.cursor, Position { row: 3, column: 2 });
        selection.move_cursor_on_rows(&lines, &[1, 3], "down", true);
        assert_eq!(selection.cursor.row, 3);
        selection.move_cursor_on_rows(&lines, &[1, 3], "up", false);
        assert_eq!(selection.cursor.row, 1);
        selection.move_cursor_on_rows(&lines, &[1, 3], "up", false);
        assert_eq!(selection.cursor.row, 1);
        let before = selection.cursor;
        selection.move_cursor_on_rows(&lines, &[], "down", false);
        assert_eq!(selection.cursor, before);
    }
    #[test]
    fn unicode_reverse_multiline_copy_preserves_tabs_and_empty_lines() {
        let lines = vec!["a한글\t끝".into(), "".into(), "마지막".into()];
        let mut selection = Selection {
            anchor: Position { row: 2, column: 2 },
            cursor: Position { row: 0, column: 1 },
        };
        assert_eq!(selection.text(&lines), "한글\t끝\n\n마지");
        selection.move_cursor(&lines, "left", true);
        assert_eq!(selection.text(&lines), "a한글\t끝\n\n마지");
        selection.move_cursor(&lines, "up", false);
        assert!(selection.text(&lines).is_empty());
    }
}
