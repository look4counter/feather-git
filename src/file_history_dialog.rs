use crate::{
    backend::ReadSlot,
    dialog::Dialog,
    diff_model::{Content, FileChange},
    diff_viewer::{DiffViewer, ViewerTarget},
    file_history::{self, FileHistoryEntry},
    history::{HistoryState, PreparedHistory},
    history_grid::HistoryColumns,
    repository::Project,
    theme::*,
    virtual_list::{ListLoading, SingleSelection, panel_message, virtual_list},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, path::PathBuf, rc::Rc};

pub struct FileHistoryClosed;

struct FileHistoryState {
    generation: u64,
    open: bool,
    loading: bool,
    list_loading: ListLoading,
    entries: Vec<FileHistoryEntry>,
    selection: SingleSelection,
    error: Option<String>,
}
impl Default for FileHistoryState {
    fn default() -> Self {
        let list_loading = ListLoading::default();
        list_loading.begin(1, std::time::Instant::now());
        Self {
            generation: 1,
            open: true,
            loading: true,
            list_loading,
            entries: Vec::new(),
            selection: SingleSelection::default(),
            error: None,
        }
    }
}
impl FileHistoryState {
    fn finish(&mut self, generation: u64, result: Result<Vec<FileHistoryEntry>, String>) -> bool {
        if !self.open || self.generation != generation {
            return false;
        }
        self.loading = false;
        self.list_loading.reset();
        self.selection.clear();
        match result {
            Ok(entries) => {
                self.entries = entries;
                self.error = None;
                self.selection.select(0, self.entries.len());
            }
            Err(error) => {
                self.entries.clear();
                self.error = Some(error);
            }
        }
        true
    }
    fn select(&mut self, index: usize) -> bool {
        self.open && !self.loading && self.selection.select(index, self.entries.len())
    }
    fn action_entry(&self, generation: u64, index: usize) -> Option<&FileHistoryEntry> {
        if !self.open || self.loading || generation != self.generation {
            return None;
        }
        self.entries.get(index)
    }

    fn viewer_targets(&self, root: &std::path::Path) -> (ViewerTarget, ViewerTarget) {
        let entry = self
            .selection
            .selected(self.entries.len())
            .filter(|_| self.open && !self.loading)
            .and_then(|index| self.entries.get(index));
        match entry {
            Some(entry) => (
                ViewerTarget::File {
                    root: root.to_path_buf(),
                    file: entry.file.clone(),
                    historical: true,
                },
                ViewerTarget::Detail {
                    root: root.to_path_buf(),
                    id: entry.commit.id.clone(),
                },
            ),
            None => (
                ViewerTarget::Text {
                    identity: "file-history-empty".into(),
                    title: "선택된 파일 0개".into(),
                    lines: Vec::new(),
                },
                ViewerTarget::Text {
                    identity: "file-history-detail-empty".into(),
                    title: "커밋상세".into(),
                    lines: Vec::new(),
                },
            ),
        }
    }

    fn close(&mut self) {
        self.open = false;
        self.list_loading.reset();
        self.generation = self.generation.wrapping_add(1);
    }
}

fn can_open_revision(file: &FileChange) -> bool {
    matches!(file.after, Content::Object(_, mode) if mode & 0o170000 == 0o100000)
}

pub struct FileHistoryDialog {
    root: PathBuf,
    workdir: PathBuf,
    label: String,
    state: FileHistoryState,
    project: Project,
    history: HistoryState,
    columns: HistoryColumns,
    resize: Option<(usize, f32, [f32; 5])>,
    reads: ReadSlot,
    actions: ReadSlot,
    temporary_files: std::sync::Arc<crate::history_temp::Session>,
    action_error: Option<String>,
    viewer: Entity<DiffViewer>,
    detail_viewer: Entity<DiffViewer>,
    focus: FocusHandle,
}
impl EventEmitter<FileHistoryClosed> for FileHistoryDialog {}

impl FileHistoryDialog {
    pub fn new(
        root: PathBuf,
        workdir: PathBuf,
        file: FileChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let viewer = cx.new(DiffViewer::new);
        let detail_viewer = cx.new(DiffViewer::new);
        let focus = cx.focus_handle();
        focus.focus(window);
        let mut reads = ReadSlot::default();
        let owner = root.clone();
        let raw = file.path;
        let state = FileHistoryState::default();
        let completed = state.list_loading.completion();
        let task = reads.spawn(cx.background_executor(), move || {
            let result = file_history::load(&owner, &raw).map_err(|error| format!("{error:#}"));
            completed.store(true, std::sync::atomic::Ordering::Release);
            result
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = task.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                if this.state.finish(1, result) {
                    this.project.commits = this
                        .state
                        .entries
                        .iter()
                        .map(|entry| entry.commit.clone())
                        .collect();
                    this.history = HistoryState::from_prepared(PreparedHistory::without_graph(
                        &this.project.commits,
                    ));
                    this.update_viewer(window, cx);
                    cx.notify();
                }
            });
        })
        .detach();
        let mut this = Self {
            root,
            workdir,
            label: file.label,
            viewer,
            detail_viewer,
            focus,
            reads,
            actions: ReadSlot::default(),
            temporary_files: Default::default(),
            action_error: None,
            state,
            project: Project::default(),
            history: HistoryState::from_prepared(PreparedHistory::empty()),
            columns: HistoryColumns::default(),
            resize: None,
        };
        this.update_viewer(window, cx);
        this
    }

    pub fn deactivate(&mut self, cx: &mut Context<Self>) {
        self.state.close();
        self.reads.cancel();
        self.actions.cancel();
        self.temporary_files.close();
        self.history.scroll_frame.set_suspended(true);
        self.viewer.update(cx, |viewer, cx| viewer.deactivate(cx));
        self.detail_viewer
            .update(cx, |viewer, cx| viewer.deactivate(cx));
    }

    fn update_viewer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (target, detail) = self.state.viewer_targets(&self.root);
        self.viewer
            .update(cx, |viewer, cx| viewer.set_target(target, window, cx));
        self.detail_viewer
            .update(cx, |viewer, cx| viewer.set_target(detail, window, cx));
    }

    fn run_file_action(
        &mut self,
        generation: u64,
        index: usize,
        reveal: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !window.is_window_active() {
            return;
        }
        let Some(entry) = self.state.action_entry(generation, index).cloned() else {
            return;
        };
        if !reveal && !can_open_revision(&entry.file) {
            return;
        }
        self.action_error = None;
        let root = self.root.clone();
        let temporary_files = self.temporary_files.clone();
        let task = self.actions.spawn(cx.background_executor(), move || {
            if reveal {
                crate::workspace_backend::prepare_file_location(&root, &entry.file.path, true)
                    .map(|target| target.path)
            } else {
                file_history::export_revision(
                    &root,
                    &entry.commit.id,
                    &entry.file.path,
                    &temporary_files,
                )
                .map_err(|error| format!("{error:#}"))
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = task.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                if !window.is_window_active()
                    || this.state.action_entry(generation, index).is_none()
                {
                    return;
                }
                match result {
                    Ok(path) => cx
                        .open_with_system(&PathBuf::from(crate::path_display::display_path(&path))),
                    Err(error) => {
                        this.action_error = Some(error);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn copy_row_path(&self, generation: u64, index: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.state.action_entry(generation, index) else {
            return;
        };
        if let Ok(path) = crate::path_display::absolute_file_path(&self.workdir, &entry.file.path) {
            cx.write_to_clipboard(ClipboardItem::new_string(
                crate::path_display::display_path(&path),
            ));
        }
    }

    fn open_row_menu(
        &mut self,
        index: usize,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let generation = self.state.generation;
        let Some(entry) = self.state.action_entry(generation, index) else {
            return;
        };
        let openable = can_open_revision(&entry.file);
        self.select(index, window, cx);
        self.focus.focus(window);
        let open_owner = cx.weak_entity();
        let reveal_owner = cx.weak_entity();
        let copy_owner = cx.weak_entity();
        crate::context_menu::open(
            position,
            vec![
                crate::context_menu::MenuItem::new("열기", "", openable, move |window, cx| {
                    let _ = open_owner.update(cx, |this, cx| {
                        this.run_file_action(generation, index, false, window, cx)
                    });
                }),
                crate::context_menu::MenuItem::new(
                    "탐색기에서 열기",
                    "",
                    true,
                    move |window, cx| {
                        let _ = reveal_owner.update(cx, |this, cx| {
                            this.run_file_action(generation, index, true, window, cx)
                        });
                    },
                ),
                crate::context_menu::MenuItem::new("경로 복사", "", true, move |window, cx| {
                    if window.is_window_active() {
                        let _ = copy_owner
                            .update(cx, |this, cx| this.copy_row_path(generation, index, cx));
                    }
                }),
            ],
            window,
            cx,
        );
        cx.stop_propagation();
    }

    fn resize_columns(&mut self, x: f32, pressed: bool) -> bool {
        let Some((column, start_x, widths)) = self.resize else {
            return false;
        };
        if !pressed {
            self.resize = None;
            return true;
        }
        self.columns.resize_boundary(column, widths, x - start_x)
    }

    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.select(index) {
            self.focus.focus(window);
            self.update_viewer(window, cx);
            cx.notify();
        }
    }
}

impl Render for FileHistoryDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = self.columns.widths();
        let count = self.state.entries.len();
        let measured = Rc::new(Cell::new(columns));
        let resize_widths = measured.clone();
        let resize = cx.processor(move |this, (column, x), _, cx| {
            if this.state.open {
                this.resize = Some((column, x, resize_widths.get()));
                cx.stop_propagation();
            }
        });
        let header = crate::ui::commit_grid_header(
            columns,
            false,
            measured,
            move |column, x, window, cx| resize((column, x), window, cx),
        );
        let on_context = Rc::new(cx.processor(|this, (index, position), window, cx| {
            this.open_row_menu(index, position, window, cx)
        }));
        let list = virtual_list(
            "file-history-commits",
            count,
            self.state.selection.selected(count),
            PANEL,
            cx.processor(move |this, range, _, _| {
                this.history
                    .prepare_rows(&this.project, range)
                    .into_iter()
                    .map(|row| {
                        let index = row.index;
                        let on_context = on_context.clone();
                        crate::ui::commit_grid_row(row, columns, 1, false)
                            .id(("history-context-row", index))
                            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                on_context((index, event.position), window, cx);
                                cx.stop_propagation();
                            })
                    })
                    .collect::<Vec<_>>()
            }),
            cx.processor(|this, index, window, cx| this.select(index, window, cx)),
        )
        .scroll_handle(self.history.scroll_frame.clone())
        .header(header, crate::virtual_list::ROW_HEIGHT)
        .loading(self.state.list_loading.clone())
        .min_content_width(self.columns.minimum_total_width_for(false));
        let commits = div()
            .flex()
            .flex_col()
            .w_full()
            .h(relative(0.5))
            .flex_shrink_0()
            .min_w_0()
            .min_h_0()
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(MUTED))
                    .child(format!("모든 브랜치 · {}개 커밋", count)),
            )
            .when_some(self.state.error.clone(), |panel, error| {
                panel.child(panel_message(error).text_color(rgb(0xffa7b2)))
            })
            .when_some(self.action_error.clone(), |panel, error| {
                panel.child(div().p_3().text_color(rgb(0xffa7b2)).child(error))
            })
            .when(
                !self.state.loading && self.state.error.is_none() && count == 0,
                |panel| panel.child(panel_message("이 파일의 커밋 이력이 없습니다.")),
            )
            .when(self.state.loading || count > 0, |panel| panel.child(list));
        let left = div()
            .flex()
            .flex_col()
            .w(relative(0.48))
            .min_w_0()
            .min_h_0()
            .border_r_1()
            .border_color(rgb(LINE))
            .overflow_hidden()
            .child(commits)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h(relative(0.5))
                    .flex_shrink_0()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.detail_viewer.clone()),
            );
        let body = div()
            .flex()
            .w_full()
            .h(window.viewport_size().height * 0.72)
            .min_h_0()
            .flex_shrink_0()
            .border_1()
            .border_color(rgb(LINE))
            .overflow_hidden()
            .child(left)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .child(self.viewer.clone()),
            );
        Dialog::new(
            "file-history-overlay",
            format!("파일 변경 이력 · {}", self.label),
        )
        .width(window.viewport_size().width * 0.94)
        .track_focus(&self.focus)
        .on_key_down(cx.listener(|_, event: &KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" {
                cx.emit(FileHistoryClosed);
                cx.stop_propagation();
            }
        }))
        .body(body)
        .footer(
            crate::dialog::button("file-history-close", "닫기", false, true)
                .on_click(cx.listener(|_, _, _, cx| cx.emit(FileHistoryClosed))),
        )
        .build()
        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
            if this.resize_columns(event.position.x.into(), event.dragging()) {
                cx.notify();
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, event: &MouseUpEvent, _, cx| {
                if this.resize_columns(event.position.x.into(), false) {
                    cx.notify();
                }
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|this, event: &MouseUpEvent, _, cx| {
                if this.resize_columns(event.position.x.into(), false) {
                    cx.notify();
                }
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff_model::Content;
    fn entry(id: &str) -> FileHistoryEntry {
        FileHistoryEntry {
            commit: crate::repository::Commit {
                id: id.into(),
                ..Default::default()
            },
            file: FileChange {
                path: b"file.txt".to_vec(),
                label: "file.txt".into(),
                status: "A".into(),
                staged: false,
                before: Content::Empty,
                after: Content::Empty,
            },
        }
    }
    #[::core::prelude::v1::test]
    fn viewer_targets_follow_selection_and_clear_for_unavailable_history() {
        let root = PathBuf::from("repository");
        let mut state = FileHistoryState::default();
        let empty = |state: &FileHistoryState| {
            let (file, detail) = state.viewer_targets(&root);
            assert!(matches!(file, ViewerTarget::Text { .. }));
            assert!(matches!(detail, ViewerTarget::Text { .. }));
        };
        empty(&state);
        let mut second = entry("second");
        second.file.path = b"renamed.txt".to_vec();
        state.finish(1, Ok(vec![entry("first"), second]));
        let (file, detail) = state.viewer_targets(&root);
        assert!(matches!(
            file,
            ViewerTarget::File {
                historical: true,
                ..
            }
        ));
        assert_eq!(
            detail,
            ViewerTarget::Detail {
                root: root.clone(),
                id: "first".into()
            }
        );
        assert!(state.select(1));
        let (file, detail) = state.viewer_targets(&root);
        assert!(matches!(file, ViewerTarget::File { file, .. } if file.path == b"renamed.txt"));
        assert_eq!(
            detail,
            ViewerTarget::Detail {
                root: root.clone(),
                id: "second".into()
            }
        );
        assert!(!state.select(9));
        assert_eq!(state.viewer_targets(&root).1, detail);
        state.close();
        empty(&state);
        let mut state = FileHistoryState::default();
        state.finish(1, Ok(Vec::new()));
        empty(&state);
        state.finish(1, Err("failed".into()));
        empty(&state);
    }

    #[::core::prelude::v1::test]
    fn row_actions_keep_requested_commit_and_reject_deleted_closed_or_stale_state() {
        let mut state = FileHistoryState::default();
        assert!(state.action_entry(1, 0).is_none());
        state.finish(1, Ok(vec![entry("first"), entry("second")]));
        assert_eq!(state.action_entry(1, 1).unwrap().commit.id, "second");
        assert!(state.action_entry(0, 1).is_none());
        assert!(state.action_entry(1, 2).is_none());
        assert!(!can_open_revision(&state.entries[0].file));
        state.entries[0].file.after = Content::Object(
            gix::ObjectId::from_hex(b"0000000000000000000000000000000000000000").unwrap(),
            0o100644,
        );
        assert!(can_open_revision(&state.entries[0].file));
        let root = std::env::current_dir().unwrap();
        assert_eq!(
            crate::path_display::absolute_file_path(
                &root,
                &state.action_entry(1, 1).unwrap().file.path
            )
            .unwrap(),
            root.join("file.txt")
        );
        state.close();
        assert!(state.action_entry(1, 1).is_none());
    }

    #[::core::prelude::v1::test]
    fn popup_grid_loading_is_delayed_and_cleared_on_completion_or_close() {
        use std::sync::atomic::Ordering;
        let mut state = FileHistoryState::default();
        let start = std::time::Instant::now();
        state.list_loading.begin(1, start);
        assert!(
            !state
                .list_loading
                .reveal(1, start + std::time::Duration::from_millis(149))
        );
        assert!(
            state
                .list_loading
                .reveal(1, start + crate::loading::LOADING_DELAY)
        );
        assert!(state.list_loading.is_visible(1));
        state
            .list_loading
            .completion()
            .store(true, Ordering::Release);
        assert!(!state.list_loading.is_visible(1));
        assert!(state.finish(1, Ok(vec![entry("loaded")])));
        assert!(!state.list_loading.is_pending());
        let mut fast = FileHistoryState::default();
        let completion = fast.list_loading.completion();
        completion.store(true, Ordering::Release);
        assert!(
            !fast
                .list_loading
                .reveal(1, start + std::time::Duration::from_secs(1))
        );
        fast.close();
        assert!(!fast.list_loading.is_pending());
        assert!(!fast.finish(1, Ok(vec![entry("late")])));
    }

    #[::core::prelude::v1::test]
    fn history_selection_is_independent_and_repeated_or_late_results_are_ignored() {
        let mut state = FileHistoryState::default();
        assert!(!state.select(0));
        assert!(!state.finish(0, Ok(vec![entry("old")])));
        assert!(state.finish(1, Ok(vec![entry("first"), entry("second")])));
        assert_eq!(state.selection.selected(2), Some(0));
        assert!(!state.select(0));
        assert!(!state.select(2));
        assert!(state.select(1));
        assert!(!state.select(1));
        state.close();
        assert!(!state.finish(1, Ok(vec![entry("late")])));
        assert!(!state.select(0));
        assert_eq!(state.entries[1].commit.id, "second");
        let mut empty = FileHistoryState::default();
        assert!(empty.finish(1, Ok(Vec::new())));
        assert_eq!(empty.selection.selected(0), None);
        let mut failed = FileHistoryState::default();
        assert!(failed.finish(1, Err("failed".into())));
        assert_eq!(failed.error.as_deref(), Some("failed"));
        assert!(!failed.loading);
        assert!(!failed.select(0));
    }
}
