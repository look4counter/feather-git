use crate::theme::*;
use gpui::{prelude::*, *};
use std::{
    f32::consts::{PI, TAU},
    time::Duration,
};

const IMPACT: f32 = 0.52;
const WATER_Y: f32 = 104.;
const WATER_X: f32 = 92.;
const FEATHER_TIP_Y: f32 = 19.;

#[derive(Clone, Copy)]
struct Ripple {
    radius: f32,
    opacity: f32,
}
struct Frame {
    x: f32,
    y: f32,
    angle: f32,
    opacity: f32,
    ripples: [Option<Ripple>; 3],
}
impl Frame {
    fn at(phase: f32) -> Self {
        let phase = phase.clamp(0., 1.);
        let fall = (phase / IMPACT).min(1.);
        let eased = fall * fall * (3. - 2. * fall);
        let sink = ((phase - IMPACT) / 0.14).clamp(0., 1.);
        Self {
            x: 90. + (fall * TAU).sin() * 12. * (1. - fall),
            y: 20. + (WATER_Y - FEATHER_TIP_Y - 20.) * eased + sink * 9.,
            angle: (-0.35 + (fall * PI * 3.).sin() * 0.18) * (1. - fall),
            opacity: (phase / 0.08).min(1.) * (1. - sink),
            ripples: std::array::from_fn(|index| {
                let elapsed = phase - IMPACT - index as f32 * 0.095;
                if !(0.0..=0.34).contains(&elapsed) {
                    return None;
                }
                let progress = elapsed / 0.34;
                Some(Ripple {
                    radius: 5. + 59. * progress,
                    opacity: (1. - progress) * 0.6 * ((1. - phase) / 0.08).min(1.),
                })
            }),
        }
    }
}
fn tint(color: u32, opacity: f32) -> Rgba {
    rgba((color << 8) | (opacity.clamp(0., 1.) * 255.).round() as u32)
}
fn paint_feather(
    window: &mut Window,
    center: Point<Pixels>,
    angle: f32,
    opacity: f32,
    scale: f32,
    color: u32,
) {
    let (sin, cos) = angle.sin_cos();
    let feather = |x: f32, y: f32| {
        center
            + point(
                px((x * cos - y * sin) * scale),
                px((x * sin + y * cos) * scale),
            )
    };
    let mut shape = PathBuilder::fill();
    shape.move_to(feather(-2., -18.));
    shape.cubic_bezier_to(feather(8., 0.), feather(10., -13.), feather(12., -5.));
    shape.cubic_bezier_to(feather(2., 17.), feather(6., 7.), feather(2., 13.));
    shape.cubic_bezier_to(feather(-9., 2.), feather(-5., 13.), feather(-10., 8.));
    shape.cubic_bezier_to(feather(-2., -18.), feather(-14., -6.), feather(-5., -13.));
    shape.close();
    window.paint_path(
        shape.build().expect("깃털 윤곽 경로 생성 실패"),
        tint(color, opacity * 0.94),
    );
    let mut stem = PathBuilder::stroke(px(1.));
    stem.move_to(feather(-2., -14.));
    stem.curve_to(feather(2., FEATHER_TIP_Y), feather(-3., 2.));
    for (start, end) in [
        ((-2., -8.), (5., -12.)),
        ((-2., -3.), (7., -6.)),
        ((-1., 3.), (6., -1.)),
        ((0., 8.), (4., 5.)),
        ((-2., -5.), (-8., -9.)),
        ((-1., 1.), (-9., -3.)),
        ((0., 7.), (-6., 3.)),
    ] {
        stem.move_to(feather(start.0, start.1));
        stem.line_to(feather(end.0, end.1));
    }
    window.paint_path(
        stem.build().expect("깃털 줄기 경로 생성 실패"),
        tint(BASE, opacity * 0.7),
    );
}

fn paint_scene(window: &mut Window, origin: Point<Pixels>, frame: &Frame) {
    let at = |x, y| origin + point(px(x), px(y));
    let mut water = PathBuilder::stroke(px(1.));
    water.move_to(at(18., WATER_Y));
    water.line_to(at(162., WATER_Y));
    window.paint_path(
        water.build().expect("수면 경로 생성 실패"),
        tint(ACCENT, 0.16),
    );

    if frame.opacity > 0. {
        paint_feather(
            window,
            at(frame.x, frame.y),
            frame.angle,
            frame.opacity,
            1.,
            TEXT,
        );
    }
    for ripple in frame.ripples.iter().flatten().filter(|r| r.opacity > 0.) {
        let center = at(WATER_X, WATER_Y);
        let radius = point(px(ripple.radius), px(ripple.radius * 0.26));
        let mut path = PathBuilder::stroke(px(1.3));
        path.move_to(center + point(radius.x, px(0.)));
        path.arc_to(
            radius,
            px(0.),
            false,
            true,
            center - point(radius.x, px(0.)),
        );
        path.arc_to(
            radius,
            px(0.),
            false,
            true,
            center + point(radius.x, px(0.)),
        );
        path.close();
        window.paint_path(
            path.build().expect("물결 경로 생성 실패"),
            tint(ACCENT, ripple.opacity),
        );
    }
}

#[derive(IntoElement)]
struct FeatherWaterScene {
    phase: f32,
}
impl RenderOnce for FeatherWaterScene {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| paint_scene(window, bounds.origin, &Frame::at(self.phase)),
        )
        .w(px(180.))
        .h(px(140.))
        .flex_shrink_0()
    }
}

#[derive(IntoElement)]
pub struct ProgressOverlay {
    message: SharedString,
    top_inset: Pixels,
    focus: Option<FocusHandle>,
}
impl ProgressOverlay {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
            top_inset: px(0.),
            focus: None,
        }
    }
    pub fn top_inset(mut self, inset: Pixels) -> Self {
        self.top_inset = inset;
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
}
impl RenderOnce for ProgressOverlay {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id("progress-overlay")
            .when_some(self.focus, |overlay, focus| overlay.track_focus(&focus))
            .absolute()
            .top(self.top_inset)
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .p_5()
            .bg(rgba(0x00000099))
            .occlude()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_key_down(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .id("progress-card")
                    .w(px(280.))
                    .max_w_full()
                    .max_h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .p_4()
                    .bg(rgb(PANEL))
                    .border_1()
                    .border_color(rgb(LINE))
                    .rounded_lg()
                    .child(FeatherWaterScene { phase: 0. }.with_animation(
                        (ElementId::from("feather-water"), self.message.clone()),
                        Animation::new(Duration::from_millis(2800)).repeat(),
                        |mut scene, phase| {
                            scene.phase = phase;
                            scene
                        },
                    ))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(TEXT))
                            .text_align(TextAlign::Center)
                            .child(self.message),
                    ),
            )
    }
}

const FLIGHT_WIDTH: f32 = 680.;
const FLIGHT_HEIGHT: f32 = 300.;

#[derive(Clone, Copy)]
struct FallingFeather {
    x: f32,
    y: f32,
    angle: f32,
    opacity: f32,
    scale: f32,
}

struct FlightFrame {
    bird_x: f32,
    bird_y: f32,
    bird_opacity: f32,
    wing: f32,
    feathers: [Option<FallingFeather>; 6],
}
impl FlightFrame {
    fn at(phase: f32) -> Self {
        let phase = phase.clamp(0., 1.);
        Self {
            bird_x: -70. + 820. * phase,
            bird_y: 66. + (phase * TAU * 1.5).sin() * 11.,
            bird_opacity: (phase / 0.07).min(1.) * ((1. - phase) / 0.09).min(1.),
            wing: (phase * TAU * 11.).sin(),
            feathers: std::array::from_fn(|index| {
                let release = 0.15 + index as f32 * 0.095;
                let age = (phase - release) / 0.37;
                if !(0.0..=1.0).contains(&age) {
                    return None;
                }
                let sway = (age * TAU * 1.4 + index as f32).sin();
                Some(FallingFeather {
                    x: -82. + 820. * release + age * 48. + sway * 16. * age,
                    y: 78. + (release * TAU * 1.5).sin() * 11. + age.powf(1.22) * 170.,
                    angle: -0.42 + age * 1.1 + sway * 0.45,
                    opacity: (age / 0.10).min(1.) * ((1. - age) / 0.22).min(1.),
                    scale: 0.68 + (index % 3) as f32 * 0.12,
                })
            }),
        }
    }
}

fn paint_flight(window: &mut Window, bounds: Bounds<Pixels>, frame: &FlightFrame) {
    let width: f32 = bounds.size.width.into();
    let height: f32 = bounds.size.height.into();
    let scale = (width / FLIGHT_WIDTH).min(height / FLIGHT_HEIGHT).max(0.);
    let origin = bounds.origin
        + point(
            px((width - FLIGHT_WIDTH * scale) / 2.),
            px((height - FLIGHT_HEIGHT * scale) / 2.),
        );
    let at = |x, y| origin + point(px(x * scale), px(y * scale));
    for (offset, opacity) in [(0., 0.07), (17., 0.04)] {
        let mut trail = PathBuilder::stroke(px(scale));
        trail.move_to(at(80., 95. + offset));
        trail.cubic_bezier_to(
            at(590., 66. + offset),
            at(240., 10. + offset),
            at(410., 130. + offset),
        );
        window.paint_path(
            trail.build().expect("비행 궤적 경로 생성 실패"),
            tint(ACCENT, opacity),
        );
    }
    if frame.bird_opacity > 0. {
        let bird = |x, y| at(frame.bird_x + x, frame.bird_y + y);
        let wing_tip = -32. + frame.wing * 39.;
        let mut far_wing = PathBuilder::fill();
        far_wing.move_to(bird(-7., 2.));
        far_wing.cubic_bezier_to(
            bird(5., wing_tip * 0.8 - 7.),
            bird(-29., -12.),
            bird(-31., wing_tip - 4.),
        );
        far_wing.cubic_bezier_to(bird(13., 0.), bird(14., wing_tip * 0.6), bird(9., -5.));
        far_wing.close();
        window.paint_path(
            far_wing.build().expect("새 뒷날개 경로 생성 실패"),
            tint(ACCENT, frame.bird_opacity * 0.45),
        );

        let mut body = PathBuilder::fill();
        body.move_to(bird(-24., 0.));
        body.line_to(bird(-44., -9.));
        body.line_to(bird(-35., 5.));
        body.line_to(bird(-43., 13.));
        body.line_to(bird(-20., 8.));
        body.cubic_bezier_to(bird(19., 6.), bird(-5., 18.), bird(11., 14.));
        body.cubic_bezier_to(bird(31., -4.), bird(29., 1.), bird(20., -9.));
        body.line_to(bird(43., -1.));
        body.line_to(bird(31., 2.));
        body.cubic_bezier_to(bird(-24., 0.), bird(18., -6.), bird(-4., -6.));
        body.close();
        window.paint_path(
            body.build().expect("새 몸체 경로 생성 실패"),
            tint(TEXT, frame.bird_opacity),
        );

        let mut wing = PathBuilder::fill();
        wing.move_to(bird(9., 4.));
        wing.cubic_bezier_to(
            bird(-20., wing_tip),
            bird(-7., -8.),
            bird(-38., wing_tip - 7.),
        );
        wing.cubic_bezier_to(
            bird(-4., wing_tip * 0.48),
            bird(-14., wing_tip + 12.),
            bird(-11., wing_tip + 19.),
        );
        wing.cubic_bezier_to(bird(9., 4.), bird(-3., 8.), bird(5., 12.));
        wing.close();
        window.paint_path(
            wing.build().expect("새 날개 경로 생성 실패"),
            tint(TEXT, frame.bird_opacity * 0.95),
        );
    }
    for (index, feather) in frame
        .feathers
        .iter()
        .enumerate()
        .filter_map(|(i, f)| f.map(|f| (i, f)))
    {
        paint_feather(
            window,
            at(feather.x, feather.y),
            feather.angle,
            feather.opacity * 0.85,
            feather.scale * scale,
            if index % 3 == 1 { ACCENT } else { TEXT },
        );
    }
}

#[derive(IntoElement)]
struct BirdFlightScene {
    phase: f32,
}
impl RenderOnce for BirdFlightScene {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                paint_flight(window, bounds, &FlightFrame::at(self.phase));
            },
        )
        .size_full()
    }
}

#[derive(IntoElement)]
pub struct StartupLoadingScreen;
impl RenderOnce for StartupLoadingScreen {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id("startup-loading-screen")
            .flex()
            .flex_col()
            .flex_1()
            .size_full()
            .min_w_0()
            .min_h_0()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .pb(px(80.))
            .child(
                div()
                    .w_full()
                    .max_w(px(720.))
                    .h(px(300.))
                    .flex_shrink_0()
                    .child(BirdFlightScene { phase: 0. }.with_animation(
                        "startup-bird-flight",
                        Animation::new(Duration::from_millis(7600)).repeat(),
                        |mut scene, phase| {
                            scene.phase = phase;
                            scene
                        },
                    )),
            )
            .child(
                div()
                    .text_size(px(27.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Feather Git"),
            )
            .child(
                div()
                    .mt_3()
                    .text_size(px(13.))
                    .text_color(rgb(MUTED))
                    .child("작업 공간을 불러오고 있습니다"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn flight_has_forward_motion_flapping_and_multiple_falling_feathers() {
        assert!(FlightFrame::at(0.7).bird_x > FlightFrame::at(0.2).bird_x);
        assert!((FlightFrame::at(0.20).wing - FlightFrame::at(0.225).wing).abs() > 0.5);
        let first = FlightFrame::at(0.22).feathers[0].unwrap();
        let later = FlightFrame::at(0.38).feathers[0].unwrap();
        assert!(later.y > first.y);
        assert!((later.angle - first.angle).abs() > 0.05);
        assert!(FlightFrame::at(0.48).feathers.iter().flatten().count() >= 3);
    }

    #[::core::prelude::v1::test]
    fn flight_frames_have_valid_values_and_fade_at_loop_boundary() {
        for step in 0..=1000 {
            let frame = FlightFrame::at(step as f32 / 1000.);
            assert!((0.0..=1.0).contains(&frame.bird_opacity));
            assert!(frame.bird_x.is_finite() && frame.bird_y.is_finite());
            assert!((-1.0..=1.0).contains(&frame.wing));
            for feather in frame.feathers.iter().flatten() {
                assert!((0.0..=1.0).contains(&feather.opacity));
                assert!(feather.angle.is_finite());
                assert!(feather.x > 20. && feather.x < FLIGHT_WIDTH - 20.);
                assert!(feather.y > 20. && feather.y + 20. < FLIGHT_HEIGHT);
            }
        }
        for phase in [0., 1.] {
            let frame = FlightFrame::at(phase);
            assert_eq!(frame.bird_opacity, 0.);
            assert!(frame.feathers.iter().all(Option::is_none));
        }
    }

    #[::core::prelude::v1::test]
    fn ripples_start_at_feather_contact_then_expand_and_disappear() {
        let before = Frame::at(IMPACT - 0.01);
        assert!(before.y + FEATHER_TIP_Y < WATER_Y);
        assert!(before.ripples.iter().all(Option::is_none));
        let contact = Frame::at(IMPACT);
        assert!((contact.y + FEATHER_TIP_Y - WATER_Y).abs() < 0.001);
        let later = Frame::at(IMPACT + 0.2);
        assert!(later.ripples[0].unwrap().radius > contact.ripples[0].unwrap().radius);
        assert!(later.ripples[0].unwrap().opacity < contact.ripples[0].unwrap().opacity);
        assert!(later.ripples.iter().all(Option::is_some));
        for edge in [0., 1.] {
            let frame = Frame::at(edge);
            assert_eq!(frame.opacity, 0.);
            assert!(frame.ripples.iter().flatten().all(|r| r.opacity == 0.));
        }
    }
    #[::core::prelude::v1::test]
    fn all_frames_stay_within_canvas_with_valid_opacity() {
        for step in 0..=280 {
            let frame = Frame::at(step as f32 / 280.);
            assert!((0.0..=1.0).contains(&frame.opacity));
            assert!(frame.x.is_finite() && frame.y.is_finite() && frame.angle.is_finite());
            for ripple in frame.ripples.iter().flatten() {
                assert!((0.0..=1.0).contains(&ripple.opacity));
                assert!(WATER_X - ripple.radius >= 0. && WATER_X + ripple.radius <= 180.);
                assert!(WATER_Y + ripple.radius * 0.26 <= 140.);
            }
        }
    }
}
