pub const DIVIDER_WIDTH: f32 = 6.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelLayout {
    left: f32,
    top: Option<f32>,
}
impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            left: 310.,
            top: None,
        }
    }
}
fn bounded(desired: f32, total: f32, first: f32, second: f32) -> f32 {
    if !total.is_finite() || total <= 0. {
        return desired;
    }
    if total < first + second {
        return total * first / (first + second);
    }
    desired.clamp(first, total - second)
}
impl PanelLayout {
    pub fn split(&self, total: [f32; 2]) -> [f32; 2] {
        [
            bounded(self.left, total[0], 200., 320.),
            bounded(
                self.top.unwrap_or((total[1] * 0.54).round()),
                total[1],
                160.,
                160.,
            ),
        ]
    }
    pub fn resize(
        &mut self,
        axis: usize,
        start: [f32; 2],
        delta: [f32; 2],
        total: [f32; 2],
    ) -> bool {
        if axis > 2
            || start
                .iter()
                .chain(delta.iter())
                .chain(total.iter())
                .any(|v| !v.is_finite())
            || total.iter().any(|v| *v <= 0.)
        {
            return false;
        }
        let mut next = *self;
        if axis != 1 {
            next.left = bounded(start[0] + delta[0], total[0], 200., 320.);
        }
        if axis != 0 {
            next.top = Some(bounded(start[1] + delta[1], total[1], 160., 160.));
        }
        if next == *self {
            return false;
        }
        *self = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axes_and_center_resize_shared_cross() {
        let mut layout = PanelLayout::default();
        let total = [1200., 800.];
        let start = layout.split(total);
        assert!(layout.resize(0, start, [100., 50.], total));
        assert_eq!(layout.split(total), [410., start[1]]);
        let start = layout.split(total);
        assert!(layout.resize(1, start, [100., 50.], total));
        assert_eq!(layout.split(total), [410., start[1] + 50.]);
        let start = layout.split(total);
        assert!(layout.resize(2, start, [40., -20.], total));
        assert_eq!(layout.split(total), [450., start[1] - 20.]);
    }
    #[test]
    fn minimums_small_windows_and_invalid_inputs_are_safe() {
        let mut layout = PanelLayout::default();
        layout.resize(2, [310., 400.], [10000., -10000.], [1200., 800.]);
        assert_eq!(layout.split([1200., 800.]), [880., 160.]);
        layout.resize(2, [310., 400.], [-10000., 10000.], [1200., 800.]);
        assert_eq!(layout.split([1200., 800.]), [200., 640.]);
        for total in [[100., 100.], [300., 200.], [520., 320.]] {
            let split = layout.split(total);
            assert!(
                split
                    .iter()
                    .zip(total)
                    .all(|(value, total)| *value >= 0. && *value <= total)
            );
        }
        let before = layout;
        assert!(!layout.resize(3, [310., 400.], [1., 1.], [1200., 800.]));
        assert!(!layout.resize(2, [310., 400.], [f32::NAN, 1.], [1200., 800.]));
        assert_eq!(layout, before);
    }
}
