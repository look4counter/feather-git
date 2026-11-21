use crate::graph::GraphCommit;
use gpui::{prelude::*, *};
use std::{cell::RefCell, rc::Rc};

pub use crate::virtual_list::ROW_HEIGHT;
const HEAD_FILL: u32 = 0xffffff;
const GRAPH: [u32; 8] = [
    0x2563eb, 0x7c3aed, 0x168352, 0xd13d49, 0xa66c0d, 0x087f98, 0xb3297a, 0x4f56b9,
];

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
    rgb(GRAPH[index % GRAPH.len()])
}

fn node_geometry(center: Point<Pixels>, checked_out: bool, scale: f32) -> (Bounds<Pixels>, Pixels) {
    let radius = if checked_out { 5.5 } else { 4.5 };
    let diameter = (radius * 2. * scale).round().max(1.);
    let expansion = if checked_out {
        scale.round().max(1.)
    } else {
        0.
    };
    let x =
        (snap_graph_coordinate(center.x.into(), scale) * scale - diameter / 2. - expansion) / scale;
    let y =
        (snap_graph_coordinate(center.y.into(), scale) * scale - diameter / 2. - expansion) / scale;
    let diameter = px((diameter + expansion * 2.) / scale);
    (
        Bounds::new(point(px(x), px(y)), size(diameter, diameter)),
        px(graph_stroke_width(scale) + expansion / scale),
    )
}

fn circle_path(center: Point<Pixels>, radius: f32) -> anyhow::Result<Path<Pixels>> {
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
    path.build()
}

fn paint_node(window: &mut Window, center: Point<Pixels>, checked_out: bool, tint: Rgba) {
    let (bounds, ring) = node_geometry(center, checked_out, window.scale_factor());
    let radius = bounds.size.width / 2.;
    let center = bounds.origin + point(radius, radius);
    if let Ok(path) = circle_path(center, radius.into()) {
        window.paint_path(path, tint);
    }
    if checked_out && let Ok(path) = circle_path(center, (radius - ring).into()) {
        window.paint_path(path, rgb(HEAD_FILL));
    }
}

fn graph_stroke_width(scale: f32) -> f32 {
    (2. * scale).round().max(1.) / scale
}

fn snap_graph_coordinate(value: f32, scale: f32) -> f32 {
    let phase = (2. * scale).round().max(1.) % 2. / 2.;
    ((value * scale - phase).round() + phase) / scale
}

const LANE_SPACING: f32 = 18.;
const GRAPH_PADDING: f32 = 10.;
pub fn graph_width(lanes: usize) -> f32 {
    (GRAPH_PADDING * 2. + lanes.saturating_sub(1) as f32 * LANE_SPACING).max(45.)
}
fn lane_x(lane: usize) -> f32 {
    GRAPH_PADDING + lane as f32 * LANE_SPACING
}

const BOUNDARY_VERTICAL: f32 = 4.;

fn segment_points(
    start: Point<Pixels>,
    end: Point<Pixels>,
    boundary_at_start: bool,
) -> [Point<Pixels>; 3] {
    let elbow = if boundary_at_start {
        point(start.x, start.y + px(BOUNDARY_VERTICAL))
    } else {
        point(end.x, end.y - px(BOUNDARY_VERTICAL))
    };
    [start, elbow, end]
}

fn snapped_segment_points(
    start: Point<Pixels>,
    end: Point<Pixels>,
    origin: Point<Pixels>,
    scale: f32,
) -> [Point<Pixels>; 3] {
    let boundary_at_start = start.y == px(0.);
    let mut points = segment_points(start, end, boundary_at_start).map(|position| {
        let position = position + origin;
        point(
            px(snap_graph_coordinate(position.x.into(), scale)),
            px(snap_graph_coordinate(position.y.into(), scale)),
        )
    });
    // 인접 행이 별도로 그려져도 경계에서 배경이 비치지 않도록 겹친다.
    let overlap = px(1. / scale);
    if boundary_at_start {
        points[0].y -= overlap;
    } else {
        points[2].y += overlap;
    }
    points
}

fn segment_path(points: [Point<Pixels>; 3], stroke: Pixels) -> anyhow::Result<Path<Pixels>> {
    let [start, elbow, end] = points;
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
        let boundary_at_start = start.x == elbow.x;
        let (curve_start, curve_end) = if boundary_at_start {
            path.line_to(elbow);
            (elbow, end)
        } else {
            (start, elbow)
        };
        let middle_y = (curve_start.y + curve_end.y) / 2.;
        path.cubic_bezier_to(
            curve_end,
            point(curve_start.x, middle_y),
            point(curve_end.x, middle_y),
        );
        if !boundary_at_start {
            path.line_to(end);
        }
    }
    path.build()
}

impl RowGraph {
    pub fn new(commit: &GraphCommit) -> Self {
        let mut paths = Vec::new();
        for (edges, top) in [(&commit.input_edges, true), (&commit.output_edges, false)] {
            for edge in edges {
                let start = point(
                    px(lane_x(edge.from_lane)),
                    px(if top { 0. } else { ROW_HEIGHT / 2. }),
                );
                let end = point(
                    px(lane_x(edge.to_lane)),
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
                let key = (bounds.origin, bounds.size.width, lanes, scale);
                let mut cache = graph.cache.borrow_mut();
                if cache
                    .as_ref()
                    .is_none_or(|(cached_key, _)| *cached_key != key)
                {
                    let mut paths = Vec::new();
                    for (start, end, color) in graph.segments.iter() {
                        let points = snapped_segment_points(*start, *end, bounds.origin, scale);
                        if let Ok(path) = segment_path(points, stroke) {
                            paths.push((path, *color));
                        }
                    }
                    *cache = Some((key, paths));
                }
                let overlap = px(1. / scale);
                let mask = ContentMask {
                    bounds: Bounds::new(
                        bounds.origin - point(px(0.), overlap),
                        bounds.size + size(px(0.), overlap * 2.),
                    ),
                };
                window.with_content_mask(Some(mask), |window| {
                    if let Some((_, paths)) = cache.as_ref() {
                        for (path, color) in paths {
                            window.paint_path(path.clone(), *color);
                        }
                    }
                    let center = bounds.origin + point(px(lane_x(graph.lane)), px(ROW_HEIGHT / 2.));
                    paint_node(window, center, checked_out, color(graph.color));
                });
            },
        )
        .size_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn paths_build_for_vertical_curved_and_checkout_nodes() {
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            for offset in [-100.25, 0., 12.3] {
                let snap = |v: f32| px(snap_graph_coordinate(v + offset, scale));
                let stroke = px(graph_stroke_width(scale));
                for end_x in [10., 30.] {
                    assert!(
                        segment_path(
                            segment_points(
                                point(snap(10.), snap(0.)),
                                point(snap(end_x), snap(12.)),
                                true
                            ),
                            stroke
                        )
                        .is_ok()
                    );
                    assert!(
                        segment_path(
                            segment_points(
                                point(snap(end_x), snap(12.)),
                                point(snap(10.), snap(24.)),
                                false
                            ),
                            stroke
                        )
                        .is_ok()
                    );
                }
                for checked_out in [false, true] {
                    let (bounds, ring) =
                        node_geometry(point(snap(10.), snap(12.)), checked_out, scale);
                    let radius = bounds.size.width / 2.;
                    let center = bounds.origin + point(radius, radius);
                    assert!(circle_path(center, radius.into()).is_ok());
                    if checked_out {
                        assert!(circle_path(center, (radius - ring).into()).is_ok());
                    }
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn adjacent_nodes_keep_four_pixels_between_outer_bounds() {
        for lanes in 1..=64 {
            let width = graph_width(lanes);
            for scale in [1., 1.25, 1.5, 1.75, 2.] {
                for lane in 0..lanes - 1 {
                    let x = lane_x(lane);
                    let next_x = lane_x(lane + 1);
                    let (left, _) = node_geometry(point(px(x), px(12.)), true, scale);
                    let (right, _) = node_geometry(point(px(next_x), px(12.)), true, scale);
                    // 경계는 물리 픽셀에 정렬된다. 논리 좌표의 f32 뺄셈 오차를
                    // 실제 픽셀로 환산한 뒤 검증하며 최소 4px 조건은 유지한다.
                    let physical_gap = f32::from(right.left() - left.right()) * scale;
                    assert!((physical_gap - physical_gap.round()).abs() < 0.001);
                    assert!(
                        physical_gap.round() >= (4. * scale).ceil(),
                        "lane overlap: lanes={lanes}, dpi={scale}, physical_gap={physical_gap}"
                    );
                    assert!(left.left() >= px(0.) && right.right() <= px(width));
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn compact_rows_connect_at_top_middle_and_bottom() {
        assert_eq!(ROW_HEIGHT, 24.);
        let mut commits = vec![
            GraphCommit {
                id: "child".into(),
                parent_ids: vec!["root".into()],
                ..Default::default()
            },
            GraphCommit {
                id: "root".into(),
                ..Default::default()
            },
        ];
        crate::graph::build_commit_graph(&mut commits);
        let child = RowGraph::new(&commits[0]);
        let root = RowGraph::new(&commits[1]);
        assert_eq!(child.segments[0].0.y, px(12.));
        assert_eq!(child.segments[0].1.y, px(24.));
        assert_eq!(root.segments[0].0.y, px(0.));
        assert_eq!(root.segments[0].1.y, px(12.));
    }

    #[::core::prelude::v1::test]
    fn node_size_and_ring_keep_physical_pixels_while_center_tracks_lane() {
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            for checked_out in [false, true] {
                for offset in [-10.4, 0., 8.3, 21.75, 100.2] {
                    let center = point(px(offset), px(offset + 12.));
                    let (bounds, ring) = node_geometry(center, checked_out, scale);
                    let x: f32 = bounds.origin.x.into();
                    let y: f32 = bounds.origin.y.into();
                    let diameter: f32 = bounds.size.width.into();
                    assert_eq!(bounds.size.width, bounds.size.height);
                    for value in [diameter, f32::from(ring)] {
                        let physical = value * scale;
                        assert!((physical - physical.round()).abs() < 0.0001);
                    }
                    // 원 크기를 유지하며 중심을 맞추기 위해 원 경계는 반 픽셀을 허용한다.
                    for edge in [x, y, x + diameter, y + diameter] {
                        let half_pixels = edge * scale * 2.;
                        assert!((half_pixels - half_pixels.round()).abs() < 0.0002);
                    }
                    for (actual, expected) in [
                        (x + diameter / 2., offset),
                        (y + diameter / 2., offset + 12.),
                    ] {
                        assert!((actual - expected).abs() <= 0.501 / scale);
                    }
                    let radius = if checked_out { 5.5 } else { 4.5 };
                    let expansion = if checked_out {
                        scale.round() / scale
                    } else {
                        0.
                    };
                    assert!((diameter - expansion * 2. - radius * 2.).abs() <= 0.501 / scale);
                    if checked_out {
                        let old_diameter = (11. * scale).round() / scale;
                        let old_ring = graph_stroke_width(scale);
                        assert!(
                            (diameter - f32::from(ring) * 2. - (old_diameter - old_ring * 2.))
                                .abs()
                                < 0.0001
                        );
                        if scale == 1. {
                            assert_eq!(diameter, 13.);
                            assert_eq!(diameter - f32::from(ring) * 2., 7.);
                        }
                    }
                    assert!(ring * 2. < bounds.size.width);
                    assert!(bounds.size.height < px(ROW_HEIGHT));
                }
            }
        }
    }

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
    fn graph_column_grows_instead_of_compressing_lanes() {
        for lanes in [1, 2, 8, 32, 64] {
            let width = graph_width(lanes);
            assert_eq!(lane_x(0), 10.);
            assert!(lane_x(0) < width / 2.);
            assert!(width >= 45.);
            for lane in 0..lanes {
                let x = lane_x(lane);
                assert!(x - 7. >= 0. && x + 7. <= width);
                if lane > 0 {
                    assert_eq!(x - lane_x(lane - 1), 18.);
                }
            }
        }
        assert!(graph_width(8) > graph_width(2));
    }
}

#[cfg(test)]
mod center_regression_tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn node_center_matches_lane_center_at_every_dpi_and_fractional_origin() {
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            for offset in [-10.4, 0., 8.3, 21.75, 100.2] {
                for lane in 0..8 {
                    for checked_out in [false, true] {
                        let raw = point(px(offset + lane_x(lane)), px(offset + ROW_HEIGHT / 2.));
                        let line = point(
                            px(snap_graph_coordinate(raw.x.into(), scale)),
                            px(snap_graph_coordinate(raw.y.into(), scale)),
                        );
                        let (bounds, _) = node_geometry(raw, checked_out, scale);
                        let radius = bounds.size.width / 2.;
                        let node = bounds.origin + point(radius, radius);
                        assert!(
                            (f32::from(node.x - line.x) * scale).abs() < 0.0001,
                            "x center: dpi={scale}, checkout={checked_out}, node={node:?}, line={line:?}"
                        );
                        assert!(
                            (f32::from(node.y - line.y) * scale).abs() < 0.0001,
                            "y center: dpi={scale}, checkout={checked_out}, node={node:?}, line={line:?}"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn diagonal_segments_enter_and_leave_boundaries_vertically_at_every_dpi() {
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            for origin in [-100.25, 0., 12.3] {
                for (from, to) in [(0, 1), (1, 0), (0, 7), (7, 0)] {
                    for top in [true, false] {
                        let start =
                            point(px(lane_x(from)), px(if top { 0. } else { ROW_HEIGHT / 2. }));
                        let end = point(
                            px(lane_x(to)),
                            px(if top { ROW_HEIGHT / 2. } else { ROW_HEIGHT }),
                        );
                        let raw = segment_points(start, end, top);
                        assert_eq!(raw[0], start);
                        assert_eq!(raw[2], end);
                        if top {
                            assert_eq!(raw[0].x, raw[1].x);
                            assert_eq!(raw[1].y - raw[0].y, px(4.));
                        } else {
                            assert_eq!(raw[1].x, raw[2].x);
                            assert_eq!(raw[2].y - raw[1].y, px(4.));
                        }
                        let points = raw.map(|p| {
                            point(
                                px(snap_graph_coordinate(f32::from(p.x) + origin, scale)),
                                px(snap_graph_coordinate(f32::from(p.y) + origin, scale)),
                            )
                        });
                        let (boundary, elbow) = if top {
                            (points[0], points[1])
                        } else {
                            (points[2], points[1])
                        };
                        assert_eq!(boundary.x, elbow.x);
                        assert!((f32::from(boundary.y - elbow.y)).abs() >= 3.);
                        assert!(points[0].y < points[1].y && points[1].y < points[2].y);
                        assert!(segment_path(points, px(graph_stroke_width(scale))).is_ok());
                    }
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn rendered_edges_overlap_row_boundary_at_fractional_positions() {
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            for step in -32..=32 {
                let origin = point(px(0.3), px(step as f32 / (16. * scale)));
                let next_origin = origin + point(px(0.), px(ROW_HEIGHT));
                for (from, seam, to) in [(0, 0, 0), (0, 1, 2), (7, 0, 7), (0, 0, 1)] {
                    let before = snapped_segment_points(
                        point(px(lane_x(from)), px(ROW_HEIGHT / 2.)),
                        point(px(lane_x(seam)), px(ROW_HEIGHT)),
                        origin,
                        scale,
                    );
                    let after = snapped_segment_points(
                        point(px(lane_x(seam)), px(0.)),
                        point(px(lane_x(to)), px(ROW_HEIGHT / 2.)),
                        next_origin,
                        scale,
                    );
                    assert_eq!(before[2].x, after[0].x);
                    let minimum = px(0.499 / scale);
                    assert!(
                        before[2].y >= next_origin.y + minimum,
                        "outgoing dpi={scale}, step={step}",
                    );
                    assert!(
                        after[0].y <= next_origin.y - minimum,
                        "incoming dpi={scale}, step={step}",
                    );
                    assert!(segment_path(before, px(graph_stroke_width(scale))).is_ok());
                    assert!(segment_path(after, px(graph_stroke_width(scale))).is_ok());
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn adjacent_diagonal_rows_share_one_vertical_lane_at_the_boundary() {
        let outgoing = segment_points(
            point(px(lane_x(0)), px(12.)),
            point(px(lane_x(1)), px(24.)),
            false,
        );
        let incoming = segment_points(
            point(px(lane_x(1)), px(0.)),
            point(px(lane_x(0)), px(12.)),
            true,
        );
        for scale in [1., 1.25, 1.5, 1.75, 2.] {
            for origin in [-10.4, 0., 8.3, 21.75, 100.2] {
                let transform = |p: Point<Pixels>, row_offset: f32| {
                    point(
                        px(snap_graph_coordinate(f32::from(p.x) + origin, scale)),
                        px(snap_graph_coordinate(
                            f32::from(p.y) + origin + row_offset,
                            scale,
                        )),
                    )
                };
                let before = outgoing.map(|p| transform(p, 0.));
                let after = incoming.map(|p| transform(p, ROW_HEIGHT));
                assert_eq!(before[2], after[0]);
                assert_eq!(before[1].x, after[1].x);
                assert_eq!(before[1].x, before[2].x);
            }
        }
    }
}
