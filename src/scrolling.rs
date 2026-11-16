#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Visibility {
    pub vertical: bool,
    pub horizontal: bool,
}

impl Visibility {
    pub fn new(
        width: f32,
        height: f32,
        content_width: f32,
        content_height: f32,
        horizontal: bool,
        bar: f32,
    ) -> Self {
        if [width, height, content_width, content_height, bar]
            .iter()
            .any(|value| !value.is_finite())
            || width <= 0.
            || height <= 0.
        {
            return Self::default();
        }
        let mut visible = Self::default();
        for _ in 0..3 {
            visible.vertical =
                content_height > (height - if visible.horizontal { bar } else { 0. }).max(0.) + 0.5;
            visible.horizontal = horizontal
                && content_width > (width - if visible.vertical { bar } else { 0. }).max(0.) + 0.5;
        }
        visible
    }
}

fn nonnegative(value: f32) -> f32 {
    if value.is_finite() { value.max(0.) } else { 0. }
}

#[derive(Clone, Copy, Debug)]
pub struct Thumb {
    pub start: f32,
    pub length: f32,
    pub travel: f32,
    pub maximum: f32,
}

impl Thumb {
    pub fn new(track: f32, maximum: f32, offset: f32) -> Self {
        let track = nonnegative(track);
        let maximum = nonnegative(maximum);
        let length = if maximum == 0. {
            track
        } else {
            (track * (track / (track + maximum))).max(20.).min(track)
        };
        let travel = (track - length).max(0.);
        let start = if maximum > 0. {
            nonnegative(offset).min(maximum) / maximum * travel
        } else {
            0.
        };
        Self {
            start,
            length,
            travel,
            maximum,
        }
    }

    pub fn offset(self, position: f32) -> f32 {
        if self.travel == 0. {
            0.
        } else {
            (nonnegative(position) / self.travel).min(1.) * self.maximum
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollDrag {
    pub axis: ScrollAxis,
    grab: f32,
}

impl ScrollDrag {
    pub fn start(axis: ScrollAxis, thumb: Thumb, local: f32) -> Option<Self> {
        if !local.is_finite() || thumb.maximum <= 0. || thumb.travel <= 0. {
            return None;
        }
        let grab = if local >= thumb.start && local <= thumb.start + thumb.length {
            local - thumb.start
        } else {
            thumb.length / 2.
        };
        Some(Self { axis, grab })
    }

    pub fn offset(self, thumb: Thumb, local: f32) -> Option<f32> {
        local.is_finite().then(|| thumb.offset(local - self.grab))
    }
}

pub fn wheel_offsets(
    offset: [f32; 2],
    maximum: [f32; 2],
    delta: [f32; 2],
    shift: bool,
    horizontal: bool,
) -> [f32; 2] {
    let maximum = [
        if horizontal {
            nonnegative(maximum[0])
        } else {
            0.
        },
        nonnegative(maximum[1]),
    ];
    let mut delta = delta.map(|value| if value.is_finite() { value } else { 0. });
    if horizontal && shift {
        delta[0] += delta[1];
        delta[1] = 0.;
    }
    std::array::from_fn(|axis| {
        let offset = if offset[axis].is_finite() {
            offset[axis]
        } else {
            0.
        };
        (offset + delta[axis]).clamp(-maximum[axis], 0.)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_visibility_accounts_for_both_axes_and_resets_when_fitting() {
        assert_eq!(
            Visibility::new(300., 200., 300., 200., true, 12.),
            Visibility::default()
        );
        assert_eq!(
            Visibility::new(300., 200., 295., 201., true, 12.),
            Visibility {
                vertical: true,
                horizontal: true
            }
        );
        assert_eq!(
            Visibility::new(300., 200., 301., 195., true, 12.),
            Visibility {
                vertical: true,
                horizontal: true
            }
        );
        assert_eq!(
            Visibility::new(500., 500., 301., 201., true, 12.),
            Visibility::default()
        );
        assert!(!Visibility::new(300., 200., 500., 100., false, 12.).horizontal);
        assert_eq!(
            Visibility::new(0., 0., 100., 100., true, 12.),
            Visibility::default()
        );
        assert_eq!(
            Visibility::new(f32::NAN, 200., 400., 300., true, 12.),
            Visibility::default()
        );
    }

    #[test]
    fn thumb_tracks_offset_and_clamps_short_and_invalid_tracks() {
        let thumb = Thumb::new(300., 900., 450.);
        assert_eq!(thumb.length, 75.);
        assert_eq!(thumb.start, 112.5);
        assert_eq!(thumb.offset(thumb.start), 450.);
        assert_eq!(thumb.offset(-100.), 0.);
        assert_eq!(thumb.offset(1000.), 900.);
        for track in [0., 10., 300.] {
            let thumb = Thumb::new(track, 100_000., 0.);
            assert!(thumb.length >= 0. && thumb.length <= track);
            assert!(thumb.offset(100.).is_finite());
        }
        assert_eq!(Thumb::new(f32::NAN, f32::INFINITY, f32::NAN).length, 0.);
    }

    #[test]
    fn dragging_keeps_grab_and_track_click_centers_the_thumb() {
        let thumb = Thumb::new(300., 900., 450.);
        let drag = ScrollDrag::start(ScrollAxis::Vertical, thumb, thumb.start + 10.).unwrap();
        assert_eq!(drag.offset(thumb, thumb.start + 10.), Some(450.));
        assert_eq!(drag.offset(thumb, thumb.start + 40.), Some(570.));
        assert_eq!(drag.offset(thumb, -100.), Some(0.));
        assert_eq!(drag.offset(thumb, 1000.), Some(900.));
        assert_eq!(drag.offset(thumb, f32::NAN), None);
        let clicked = ScrollDrag::start(ScrollAxis::Horizontal, thumb, 250.).unwrap();
        assert_eq!(clicked.offset(thumb, 250.), Some(850.));
        let reduced = Thumb::new(300., 30., 0.);
        assert_eq!(drag.offset(reduced, 1000.), Some(30.));
        assert!(ScrollDrag::start(ScrollAxis::Vertical, Thumb::new(300., 0., 0.), 100.).is_none());
    }

    #[test]
    fn vertical_wheel_at_boundaries_never_changes_horizontal_offset() {
        for y in [0., -900.] {
            assert_eq!(
                wheel_offsets(
                    [-30., y],
                    [300., 900.],
                    [0., if y == 0. { 48. } else { -48. }],
                    false,
                    true
                ),
                [-30., y]
            );
        }
        assert_eq!(
            wheel_offsets([-30., -900.], [300., 900.], [0., -48.], true, true),
            [-78., -900.]
        );
        assert_eq!(
            wheel_offsets([-30., -900.], [300., 900.], [-48., 0.], false, true),
            [-78., -900.]
        );
    }

    #[test]
    fn wheel_shift_and_touchpad_deltas_are_applied_once_and_clamped() {
        assert_eq!(
            wheel_offsets([0., 0.], [300., 900.], [0., -48.], false, true),
            [0., -48.]
        );
        assert_eq!(
            wheel_offsets([0., 0.], [300., 900.], [0., -48.], true, true),
            [-48., 0.]
        );
        assert_eq!(
            wheel_offsets([0., 0.], [300., 900.], [-5., -40.], false, true),
            [-5., -40.]
        );
        assert_eq!(
            wheel_offsets([-290., -890.], [300., 900.], [-100., -100.], false, true),
            [-300., -900.]
        );
        assert_eq!(
            wheel_offsets([-50., -50.], [300., 900.], [100., 100.], false, true),
            [0., 0.]
        );
        assert_eq!(
            wheel_offsets([0., 0.], [300., 900.], [0., -48.], true, false),
            [0., -48.]
        );
        assert_eq!(
            wheel_offsets([0., 0.], [300., 0.], [0., -48.], false, true),
            [0., 0.]
        );
        assert_eq!(
            wheel_offsets([0., 0.], [0., 0.], [f32::NAN, f32::INFINITY], false, false),
            [0., 0.]
        );
    }
}
