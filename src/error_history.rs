use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorEntry {
    pub source: String,
    pub message: String,
}

#[derive(Default)]
pub struct ErrorHistory {
    entries: Vec<ErrorEntry>,
    observed: HashMap<String, String>,
    page: usize,
}

impl ErrorHistory {
    pub fn mirror(&mut self, key: impl Into<String>, error: Option<String>) -> Option<String> {
        let key = key.into();
        if let Some(message) = &error {
            self.observed.insert(key, message.clone());
        } else {
            self.observed.remove(&key);
        }
        error
    }

    pub fn report(
        &mut self,
        key: impl Into<String>,
        source: impl Into<String>,
        error: Option<String>,
    ) -> Option<String> {
        if let Some(message) = &error {
            self.entries.push(ErrorEntry {
                source: source.into(),
                message: message.clone(),
            });
            self.observed.insert(key.into(), message.clone());
        }
        error
    }

    pub fn observe(&mut self, errors: Vec<(String, String, String)>) {
        let mut current = HashMap::new();
        for (key, source, message) in errors {
            if self.observed.get(&key) != Some(&message) {
                self.entries.push(ErrorEntry {
                    source,
                    message: message.clone(),
                });
            }
            current.insert(key, message);
        }
        self.observed = current;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn current(&self) -> Option<&ErrorEntry> {
        self.entries.get(self.page)
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn open(&mut self) {
        self.page = self.entries.len().saturating_sub(1);
    }

    pub fn acknowledge(&mut self) {
        if self.page < self.entries.len() {
            self.entries.remove(self.page);
            self.page = self.page.min(self.entries.len().saturating_sub(1));
        }
    }

    pub fn previous(&mut self) {
        self.page = self.page.saturating_sub(1);
    }

    pub fn next(&mut self) {
        if self.page + 1 < self.entries.len() {
            self.page += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_failure_mirrored_to_a_dialog_stays_one_record_and_retry_is_another() {
        let mut history = ErrorHistory::default();
        let snapshot = || {
            vec![
                error("workspace", "revert failed"),
                error("branch-dialog", "revert failed"),
            ]
        };
        for expected in [1, 2] {
            let result = history.report("workspace", "app", Some("revert failed".into()));
            assert_eq!(
                history.mirror("branch-dialog", result).as_deref(),
                Some("revert failed")
            );
            history.observe(snapshot());
            history.observe(snapshot());
            assert_eq!(history.len(), expected);
        }
        history.open();
        history.acknowledge();
        history.observe(snapshot());
        assert_eq!(history.len(), 1);
        let result = history.report("workspace", "app", Some("revert failed".into()));
        history.mirror("branch-dialog", result);
        history.observe(snapshot());
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn each_report_is_a_new_occurrence_but_observation_and_acknowledgement_do_not_repeat_it() {
        let mut history = ErrorHistory::default();
        for _ in 0..2 {
            let error = history.report("commit", "커밋", Some("같은 실패".into()));
            assert_eq!(error.as_deref(), Some("같은 실패"));
            history.observe(vec![("commit".into(), "커밋".into(), "같은 실패".into())]);
        }
        assert_eq!(history.len(), 2);
        history.acknowledge();
        history.observe(vec![("commit".into(), "커밋".into(), "같은 실패".into())]);
        assert_eq!(history.len(), 1);
        history.report("commit", "커밋", Some("같은 실패".into()));
        assert_eq!(history.len(), 2);
        assert_eq!(history.page(), 0);
        history.report("checkout", "체크아웃", Some("같은 실패".into()));
        assert_eq!(history.len(), 3);
        assert_eq!(history.page(), 0);
        assert!(history.report("commit", "커밋", None).is_none());
        assert_eq!(history.len(), 3);
    }

    fn error(key: &str, message: &str) -> (String, String, String) {
        (key.into(), key.into(), message.into())
    }

    #[test]
    fn keeps_occurrence_order_without_render_or_tab_duplicates_and_keeps_closed_errors() {
        let mut history = ErrorHistory::default();
        history.observe(vec![error("tab-b", "두 번째 탭 오류")]);
        history.observe(vec![error("tab-b", "두 번째 탭 오류")]);
        history.observe(vec![
            error("tab-a", "첫 번째 탭 오류"),
            error("tab-b", "두 번째 탭 오류"),
        ]);
        assert_eq!(history.len(), 2);
        assert_eq!(history.current().unwrap().source, "tab-b");
        history.observe(vec![]);
        assert_eq!(history.len(), 2);
        history.next();
        assert_eq!(history.current().unwrap().source, "tab-a");
    }

    #[test]
    fn records_retry_and_changed_messages_but_preserves_current_page() {
        let mut history = ErrorHistory::default();
        history.observe(vec![error("clone", "실패")]);
        history.observe(vec![]);
        history.observe(vec![error("clone", "실패")]);
        history.next();
        history.observe(vec![error("clone", "다른 실패")]);
        assert_eq!(history.len(), 3);
        assert_eq!(history.page(), 1);
        assert_eq!(history.current().unwrap().message, "실패");
        history.next();
        assert_eq!(history.current().unwrap().message, "다른 실패");
    }

    #[test]
    fn pages_have_safe_boundaries_and_reopen_from_latest() {
        let mut history = ErrorHistory::default();
        history.previous();
        history.next();
        assert!(history.current().is_none());
        history.observe(vec![error("a", "a"), error("b", "b")]);
        history.previous();
        assert_eq!(history.page(), 0);
        history.next();
        history.next();
        assert_eq!(history.page(), 1);
        history.open();
        assert_eq!(history.page(), 1);
    }

    #[test]
    fn acknowledgement_removes_only_current_and_selects_next_or_previous_at_end() {
        let mut history = ErrorHistory::default();
        history.acknowledge();
        history.observe(vec![error("a", "a"), error("b", "b"), error("c", "c")]);
        history.next();
        history.acknowledge();
        assert_eq!(history.len(), 2);
        assert_eq!(history.current().unwrap().message, "c");
        assert_eq!(history.page(), 1);
        history.acknowledge();
        assert_eq!(history.current().unwrap().message, "a");
        assert_eq!(history.page(), 0);
        history.acknowledge();
        assert!(history.current().is_none());
        assert_eq!(history.page(), 0);
    }

    #[test]
    fn acknowledged_error_stays_removed_until_source_clears_or_changes() {
        let mut history = ErrorHistory::default();
        history.observe(vec![error("a", "failure")]);
        history.acknowledge();
        history.observe(vec![error("a", "failure")]);
        assert_eq!(history.len(), 0);
        history.observe(vec![]);
        history.observe(vec![error("a", "failure")]);
        assert_eq!(history.len(), 1);
        history.acknowledge();
        history.observe(vec![error("a", "new failure")]);
        assert_eq!(history.len(), 1);
        history.open();
        assert_eq!(history.current().unwrap().message, "new failure");
    }
}
