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
        if width <= 0. || height <= 0. {
            return Self::default();
        }
        let mut visible = Self::default();
        // 바가 없는 상태에서 시작해 다른 축이 차지하는 공간을 반영한다.
        for _ in 0..3 {
            visible.vertical =
                content_height > (height - if visible.horizontal { bar } else { 0. }).max(0.) + 0.5;
            visible.horizontal = horizontal
                && content_width > (width - if visible.vertical { bar } else { 0. }).max(0.) + 0.5;
        }
        visible
    }
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
        let track = track.max(0.);
        let maximum = maximum.max(0.);
        let length = if maximum == 0. {
            track
        } else {
            (track * track / (track + maximum)).max(20.).min(track)
        };
        let travel = (track - length).max(0.);
        let start = if maximum > 0. {
            offset.clamp(0., maximum) / maximum * travel
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
            (position / self.travel).clamp(0., 1.) * self.maximum
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrollbars_only_appear_for_overflow_and_recompute_without_sticky_state() {
        let fit = Visibility::new(300., 200., 300., 200., true, 12.);
        assert_eq!(fit, Visibility::default());
        assert_eq!(
            Visibility::new(300., 200., 100., 201., true, 12.),
            Visibility {
                vertical: true,
                horizontal: false
            }
        );
        assert_eq!(
            Visibility::new(300., 200., 301., 100., true, 12.),
            Visibility {
                vertical: false,
                horizontal: true
            }
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
    }
    #[test]
    fn thumb_maps_track_to_offset_and_clamps_dragging() {
        let thumb = Thumb::new(300., 900., 450.);
        assert_eq!(thumb.length, 75.);
        assert_eq!(thumb.start, 112.5);
        assert_eq!(thumb.offset(thumb.start), 450.);
        assert_eq!(thumb.offset(-100.), 0.);
        assert_eq!(thumb.offset(1000.), 900.);
    }
    #[test]
    fn empty_and_short_tracks_have_no_invalid_travel() {
        for track in [0., 10., 300.] {
            let empty = Thumb::new(track, 0., 0.);
            assert_eq!(empty.length, track);
            assert_eq!(empty.offset(100.), 0.);
            let long = Thumb::new(track, 100000., 0.);
            assert!(long.length <= track && long.length >= 0.);
        }
    }
}
