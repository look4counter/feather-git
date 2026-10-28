use crate::theme;
use feather_git::model::Commit;
use gpui::{prelude::*, *};
use std::{cell::RefCell, rc::Rc};

pub const ROW_HEIGHT: f32 = 24.;
pub const COLUMNS: [&str; 5] = ["Graph", "Description", "Author", "Date", "Hash"];
const MIN_WIDTHS: [f32; 5] = [36., 72., 52., 136., 76.];

#[derive(Clone, Copy)]
pub struct ColumnLayout {
    weights: [f32; 5],
}

impl Default for ColumnLayout {
    fn default() -> Self {
        Self {
            weights: [0.17 / 0.79, 0.47 / 0.79, 0.15 / 0.79, 0., 0.],
        }
    }
}

impl ColumnLayout {
    pub fn widths(&self, width: f32) -> [f32; 5] {
        let width = if width.is_finite() { width.max(0.) } else { 0. };
        let minimum: f32 = MIN_WIDTHS.iter().sum();
        let scale = (width / minimum).min(1.);
        std::array::from_fn(|i| MIN_WIDTHS[i] * scale + (width - minimum).max(0.) * self.weights[i])
    }

    pub fn resize(&mut self, boundary: usize, delta: f32, width: f32) {
        let minimum: f32 = MIN_WIDTHS.iter().sum();
        if boundary >= 4 || !delta.is_finite() || !width.is_finite() || width <= minimum {
            return;
        }
        let widths = self.widths(width);
        let delta = delta.clamp(
            MIN_WIDTHS[boundary] - widths[boundary],
            widths[boundary + 1] - MIN_WIDTHS[boundary + 1],
        );
        self.weights[boundary] =
            (widths[boundary] + delta - MIN_WIDTHS[boundary]) / (width - minimum);
        self.weights[boundary + 1] =
            (widths[boundary + 1] - delta - MIN_WIDTHS[boundary + 1]) / (width - minimum);
    }
}

pub fn format_time(seconds: i64, offset: i32) -> String {
    gix::date::Time::new(seconds, offset)
        .format(gix::date::time::format::ISO8601)
        .map(|date| date.chars().take(16).collect::<String>().replace('T', " "))
        .unwrap_or_else(|_| seconds.to_string())
}

type PathCache = (
    (Point<Pixels>, Pixels, usize, f32),
    Vec<(Path<Pixels>, Rgba)>,
);
type Segment = (Point<Pixels>, Point<Pixels>, Rgba);

#[derive(Clone)]
pub struct RowGraph {
    segments: Rc<Vec<Segment>>,
    cache: Rc<RefCell<Option<PathCache>>>,
    lane: usize,
    color: usize,
}

fn color(index: usize) -> Rgba {
    rgb(theme::GRAPH[index % theme::GRAPH.len()])
}

fn paint_circle(window: &mut Window, center: Point<Pixels>, radius: f32, tint: Rgba) {
    let r = px(radius);
    let k = px(radius * 0.552_284_8);
    let mut path = PathBuilder::fill();
    path.move_to(center + point(r, px(0.)));
    path.cubic_bezier_to(
        center + point(px(0.), r),
        center + point(r, k),
        center + point(k, r),
    );
    path.cubic_bezier_to(
        center - point(r, px(0.)),
        center + point(-k, r),
        center + point(-r, k),
    );
    path.cubic_bezier_to(
        center - point(px(0.), r),
        center - point(r, k),
        center - point(k, r),
    );
    path.cubic_bezier_to(
        center + point(r, px(0.)),
        center + point(k, -r),
        center + point(r, -k),
    );
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, tint);
    }
}

fn graph_stroke_width(scale: f32) -> f32 {
    (2. * scale).round().max(1.) / scale
}

fn snap_graph_coordinate(value: f32, scale: f32) -> f32 {
    let phase = (2. * scale).round().max(1.) % 2. / 2.;
    ((value * scale - phase).round() + phase) / scale
}

fn graph_x(x: f32, width: f32, lanes: usize) -> f32 {
    let left = 8_f32.min(width.max(0.) / 2.);
    let span = lanes.saturating_sub(1) as f32 * 12.;
    let scale = if span > 0. {
        ((width - 16.).max(0.) / span).min(1.)
    } else {
        1.
    };
    left + (x - 8.) * scale
}

impl RowGraph {
    pub fn new(commit: &Commit) -> Self {
        let mut paths = Vec::new();
        for (edges, top) in [(&commit.input_edges, true), (&commit.output_edges, false)] {
            for edge in edges {
                let start = point(
                    px(8. + edge.from_lane as f32 * 12.),
                    px(if top { 0. } else { ROW_HEIGHT / 2. }),
                );
                let end = point(
                    px(8. + edge.to_lane as f32 * 12.),
                    px(if top { ROW_HEIGHT / 2. } else { ROW_HEIGHT }),
                );
                paths.push((start, end, color(edge.color)));
            }
        }
        Self {
            segments: Rc::new(paths),
            cache: Rc::new(RefCell::new(None)),
            lane: commit.marker.lane,
            color: commit.marker.color,
        }
    }

    pub fn render(&self, checked_out: bool, lanes: usize) -> impl IntoElement + use<> {
        let graph = self.clone();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let scale = window.scale_factor();
                let stroke = px(graph_stroke_width(scale));
                let snap = |position: Point<Pixels>| {
                    point(
                        px(snap_graph_coordinate(position.x.into(), scale)),
                        px(snap_graph_coordinate(position.y.into(), scale)),
                    )
                };
                let key = (bounds.origin, bounds.size.width, lanes, scale);
                let mut cache = graph.cache.borrow_mut();
                if cache
                    .as_ref()
                    .is_none_or(|(cached_key, _)| *cached_key != key)
                {
                    let mut paths = Vec::new();
                    for (start, end, color) in graph.segments.iter() {
                        let start = point(
                            px(graph_x(start.x.into(), bounds.size.width.into(), lanes)),
                            start.y,
                        ) + bounds.origin;
                        let end = point(
                            px(graph_x(end.x.into(), bounds.size.width.into(), lanes)),
                            end.y,
                        ) + bounds.origin;
                        let start = snap(start);
                        let end = snap(end);
                        let mut path = if start.x == end.x {
                            PathBuilder::fill()
                        } else {
                            PathBuilder::stroke(stroke)
                        };
                        if start.x == end.x {
                            path.move_to(start - point(stroke / 2., px(0.)));
                            path.line_to(start + point(stroke / 2., px(0.)));
                            path.line_to(end + point(stroke / 2., px(0.)));
                            path.line_to(end - point(stroke / 2., px(0.)));
                            path.close();
                        } else {
                            path.move_to(start);
                            let offset = (end.y - start.y) * 0.6;
                            path.cubic_bezier_to(
                                end,
                                point(start.x, start.y + offset),
                                point(end.x, end.y - offset),
                            );
                        }
                        if let Ok(path) = path.build() {
                            paths.push((path, *color));
                        }
                    }
                    *cache = Some((key, paths));
                }
                if let Some((_, paths)) = cache.as_ref() {
                    for (path, color) in paths {
                        window.paint_path(path.clone(), *color);
                    }
                }
                let center = bounds.origin
                    + point(
                        px(graph_x(
                            8. + graph.lane as f32 * 12.,
                            bounds.size.width.into(),
                            lanes,
                        )),
                        px(ROW_HEIGHT / 2.),
                    );
                let center = snap(center);
                let radius = if checked_out { 5.5 } else { 4.5 };
                paint_circle(window, center, radius, color(graph.color));
                if checked_out {
                    paint_circle(window, center, radius - 2., rgb(theme::HEAD_FILL));
                }
            },
        )
        .size_full()
    }
}

pub fn cell(_index: usize, width: f32) -> Div {
    div()
        .w(px(width))
        .h(px(ROW_HEIGHT))
        .flex_shrink_0()
        .min_w_0()
        .overflow_hidden()
        .px_2()
        .flex()
        .items_center()
        .text_xs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn graph_strokes_align_to_physical_pixel_edges_at_fractional_dpi() {
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            let width = graph_stroke_width(scale);
            for position in [-10.4, 0., 8.3, 21.75, 100.2] {
                let snapped = snap_graph_coordinate(position, scale);
                for edge in [snapped - width / 2., snapped + width / 2.] {
                    let physical = edge * scale;
                    assert!((physical - physical.round()).abs() < 0.0001);
                }
                assert!((snapped - position).abs() <= 0.501 / scale);
            }
        }
    }

    #[::core::prelude::v1::test]
    fn graph_nodes_stay_inside_narrow_columns_with_shared_lane_spacing() {
        for lanes in [1, 2, 8, 32] {
            for width in [16., 20., 36., 64., 240.] {
                let mut previous = -1.;
                for lane in 0..lanes {
                    let x = graph_x(8. + lane as f32 * 12., width, lanes);
                    assert!(x - 5.5 >= 0.);
                    assert!(x + 5.5 <= width);
                    assert!(x >= previous);
                    previous = x;
                }
            }
        }
        assert_eq!(graph_x(20., 100., 3), 20.);
        assert!(graph_x(20., 20., 3) < graph_x(20., 100., 3));
    }

    #[::core::prelude::v1::test]
    fn resize_preserves_total_and_minima_across_window_sizes() {
        let mut layout = ColumnLayout::default();
        for boundary in 0..4 {
            layout.resize(boundary, 10000., 1000.);
            layout.resize(boundary, -10000., 1000.);
            for width in [0., 240., 360., 800., 1920.] {
                let widths = layout.widths(width);
                assert!((widths.iter().sum::<f32>() - width).abs() < 0.001);
                assert!(widths.iter().all(|w| *w >= 0.));
                if width >= MIN_WIDTHS.iter().sum() {
                    for i in 0..5 {
                        assert!(widths[i] + 0.001 >= MIN_WIDTHS[i]);
                    }
                }
            }
        }
        let before = layout.widths(800.);
        layout.resize(9, 10., 800.);
        layout.resize(0, f32::NAN, 800.);
        assert_eq!(before, layout.widths(800.));
    }

    #[::core::prelude::v1::test]
    fn dragging_only_changes_adjacent_columns_and_keeps_relative_sizes() {
        let mut layout = ColumnLayout::default();
        let before = layout.widths(1000.);
        layout.resize(1, 20., 1000.);
        let after = layout.widths(1000.);
        assert!((after[1] - before[1] - 20.).abs() < 0.001);
        assert!((after[2] - before[2] + 20.).abs() < 0.001);
        for i in [0, 3, 4] {
            assert_eq!(before[i], after[i]);
        }
        let smaller = layout.widths(640.);
        assert!((smaller.iter().sum::<f32>() - 640.).abs() < 0.001);
    }

    #[::core::prelude::v1::test]
    fn default_metadata_widths_stay_compact_when_window_grows() {
        let layout = ColumnLayout::default();
        let widths = layout.widths(800.);
        let wider = layout.widths(1920.);
        assert_eq!(&widths[3..], &[136., 76.]);
        assert_eq!(&wider[3..], &widths[3..]);
        for i in 0..3 {
            assert!(wider[i] > widths[i]);
        }
        assert!((wider.iter().sum::<f32>() - 1920.).abs() < 0.001);
    }

    #[::core::prelude::v1::test]
    fn date_includes_hours_and_minutes_in_commit_timezone() {
        assert_eq!(format_time(0, 9 * 3600), "1970-01-01 09:00");
        assert_eq!(format_time(3661, 0), "1970-01-01 01:01");
        assert_eq!(format_time(0, -4 * 3600), "1969-12-31 20:00");
    }
}
