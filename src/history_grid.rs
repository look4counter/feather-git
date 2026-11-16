pub const COLUMN_TITLES: [&str; 5] = ["그래프", "메시지", "작성자", "날짜", "해시"];

pub fn visible_columns(show_graph: bool) -> std::ops::Range<usize> {
    usize::from(!show_graph)..COLUMN_TITLES.len()
}

pub const MESSAGE_MIN_WIDTH: f32 = 160.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistoryColumns {
    widths: [f32; 5],
}

impl Default for HistoryColumns {
    fn default() -> Self {
        Self {
            widths: [crate::history_graph::graph_width(5), 360., 110., 140., 96.],
        }
    }
}

impl HistoryColumns {
    fn minimums() -> [f32; 5] {
        [0., MESSAGE_MIN_WIDTH, 60., 96., 64.]
    }

    pub fn widths(&self) -> [f32; 5] {
        let minimums = Self::minimums();
        std::array::from_fn(|column| self.widths[column].max(minimums[column]))
    }

    pub fn minimum_total_width(&self) -> f32 {
        self.minimum_total_width_for(true)
    }

    pub fn minimum_total_width_for(&self, show_graph: bool) -> f32 {
        let mut widths = self.widths();
        widths[1] = MESSAGE_MIN_WIDTH;
        visible_columns(show_graph)
            .map(|column| widths[column])
            .sum()
    }

    pub fn resize_boundary(&mut self, boundary: usize, start: [f32; 5], delta: f32) -> bool {
        let minimums = Self::minimums();
        if boundary >= 4
            || !delta.is_finite()
            || !start.iter().sum::<f32>().is_finite()
            || start
                .iter()
                .zip(minimums)
                .any(|(width, minimum)| !width.is_finite() || *width < minimum)
        {
            return false;
        }
        let right = boundary + 1;
        let delta = delta.clamp(
            minimums[boundary] - start[boundary],
            start[right] - minimums[right],
        );
        let mut widths = start;
        widths[boundary] += delta;
        widths[right] -= delta;
        if self.widths == widths {
            return false;
        }
        self.widths = widths;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_hidden_grid_keeps_the_other_four_columns_and_widths() {
        let columns = HistoryColumns::default();
        assert_eq!(visible_columns(false).collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert_eq!(visible_columns(true).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        assert_eq!(
            columns.minimum_total_width() - columns.minimum_total_width_for(false),
            columns.widths()[0]
        );
    }

    #[test]
    fn default_graph_fits_five_lanes_and_remains_user_resizable() {
        let five_lanes = crate::history_graph::graph_width(5);
        let mut columns = HistoryColumns::default();
        assert_eq!(columns.widths()[0], five_lanes);
        assert_eq!(five_lanes, 92.);
        assert_eq!(columns.widths()[0], five_lanes);
        let start = columns.widths();
        assert!(columns.resize_boundary(0, start, -20.));
        assert_eq!(columns.widths()[0], five_lanes - 20.);
        assert_eq!(
            columns.widths().iter().sum::<f32>(),
            start.iter().sum::<f32>()
        );
    }

    #[test]
    fn graph_can_collapse_to_zero_and_expand_without_changing_total() {
        let mut columns = HistoryColumns::default();
        let start = columns.widths();
        assert!(columns.resize_boundary(0, start, -10_000.));
        let collapsed = columns.widths();
        assert_eq!(collapsed[0], 0.);
        assert_eq!(collapsed[1], start[0] + start[1]);
        assert_eq!(collapsed.iter().sum::<f32>(), start.iter().sum::<f32>());
        assert!(columns.resize_boundary(0, collapsed, 25.));
        assert_eq!(columns.widths()[0], 25.);
        assert_eq!(
            columns.widths().iter().sum::<f32>(),
            start.iter().sum::<f32>()
        );
        let mut invalid = start;
        invalid[0] = -1.;
        assert!(!columns.resize_boundary(0, invalid, 10.));
    }

    #[test]
    fn each_boundary_moves_only_its_two_columns_and_keeps_total_width() {
        let start = [100., 700., 110., 140., 96.];
        for boundary in 0..4 {
            for delta in [-20., 30.] {
                let mut columns = HistoryColumns::default();
                assert!(columns.resize_boundary(boundary, start, delta));
                let result = columns.widths();
                assert_eq!(result[boundary], start[boundary] + delta);
                assert_eq!(result[boundary + 1], start[boundary + 1] - delta);
                assert_eq!(result.iter().sum::<f32>(), start.iter().sum::<f32>());
                for column in 0..5 {
                    if column != boundary && column != boundary + 1 {
                        assert_eq!(result[column], start[column]);
                    }
                }
                assert!(columns.minimum_total_width() <= start.iter().sum::<f32>());
            }
        }
    }

    #[test]
    fn both_sides_stop_at_minimum_without_changing_the_pair_total() {
        let start = [230., 700., 110., 140., 96.];
        let minimum = [0., 160., 60., 96., 64.];
        for boundary in 0..4 {
            let mut columns = HistoryColumns::default();
            columns.resize_boundary(boundary, start, 10_000.);
            let result = columns.widths();
            assert_eq!(result[boundary + 1], minimum[boundary + 1]);
            assert_eq!(
                result[boundary] + result[boundary + 1],
                start[boundary] + start[boundary + 1]
            );
            columns.resize_boundary(boundary, start, -10_000.);
            let result = columns.widths();
            assert_eq!(result[boundary], minimum[boundary]);
            assert_eq!(
                result[boundary] + result[boundary + 1],
                start[boundary] + start[boundary + 1]
            );
        }
    }

    #[test]
    fn repeated_moves_use_original_measurements_and_can_return_to_start() {
        let start = [45., 700., 110., 140., 96.];
        let mut columns = HistoryColumns::default();
        columns.resize_boundary(1, start, 20.);
        columns.resize_boundary(1, start, 30.);
        assert_eq!(columns.widths(), [45., 730., 80., 140., 96.]);
        columns.resize_boundary(1, start, 0.);
        assert_eq!(columns.widths(), start);
        assert_eq!(
            columns.minimum_total_width(),
            45. + 160. + 110. + 140. + 96.
        );
    }

    #[test]
    fn invalid_boundaries_and_measurements_do_not_change_widths() {
        let mut columns = HistoryColumns::default();
        let original = columns;
        let start = [45., 700., 110., 140., 96.];
        assert!(!columns.resize_boundary(4, start, 10.));
        for delta in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(!columns.resize_boundary(1, start, delta));
        }
        let mut invalid = start;
        invalid[1] = f32::NAN;
        assert!(!columns.resize_boundary(1, invalid, 10.));
        invalid[1] = 10.;
        assert!(!columns.resize_boundary(1, invalid, 10.));
        assert_eq!(columns, original);
    }
}
