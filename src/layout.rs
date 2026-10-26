const MIN_LEFT: f32 = 260.;
const MIN_RIGHT: f32 = 360.;
const MIN_ROW: f32 = 180.;

#[derive(Clone, Copy)]
pub enum SplitAxis {
    Columns,
    Rows,
}

pub struct SplitLayout {
    columns: f32,
    rows: f32,
}

impl Default for SplitLayout {
    fn default() -> Self {
        Self {
            columns: 0.3,
            rows: 0.5,
        }
    }
}

impl SplitLayout {
    pub fn ratio(&self, axis: SplitAxis, extent: f32) -> f32 {
        let (ratio, before, after) = match axis {
            SplitAxis::Columns => (self.columns, MIN_LEFT, MIN_RIGHT),
            SplitAxis::Rows => (self.rows, MIN_ROW, MIN_ROW),
        };
        if !extent.is_finite() || extent <= 0. {
            return ratio;
        }
        // 최소 크기보다 작은 공간에서도 두 영역의 합이 가용 공간을 넘지 않는다.
        let scale = (extent / (before + after)).min(1.);
        let lower = before * scale / extent;
        let upper = (1. - after * scale / extent).max(lower);
        ratio.clamp(lower, upper)
    }

    pub fn resize(&mut self, axis: SplitAxis, offset: f32, extent: f32) {
        if !offset.is_finite() || !extent.is_finite() || extent <= 0. {
            return;
        }
        match axis {
            SplitAxis::Columns => self.columns = offset / extent,
            SplitAxis::Rows => self.rows = offset / extent,
        }
        let constrained = self.ratio(axis, extent);
        match axis {
            SplitAxis::Columns => self.columns = constrained,
            SplitAxis::Rows => self.rows = constrained,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_outside_the_body_preserves_minimum_sizes() {
        let mut layout = SplitLayout::default();
        layout.resize(SplitAxis::Columns, -100., 1000.);
        assert!((layout.ratio(SplitAxis::Columns, 1000.) * 1000. - 260.).abs() < 0.01);
        layout.resize(SplitAxis::Columns, 2000., 1000.);
        assert!(((1. - layout.ratio(SplitAxis::Columns, 1000.)) * 1000. - 360.).abs() < 0.01);
        for offset in [-500., 2000.] {
            layout.resize(SplitAxis::Rows, offset, 600.);
            let top = layout.ratio(SplitAxis::Rows, 600.) * 600.;
            assert!((180. - 0.01..=420. + 0.01).contains(&top));
        }
    }

    #[test]
    fn window_resize_preserves_preferred_ratio_and_fits_small_spaces() {
        let mut layout = SplitLayout::default();
        layout.resize(SplitAxis::Columns, 400., 1600.);
        assert!((layout.ratio(SplitAxis::Columns, 1000.) - 0.26).abs() < 0.001);
        assert!((layout.ratio(SplitAxis::Columns, 1600.) - 0.25).abs() < 0.001);
        let ratio = layout.ratio(SplitAxis::Columns, 300.);
        assert!((ratio - 260. / 620.).abs() < 0.001);
        assert!((layout.ratio(SplitAxis::Rows, 100.) - 0.5).abs() < 0.001);
    }

    #[test]
    fn axes_are_independent_and_invalid_measurements_are_ignored() {
        let mut layout = SplitLayout::default();
        layout.resize(SplitAxis::Rows, 240., 600.);
        assert!((layout.ratio(SplitAxis::Columns, 1200.) - 0.3).abs() < 0.001);
        assert!((layout.ratio(SplitAxis::Rows, 600.) - 0.4).abs() < 0.001);
        layout.resize(SplitAxis::Rows, f32::NAN, 600.);
        layout.resize(SplitAxis::Rows, 10., 0.);
        assert!((layout.ratio(SplitAxis::Rows, 600.) - 0.4).abs() < 0.001);
    }
}
