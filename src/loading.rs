use std::time::{Duration, Instant};

/// 빠른 응답의 깜빡임을 피하기 위한 공통 로딩 표시 지연입니다.
pub const LOADING_DELAY: Duration = Duration::from_millis(150);

#[derive(Default)]
pub struct DelayedLoading {
    request: Option<(u64, Instant)>,
    visible: bool,
}
impl DelayedLoading {
    pub fn begin(&mut self, revision: u64, now: Instant) {
        self.request = Some((revision, now));
        self.visible = false;
    }
    pub fn reset(&mut self) {
        self.request = None;
        self.visible = false;
    }
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.request.map(|(_, started)| {
            LOADING_DELAY.saturating_sub(now.saturating_duration_since(started))
        })
    }
    pub fn reveal(&mut self, revision: u64, now: Instant) -> bool {
        let Some((current, started)) = self.request else {
            return false;
        };
        if current != revision
            || self.visible
            || now.saturating_duration_since(started) < LOADING_DELAY
        {
            return false;
        }
        self.visible = true;
        true
    }
    pub fn is_visible(&self, revision: u64) -> bool {
        self.visible && self.request.is_some_and(|(current, _)| current == revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_delay_reveals_at_150ms_but_not_at_149ms() {
        assert_eq!(LOADING_DELAY, Duration::from_millis(150));
        let start = Instant::now();
        let mut loading = DelayedLoading::default();
        loading.begin(1, start);
        assert_eq!(loading.remaining(start), Some(LOADING_DELAY));
        assert!(!loading.reveal(1, start + Duration::from_millis(149)));
        assert!(!loading.is_visible(1));
        assert_eq!(
            loading.remaining(start + Duration::from_millis(149)),
            Some(Duration::from_millis(1))
        );
        assert!(loading.reveal(1, start + LOADING_DELAY));
        assert!(loading.is_visible(1));
        assert!(!loading.reveal(1, start + LOADING_DELAY));
        assert_eq!(
            loading.remaining(start + Duration::from_secs(1)),
            Some(Duration::ZERO)
        );
    }
    #[test]
    fn completion_and_new_request_invalidate_old_timers() {
        let start = Instant::now();
        let mut loading = DelayedLoading::default();
        loading.begin(1, start);
        loading.reset();
        assert!(!loading.reveal(1, start + LOADING_DELAY));
        assert_eq!(loading.remaining(start), None);
        loading.begin(2, start);
        assert!(!loading.reveal(1, start + LOADING_DELAY));
        assert!(loading.reveal(2, start + LOADING_DELAY));
        loading.begin(3, start + LOADING_DELAY);
        assert!(!loading.is_visible(2) && !loading.is_visible(3));
        assert!(!loading.reveal(2, start + LOADING_DELAY + LOADING_DELAY));
        assert!(loading.reveal(3, start + LOADING_DELAY + LOADING_DELAY));
    }
}
