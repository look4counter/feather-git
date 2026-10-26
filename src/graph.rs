use crate::model::{Commit, GraphEdge, GraphMarker};
use std::collections::{HashMap, HashSet};

/// 여러 tip의 시각이 같거나 잘못된 경우에도 부모가 자식보다 위에 나오지 않게 한다.
pub fn order_commits(commits: &mut Vec<Commit>) {
    use std::{cmp::Reverse, collections::BinaryHeap};
    let mut nodes: Vec<_> = std::mem::take(commits).into_iter().map(Some).collect();
    let indices: HashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_ref().unwrap().id.clone(), i))
        .collect();
    let mut children = vec![0usize; nodes.len()];
    for commit in nodes.iter().flatten() {
        for parent in &commit.parent_ids {
            if let Some(&index) = indices.get(parent) {
                children[index] += 1;
            }
        }
    }
    let mut ready = BinaryHeap::new();
    for (index, commit) in nodes.iter().enumerate() {
        if children[index] == 0 {
            ready.push((commit.as_ref().unwrap().time, Reverse(index)));
        }
    }
    while let Some((_, Reverse(index))) = ready.pop() {
        let commit = nodes[index].take().unwrap();
        for parent in &commit.parent_ids {
            if let Some(&index) = indices.get(parent) {
                children[index] -= 1;
                if children[index] == 0 {
                    ready.push((nodes[index].as_ref().unwrap().time, Reverse(index)));
                }
            }
        }
        commits.push(commit);
    }
    commits.extend(nodes.into_iter().flatten());
}

// D:/Project/zit의 행별 입력/출력 레인 알고리즘을 적용한다.
#[derive(Default)]
struct GraphColorPool(usize);

impl GraphColorPool {
    fn next(&mut self) -> usize {
        self.0 += 1;
        self.0
    }
}

/// Applies the project history lane algorithm to commits that are already in display order.
///
/// Existing graph fields are replaced. Repository I/O and presentation are deliberately kept
/// outside this function so topology can be tested independently.
#[derive(Default)]
pub struct GraphBuilder {
    previous_outputs: Vec<GraphEdge>,
    colors: GraphColorPool,
}

pub fn build_commit_graph(commits: &mut [Commit]) {
    GraphBuilder::default().append(commits);
}

impl GraphBuilder {
    pub fn append(&mut self, commits: &mut [Commit]) {
        for commit in commits {
            let mut marker = GraphMarker::default();
            let mut inputs = Vec::new();

            if !self.previous_outputs.is_empty() {
                let mut reached = false;
                let mut occupied = 0;
                let mut seen_destinations = HashSet::new();

                for edge in self
                    .previous_outputs
                    .iter()
                    .filter(|edge| seen_destinations.insert(edge.to_lane))
                {
                    if edge.waiting_commit_id == commit.id {
                        if !reached {
                            reached = true;
                            marker.lane = inputs.len();
                            marker.color = edge.color;
                            occupied += 1;
                        }
                        inputs.push(GraphEdge {
                            waiting_commit_id: commit
                                .parent_ids
                                .first()
                                .cloned()
                                .unwrap_or_default(),
                            from_lane: edge.to_lane,
                            to_lane: marker.lane,
                            color: edge.color,
                            tip: true,
                        });
                    } else {
                        inputs.push(GraphEdge {
                            waiting_commit_id: edge.waiting_commit_id.clone(),
                            from_lane: edge.to_lane,
                            to_lane: occupied,
                            color: edge.color,
                            tip: false,
                        });
                        occupied += 1;
                    }
                }

                if !reached {
                    marker.lane = inputs.len();
                    marker.color = self.colors.next();
                }
            }

            let mut outputs = inputs
                .iter()
                .filter(|edge| !edge.tip)
                .map(|edge| GraphEdge {
                    waiting_commit_id: edge.waiting_commit_id.clone(),
                    from_lane: edge.to_lane,
                    to_lane: edge.to_lane,
                    color: edge.color,
                    tip: false,
                })
                .collect::<Vec<_>>();

            let waiting_lanes: HashMap<&str, &GraphEdge> = inputs
                .iter()
                .map(|edge| (edge.waiting_commit_id.as_str(), edge))
                .collect();

            for (parent_index, parent_id) in commit.parent_ids.iter().enumerate() {
                if parent_index == 0 {
                    outputs.push(GraphEdge {
                        waiting_commit_id: parent_id.clone(),
                        from_lane: marker.lane,
                        to_lane: marker.lane,
                        color: marker.color,
                        tip: true,
                    });
                } else if let Some(edge) = waiting_lanes.get(parent_id.as_str()) {
                    outputs.push(GraphEdge {
                        waiting_commit_id: parent_id.clone(),
                        from_lane: marker.lane,
                        to_lane: edge.to_lane,
                        color: edge.color,
                        tip: true,
                    });
                } else {
                    outputs.push(GraphEdge {
                        waiting_commit_id: parent_id.clone(),
                        from_lane: marker.lane,
                        to_lane: outputs.len(),
                        color: self.colors.next(),
                        tip: true,
                    });
                }
            }

            outputs.sort_by_key(|edge| edge.to_lane);
            commit.marker = marker;
            commit.input_edges = inputs;
            commit.output_edges = outputs.clone();
            self.previous_outputs = outputs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(id: &str, parents: &[&str]) -> Commit {
        Commit {
            id: id.into(),
            parent_ids: parents.iter().map(|p| (*p).into()).collect(),
            ..Commit::default()
        }
    }

    #[test]
    fn page_boundaries_preserve_merge_lanes_and_colors() {
        let mut input = vec![commit("merge", &["main0", "side0"])];
        for (prefix, count) in [("main", 210), ("side", 30)] {
            for index in 0..count {
                let parent = if index + 1 == count {
                    "base".into()
                } else {
                    format!("{prefix}{}", index + 1)
                };
                input.push(commit(&format!("{prefix}{index}"), &[&parent]));
            }
        }
        input.push(commit("base", &[]));
        let mut complete = input.clone();
        build_commit_graph(&mut complete);
        let mut builder = GraphBuilder::default();
        for page in input.chunks_mut(200) {
            builder.append(page);
        }
        for (paged, full) in input.iter().zip(&complete) {
            assert_eq!(paged.marker, full.marker);
            assert_eq!(paged.input_edges, full.input_edges);
            assert_eq!(paged.output_edges, full.output_edges);
        }
    }

    #[test]
    fn multiple_tips_and_clock_skew_keep_children_above_parents() {
        let mut base = commit("base", &[]);
        base.time = 100;
        let mut commits = vec![base, commit("left", &["base"]), commit("right", &["base"])];
        order_commits(&mut commits);
        assert_eq!(commits.last().unwrap().id, "base");
        assert_eq!(commits.len(), 3);
        build_commit_graph(&mut commits);
        assert_eq!(
            commits
                .last()
                .unwrap()
                .input_edges
                .iter()
                .filter(|e| e.tip)
                .count(),
            2
        );
    }

    #[test]
    fn linear_history_keeps_lane_color_and_external_parent() {
        let mut commits = vec![commit("c", &["b"]), commit("b", &["outside"])];
        build_commit_graph(&mut commits);
        assert!(
            commits
                .iter()
                .all(|c| c.marker == GraphMarker { lane: 0, color: 0 })
        );
        assert_eq!(commits[1].output_edges[0].waiting_commit_id, "outside");
        assert_eq!(
            commits[0].output_edges[0].to_lane,
            commits[1].input_edges[0].from_lane
        );
    }

    #[test]
    fn merge_opens_a_lane_and_converges_at_base() {
        let mut commits = vec![
            commit("merge", &["main", "side"]),
            commit("main", &["base"]),
            commit("side", &["base"]),
            commit("base", &[]),
        ];
        build_commit_graph(&mut commits);
        assert_eq!(commits[0].output_edges.len(), 2);
        assert_ne!(
            commits[0].output_edges[0].color,
            commits[0].output_edges[1].color
        );
        assert_eq!(commits[2].marker.lane, 1);
        assert_eq!(commits[3].input_edges.iter().filter(|e| e.tip).count(), 2);
        assert!(
            commits[3]
                .input_edges
                .iter()
                .any(|e| e.from_lane != e.to_lane)
        );
        assert!(commits[3].output_edges.is_empty());
        for rows in commits.windows(2) {
            for input in &rows[1].input_edges {
                assert!(
                    rows[0]
                        .output_edges
                        .iter()
                        .any(|e| e.to_lane == input.from_lane && e.color == input.color)
                );
            }
        }
    }

    #[test]
    fn multiple_roots_octopus_and_rebuilding_are_deterministic() {
        let mut commits = vec![
            commit("merge", &["a", "b", "c"]),
            commit("a", &[]),
            commit("unrelated", &[]),
            commit("b", &[]),
            commit("c", &[]),
        ];
        build_commit_graph(&mut commits);
        assert_eq!(commits[0].output_edges.len(), 3);
        let first: Vec<_> = commits
            .iter()
            .map(|c| {
                (
                    c.marker.clone(),
                    c.input_edges.clone(),
                    c.output_edges.clone(),
                )
            })
            .collect();
        build_commit_graph(&mut commits);
        let second: Vec<_> = commits
            .iter()
            .map(|c| {
                (
                    c.marker.clone(),
                    c.input_edges.clone(),
                    c.output_edges.clone(),
                )
            })
            .collect();
        assert_eq!(first, second);
        assert!(commits.last().unwrap().output_edges.is_empty());
        build_commit_graph(&mut []);
    }
}
