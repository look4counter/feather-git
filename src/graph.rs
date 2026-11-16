use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct GraphCommit {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub marker: GraphMarker,
    pub input_edges: Vec<GraphEdge>,
    pub output_edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphMarker {
    pub lane: usize,
    pub color: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub waiting_commit_id: String,
    pub from_lane: usize,
    pub to_lane: usize,
    pub color: usize,
    pub tip: bool,
}

// D:/Project/zit의 행별 입력/출력 레인 알고리즘을 적용한다.
#[derive(Clone, Default)]
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
#[derive(Clone, Default)]
pub struct GraphBuilder {
    previous_outputs: Vec<GraphEdge>,
    colors: GraphColorPool,
}

#[cfg(test)]
pub fn build_commit_graph(commits: &mut [GraphCommit]) {
    GraphBuilder::default().append(commits);
}

impl GraphBuilder {
    pub fn append(&mut self, commits: &mut [GraphCommit]) {
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

            for (parent_index, parent_id) in commit.parent_ids.iter().enumerate() {
                if parent_index == 0 {
                    outputs.push(GraphEdge {
                        waiting_commit_id: parent_id.clone(),
                        from_lane: marker.lane,
                        to_lane: marker.lane,
                        color: marker.color,
                        tip: true,
                    });
                } else if let Some(edge) = inputs
                    .iter()
                    .filter(|edge| edge.waiting_commit_id == *parent_id)
                    .min_by_key(|edge| edge.to_lane)
                {
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

    fn commit(id: &str, parents: &[&str]) -> GraphCommit {
        GraphCommit {
            id: id.into(),
            parent_ids: parents.iter().map(|p| (*p).into()).collect(),
            ..GraphCommit::default()
        }
    }

    #[test]
    fn merge_parent_reuses_leftmost_waiting_lane_without_allocating_another() {
        let fixture = vec![
            commit("main-child", &["703b5ac3"]),
            commit("f5e33c1", &["703b5ac3"]),
            commit("7817dfdf", &["1bed9c3b", "703b5ac3"]),
            commit("1bed9c3b", &["base"]),
            commit("703b5ac3", &["base"]),
            commit("base", &[]),
        ];
        let mut complete = fixture.clone();
        build_commit_graph(&mut complete);
        let merge = &complete[2];
        let waiting: Vec<_> = merge
            .input_edges
            .iter()
            .filter(|edge| !edge.tip && edge.waiting_commit_id == "703b5ac3")
            .collect();
        assert_eq!(waiting.len(), 2);
        let first = waiting.iter().min_by_key(|edge| edge.to_lane).unwrap();
        assert_eq!(first.to_lane, 0);
        let parent = merge
            .output_edges
            .iter()
            .find(|edge| edge.tip && edge.waiting_commit_id == "703b5ac3")
            .unwrap();
        assert_eq!(parent.to_lane, first.to_lane);
        assert_eq!(parent.color, first.color);
        assert_eq!(
            merge.output_edges.iter().map(|edge| edge.to_lane).max(),
            Some(merge.marker.lane)
        );
        for page_size in 1..=fixture.len() {
            let mut paged = fixture.clone();
            let mut builder = GraphBuilder::default();
            for page in paged.chunks_mut(page_size) {
                builder.append(page);
            }
            for (actual, expected) in paged.iter().zip(&complete) {
                assert_eq!(actual.marker, expected.marker);
                assert_eq!(actual.input_edges, expected.input_edges);
                assert_eq!(actual.output_edges, expected.output_edges);
            }
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
