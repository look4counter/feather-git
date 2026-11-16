use crate::{
    graph::{GraphBuilder, GraphCommit},
    history_graph::RowGraph,
    repository::{Commit, Project, ReferenceKind},
    virtual_list::ListScrollHandle,
};
use gpui::{ScrollHandle, SharedString, UniformListScrollHandle};
use std::{ops::Range, rc::Rc};

#[derive(Clone)]
pub struct PreparedReference {
    pub label: SharedString,
    pub kind: ReferenceKind,
}

#[derive(Clone)]
pub struct PreparedHistoryRow {
    pub index: usize,
    pub graph: RowGraph,
    pub title: SharedString,
    pub author: SharedString,
    pub date: SharedString,
    pub hash: SharedString,
    pub reference: SharedString,
    pub references: Rc<Vec<PreparedReference>>,
    pub checked_out: bool,
}

pub struct PreparedHistory {
    builder: GraphBuilder,
    layouts: Vec<GraphCommit>,
    lanes: usize,
}
impl PreparedHistory {
    pub fn empty() -> Self {
        Self {
            builder: GraphBuilder::default(),
            layouts: Vec::new(),
            lanes: 1,
        }
    }
    pub fn without_graph(commits: &[Commit]) -> Self {
        Self {
            builder: GraphBuilder::default(),
            layouts: commits
                .iter()
                .map(|commit| GraphCommit {
                    id: commit.id.clone(),
                    ..Default::default()
                })
                .collect(),
            lanes: 1,
        }
    }

    pub fn new(commits: &[Commit]) -> Self {
        Self::with_builder(commits, GraphBuilder::default())
    }
    pub fn with_builder(commits: &[Commit], mut builder: GraphBuilder) -> Self {
        let mut layouts: Vec<_> = commits
            .iter()
            .map(|c| GraphCommit {
                id: c.id.clone(),
                parent_ids: c.parents.clone(),
                ..Default::default()
            })
            .collect();
        builder.append(&mut layouts);
        let lanes = layouts
            .iter()
            .flat_map(|c| {
                c.input_edges
                    .iter()
                    .chain(&c.output_edges)
                    .flat_map(|e| [e.from_lane, e.to_lane])
                    .chain(std::iter::once(c.marker.lane))
            })
            .max()
            .unwrap_or(0)
            + 1;
        Self {
            layouts,
            lanes,
            builder,
        }
    }
}

pub struct HistoryState {
    builder: GraphBuilder,
    pub append_error: Option<String>,
    layouts: Vec<GraphCommit>,
    rows: Vec<Option<PreparedHistoryRow>>,
    pub lanes: usize,
    pub scroll: UniformListScrollHandle,
    pub horizontal_scroll: ScrollHandle,
    pub scroll_frame: ListScrollHandle,
}
impl HistoryState {
    #[cfg(test)]
    pub fn new(commits: &[Commit]) -> Self {
        Self::from_prepared(PreparedHistory::new(commits))
    }
    pub fn from_prepared(prepared: PreparedHistory) -> Self {
        let PreparedHistory {
            layouts,
            lanes,
            builder,
        } = prepared;
        let scroll_frame = ListScrollHandle::default();
        Self {
            builder,
            append_error: None,
            rows: vec![None; layouts.len()],
            layouts,
            lanes,
            scroll: scroll_frame.vertical.clone(),
            horizontal_scroll: scroll_frame.horizontal.clone(),
            scroll_frame,
        }
    }
    pub fn graph_builder(&self) -> GraphBuilder {
        self.builder.clone()
    }
    pub fn append(&mut self, prepared: PreparedHistory) {
        self.rows.extend((0..prepared.layouts.len()).map(|_| None));
        self.layouts.extend(prepared.layouts);
        self.lanes = self.lanes.max(prepared.lanes);
        self.builder = prepared.builder;
        self.append_error = None;
    }
    pub fn len(&self) -> usize {
        self.layouts.len()
    }
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
    pub fn prepare_rows(
        &mut self,
        project: &Project,
        range: Range<usize>,
    ) -> Vec<PreparedHistoryRow> {
        let count = self.len().min(project.commits.len());
        (range.start.min(count)..range.end.min(count))
            .map(|index| {
                self.rows[index]
                    .get_or_insert_with(|| {
                        let commit = &project.commits[index];
                        PreparedHistoryRow {
                            index,
                            graph: RowGraph::new(&self.layouts[index]),
                            title: commit
                                .message
                                .lines()
                                .next()
                                .unwrap_or_default()
                                .to_owned()
                                .into(),
                            author: commit.author.clone().into(),
                            date: commit.date.clone().into(),
                            hash: commit.id.chars().take(8).collect::<String>().into(),
                            reference: commit.reference.clone().into(),
                            references: Rc::new({
                                let mut references: Vec<PreparedReference> =
                                    if commit.references.is_empty() {
                                        if commit.reference.is_empty() {
                                            Vec::new()
                                        } else {
                                            vec![PreparedReference {
                                                label: commit.reference.clone().into(),
                                                kind: ReferenceKind::Other,
                                            }]
                                        }
                                    } else {
                                        commit
                                            .references
                                            .iter()
                                            .map(|reference| PreparedReference {
                                                label: reference.label.clone().into(),
                                                kind: reference.kind(),
                                            })
                                            .collect()
                                    };
                                references.sort_by_key(|reference| match reference.kind {
                                    ReferenceKind::Tag => 0,
                                    ReferenceKind::Local => 1,
                                    ReferenceKind::Remote => 2,
                                    ReferenceKind::Other => 3,
                                });
                                references
                            }),
                            checked_out: project.head.as_deref() == Some(commit.id.as_str()),
                        }
                    })
                    .clone()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_scroll_frame_preserves_legacy_vertical_and_horizontal_handles() {
        let project = project(2);
        let state = HistoryState::new(&project.commits);
        assert!(Rc::ptr_eq(&state.scroll.0, &state.scroll_frame.vertical.0));
        state
            .scroll_frame
            .horizontal
            .set_offset(gpui::point(gpui::px(-75.), gpui::px(0.)));
        assert_eq!(state.horizontal_scroll.offset().x, gpui::px(-75.));
        state.scroll.scroll_to_item(1, gpui::ScrollStrategy::Center);
        assert_eq!(
            state
                .scroll_frame
                .vertical
                .0
                .borrow()
                .deferred_scroll_to_item
                .as_ref()
                .unwrap()
                .item_index,
            1
        );
    }

    #[test]
    fn prepared_references_preserve_kinds_names_and_cached_storage() {
        let mut project = project(1);
        project.commits[0].references = vec![
            crate::repository::CommitReference {
                full_name: "refs/remotes/origin/feature/login,extra".into(),
                name: "origin/feature/login,extra".into(),
                label: "feature/login,extra".into(),
                is_remote: true,
            },
            crate::repository::CommitReference {
                full_name: "refs/heads/origin/local".into(),
                name: "origin/local".into(),
                label: "origin/local".into(),
                is_remote: false,
            },
            crate::repository::CommitReference {
                full_name: "refs/tags/release".into(),
                name: "release".into(),
                label: "release".into(),
                is_remote: false,
            },
        ];
        let mut state = HistoryState::new(&project.commits);
        let first = state.prepare_rows(&project, 0..1);
        assert_eq!(first[0].references.len(), 3);
        assert_eq!(first[0].references[0].kind, ReferenceKind::Tag);
        assert_eq!(first[0].references[0].label.as_ref(), "release");
        assert_eq!(first[0].references[1].kind, ReferenceKind::Local);
        assert_eq!(first[0].references[1].label.as_ref(), "origin/local");
        assert_eq!(first[0].references[2].kind, ReferenceKind::Remote);
        assert_eq!(first[0].references[2].label.as_ref(), "feature/login,extra");
        let second = state.prepare_rows(&project, 0..1);
        assert!(std::rc::Rc::ptr_eq(
            &first[0].references,
            &second[0].references
        ));
    }

    #[test]
    fn legacy_reference_remains_unsplit_and_empty_has_no_badge() {
        let mut project = project(2);
        project.commits[0].reference = "origin/topic,extra".into();
        let mut state = HistoryState::new(&project.commits);
        let rows = state.prepare_rows(&project, 0..2);
        assert_eq!(rows[0].references.len(), 1);
        assert_eq!(rows[0].references[0].label.as_ref(), "origin/topic,extra");
        assert_eq!(rows[0].references[0].kind, ReferenceKind::Other);
        assert!(rows[1].references.is_empty());
    }

    fn project(count: usize) -> Project {
        Project {
            name: "fixture".into(),
            commits: (0..count)
                .map(|i| Commit {
                    id: format!("{i:040x}"),
                    parents: if i + 1 < count {
                        vec![format!("{:040x}", i + 1)]
                    } else {
                        vec![]
                    },
                    message: format!("commit {i}\nfull message"),
                    author: "Tester".into(),
                    date: "2026-11-12".into(),
                    ..Default::default()
                })
                .collect(),
            head: Some(format!("{:040x}", 0)),
            ..Default::default()
        }
    }
    #[test]
    fn prepares_only_requested_rows_and_reuses_them_after_measurement() {
        let project = project(10_000);
        let mut state = HistoryState::new(&project.commits);
        assert!(state.rows.iter().all(Option::is_none));
        let first = state.prepare_rows(&project, 100..125);
        assert_eq!(first.len(), 25);
        assert_eq!(first[0].index, 100);
        state.prepare_rows(&project, 0..1);
        let second = state.prepare_rows(&project, 100..125);
        assert_eq!(first[0].title.as_ptr(), second[0].title.as_ptr());
        assert_eq!(state.rows.iter().filter(|r| r.is_some()).count(), 26);
        assert!(
            state
                .prepare_rows(&project, usize::MAX..usize::MAX)
                .is_empty()
        );
        assert_eq!(state.prepare_rows(&project, 9999..10010).len(), 1);
    }
    #[test]
    fn row_preserves_head_summary_hash_and_reference() {
        let mut project = project(2);
        project.commits[0].reference = "main".into();
        let mut state = HistoryState::new(&project.commits);
        let rows = state.prepare_rows(&project, 0..2);
        assert!(rows[0].checked_out);
        assert!(!rows[1].checked_out);
        assert_eq!(rows[0].title.as_ref(), "commit 0");
        assert_eq!(rows[0].reference.as_ref(), "main");
        assert_eq!(rows[0].hash.len(), 8);
        assert_eq!(state.lanes, 1);
    }
    #[test]
    fn empty_history_prepares_nothing() {
        let project = project(0);
        let mut state = HistoryState::new(&project.commits);
        assert!(state.is_empty());
        assert!(state.prepare_rows(&project, 0..100).is_empty());
    }
    #[test]
    #[ignore = "manual performance measurement"]
    fn benchmark_history_row_preparation() {
        for count in [200, 10_000] {
            let project = project(count);
            let mut full = HistoryState::new(&project.commits);
            let start = std::time::Instant::now();
            std::hint::black_box(full.prepare_rows(&project, 0..count));
            let full_time = start.elapsed();
            let mut visible = HistoryState::new(&project.commits);
            let start = std::time::Instant::now();
            std::hint::black_box(visible.prepare_rows(&project, 0..25));
            let visible_time = start.elapsed();
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                std::hint::black_box(visible.prepare_rows(&project, 0..25));
            }
            eprintln!(
                "benchmark_history_row_preparation: rows={count}, cold_all={}us, cold_visible_25={}us, 1000_cached_visible={}us",
                full_time.as_micros(),
                visible_time.as_micros(),
                start.elapsed().as_micros()
            );
        }
    }
}

#[cfg(test)]
mod pagination_tests {
    use super::*;
    #[test]
    fn appended_graph_matches_full_graph_and_preserves_cached_rows_and_scroll() {
        let commits: Vec<_> = (0..1905)
            .map(|n| Commit {
                id: format!("c{n}"),
                parents: if n == 1498 {
                    vec!["c1499".into(), "c1701".into()]
                } else if n < 1904 {
                    vec![format!("c{}", n + 1)]
                } else {
                    vec![]
                },
                message: format!("row {n}"),
                ..Default::default()
            })
            .collect();
        let full = PreparedHistory::new(&commits);
        let mut project = Project {
            commits: commits[..1500].to_vec(),
            ..Default::default()
        };
        let mut state = HistoryState::new(&project.commits);
        let rows = state.prepare_rows(&project, 1490..1500);
        let cached = rows[0].references.clone();
        let vertical = state.scroll.clone();
        state
            .horizontal_scroll
            .set_offset(gpui::point(gpui::px(-75.), gpui::px(0.)));
        state
            .scroll
            .scroll_to_item(1490, gpui::ScrollStrategy::Center);
        for range in [1500..1700, 1700..1900, 1900..1905] {
            let prepared =
                PreparedHistory::with_builder(&commits[range.clone()], state.graph_builder());
            project.commits.extend_from_slice(&commits[range]);
            state.append(prepared);
        }
        assert_eq!(state.len(), 1905);
        assert_eq!(
            format!("{:?}", state.layouts),
            format!("{:?}", full.layouts)
        );
        assert_eq!(state.lanes, full.lanes);
        assert!(Rc::ptr_eq(
            &cached,
            &state.prepare_rows(&project, 1490..1491)[0].references
        ));
        assert!(Rc::ptr_eq(&vertical.0, &state.scroll.0));
        assert_eq!(state.horizontal_scroll.offset().x, gpui::px(-75.));
        assert_eq!(
            state
                .scroll
                .0
                .borrow()
                .deferred_scroll_to_item
                .as_ref()
                .unwrap()
                .item_index,
            1490
        );
    }
}

#[cfg(test)]
mod reference_order_tests {
    use super::*;
    #[test]
    fn mixed_reference_types_are_stable_and_cached_across_appended_rows() {
        let references = [
            ("refs/remotes/origin/r1", "r1", true),
            ("refs/heads/l1", "l1", false),
            ("refs/tags/t1", "t1", false),
            ("refs/heads/l2", "l2", false),
            ("refs/tags/t2", "t2", false),
            ("refs/remotes/origin/r2", "r2", true),
            ("refs/notes/other", "other", false),
        ]
        .into_iter()
        .map(
            |(full_name, label, is_remote)| crate::repository::CommitReference {
                full_name: full_name.into(),
                name: label.into(),
                label: label.into(),
                is_remote,
            },
        )
        .collect();
        let commit = Commit {
            id: "first".into(),
            references,
            ..Default::default()
        };
        let mut project = Project {
            commits: vec![commit.clone()],
            ..Default::default()
        };
        let mut state = HistoryState::new(&project.commits);
        let rows = state.prepare_rows(&project, 0..1);
        assert_eq!(
            rows[0]
                .references
                .iter()
                .map(|r| r.label.as_ref())
                .collect::<Vec<_>>(),
            ["t1", "t2", "l1", "l2", "r1", "r2", "other"]
        );
        let mut next = commit;
        next.id = "next".into();
        let prepared =
            PreparedHistory::with_builder(std::slice::from_ref(&next), state.graph_builder());
        project.commits.push(next);
        state.append(prepared);
        let appended = state.prepare_rows(&project, 0..2);
        assert!(Rc::ptr_eq(&rows[0].references, &appended[0].references));
        assert_eq!(
            appended[0]
                .references
                .iter()
                .map(|r| r.label.as_ref())
                .collect::<Vec<_>>(),
            appended[1]
                .references
                .iter()
                .map(|r| r.label.as_ref())
                .collect::<Vec<_>>()
        );
    }
}
