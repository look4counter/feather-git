/// 줄바꿈/말줄임 측정 결과의 재사용 가능 여부를 판정한다.
pub(super) fn can_reuse_layout<T: PartialEq>(
    cached_wrap_width: Option<T>,
    cached_truncate_width: Option<T>,
    wrap_width: Option<T>,
    truncate_width: Option<T>,
) -> bool {
    wrap_width == cached_wrap_width && truncate_width == cached_truncate_width
}

#[cfg(test)]
mod tests {
    use super::can_reuse_layout;

    #[test]
    fn no_wrap_text_remeasures_when_truncation_width_grows_or_shrinks() {
        for (previous, current) in [(0, 320), (24, 320), (320, 24)] {
            assert!(
                !can_reuse_layout(None, Some(previous), None, Some(current)),
                "말줄임 폭 {previous}에서 {current}로 바뀌면 다시 측정해야 한다"
            );
        }
    }

    #[test]
    fn unchanged_constraints_reuse_layout_including_zero_and_unbounded_widths() {
        assert!(can_reuse_layout::<u32>(None, None, None, None));
        assert!(can_reuse_layout(None, Some(0), None, Some(0)));
        assert!(can_reuse_layout(None, Some(320), None, Some(320)));
        assert!(can_reuse_layout(Some(320), None, Some(320), None));
        assert!(can_reuse_layout(Some(320), Some(640), Some(320), Some(640)));
    }

    #[test]
    fn adding_or_removing_truncation_limit_requires_measurement() {
        assert!(!can_reuse_layout(None, None, None, Some(320)));
        assert!(!can_reuse_layout(None, Some(320), None, None));
        assert!(!can_reuse_layout(Some(320), None, Some(320), Some(320)));
        assert!(!can_reuse_layout(Some(320), Some(320), Some(320), None));
    }

    #[test]
    fn wrapping_width_or_mode_change_requires_measurement() {
        assert!(!can_reuse_layout(Some(120), None, Some(320), None));
        assert!(!can_reuse_layout(Some(320), None, None, None));
        assert!(!can_reuse_layout(None, None, Some(320), None));
        assert!(!can_reuse_layout(
            Some(320),
            Some(640),
            Some(320),
            Some(320)
        ));
    }
}
