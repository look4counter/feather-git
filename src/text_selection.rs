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

    pub fn move_cursor(&mut self, lines: &[String], key: &str, extend: bool) {
        if lines.is_empty() {
            return;
        }
        self.cursor.row = self.cursor.row.min(lines.len() - 1);
        let len = lines[self.cursor.row].chars().count();
        match key {
            "left" if self.cursor.column > 0 => self.cursor.column -= 1,
            "left" if self.cursor.row > 0 => {
                self.cursor.row -= 1;
                self.cursor.column = lines[self.cursor.row].chars().count();
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
            .min(lines[self.cursor.row].chars().count());
        if !extend {
            self.anchor = self.cursor;
        }
    }
}

pub fn adjacent(current: Option<usize>, count: usize, down: bool) -> Option<usize> {
    if count == 0 {
        None
    } else {
        Some(match current {
            None => {
                if down {
                    0
                } else {
                    count - 1
                }
            }
            Some(index) => {
                if down {
                    index.saturating_add(1).min(count - 1)
                } else {
                    index.min(count - 1).saturating_sub(1)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    #[test]
    fn list_navigation_stops_at_edges_and_handles_empty_lists() {
        assert_eq!(adjacent(None, 0, true), None);
        assert_eq!(adjacent(None, 3, true), Some(0));
        assert_eq!(adjacent(Some(0), 3, false), Some(0));
        assert_eq!(adjacent(Some(2), 3, true), Some(2));
    }
}
