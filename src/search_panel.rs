use crate::{
    backend::ReadSlot,
    repository_search::{self, Match},
    text_input::TextInput,
    theme::*,
    virtual_list::{ListLoading, ListScrollHandle, SingleSelection, panel_message, virtual_list},
};
use gpui::{prelude::*, *};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchEvent {
    Commit { id: String, revision: u64 },
    File { path: String, revision: u64 },
}
pub struct SearchPanel {
    root: PathBuf,
    editor: Entity<TextInput>,
    revision: u64,
    loading: ListLoading,
    reads: ReadSlot,
    results: Vec<Match>,
    selection: SingleSelection,
    scroll: ListScrollHandle,
    error: Option<String>,
    searched: bool,
    enabled: bool,
}
fn search_enter(key: &Keystroke, focused: bool, composing: bool, enabled: bool) -> bool {
    let modifiers = key.modifiers;
    enabled
        && focused
        && !composing
        && key.key == "enter"
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.shift
        && !modifiers.platform
        && !modifiers.function
}

fn select_search_result(
    results: &[Match],
    selection: &mut SingleSelection,
    index: usize,
    revision: u64,
    enabled: bool,
    pending: bool,
) -> Option<SearchEvent> {
    if !enabled || pending || index >= results.len() {
        return None;
    }
    selection.select(index, results.len());
    match &results[index] {
        Match::Commit { id, .. } => Some(SearchEvent::Commit {
            id: id.clone(),
            revision,
        }),
        Match::File { .. } => None,
    }
}

impl EventEmitter<SearchEvent> for SearchPanel {}
impl SearchPanel {
    pub fn new(root: PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            root,
            editor: cx.new(TextInput::compact),
            revision: 0,
            loading: Default::default(),
            reads: Default::default(),
            results: Vec::new(),
            selection: Default::default(),
            scroll: Default::default(),
            error: None,
            searched: false,
            enabled: true,
        }
    }
    pub fn search_pending(&self) -> bool {
        self.loading.is_pending()
    }

    pub fn is_current(&self, revision: u64) -> bool {
        self.enabled && self.revision == revision && !self.search_pending()
    }
    pub fn is_for(&self, root: &std::path::Path) -> bool {
        self.root == root
    }
    pub fn selected_commit_is(&self, revision: u64, expected: &str) -> bool {
        self.is_current(revision)
            && self
                .selection
                .selected(self.results.len())
                .and_then(|index| self.results.get(index))
                .is_some_and(|result| matches!(result, Match::Commit { id, .. } if id == expected))
    }
    fn select_result(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(event) = select_search_result(
            &self.results,
            &mut self.selection,
            index,
            self.revision,
            self.enabled,
            self.loading.is_pending(),
        ) {
            cx.emit(event);
        }
        cx.notify();
    }
    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        self.scroll.set_suspended(!enabled);
        self.editor.update(cx, |editor, cx| {
            editor.enabled = enabled;
            cx.notify();
        });
        cx.notify();
    }
    pub fn deactivate(&mut self) {
        self.enabled = false;
        self.scroll.set_suspended(true);
        self.reads.cancel();
        self.loading.reset();
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search(window, cx);
    }
    fn search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }
        let query = self.editor.read(cx).text();
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        self.error = None;
        self.selection.clear();
        self.scroll.reset_for_content();
        self.searched = true;
        self.reads.cancel();
        self.loading.reset();
        if query.trim().is_empty() {
            self.results.clear();
            self.searched = false;
            cx.notify();
            return;
        }
        self.loading.begin(revision, std::time::Instant::now());
        let completed = self.loading.completion();
        let root = self.root.clone();
        let task = self.reads.spawn(cx.background_executor(), move || {
            let result = crate::backend::run("search::search", || {
                repository_search::search(&root, &query)
            })
            .map_err(|error| format!("{error:#}"));
            completed.store(true, std::sync::atomic::Ordering::Release);
            result
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = task.await else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                if this.revision != revision {
                    return;
                }
                this.loading.reset();
                match result {
                    Ok(results) => this.results = results,
                    Err(error) => {
                        this.results.clear();
                        this.error = Some(error);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn menu(
        &mut self,
        index: usize,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.enabled || self.loading.is_pending() {
            return;
        }
        let Some(result @ Match::File { .. }) = self.results.get(index).cloned() else {
            return;
        };
        self.selection.select(index, self.results.len());
        let revision = self.revision;
        let owner = cx.weak_entity();
        let open = crate::context_menu::MenuItem::new(
            "파일 이력 보기",
            "",
            true,
            move |window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if !window.is_window_active()
                        || !this.is_current(revision)
                        || this.results.get(index) != Some(&result)
                    {
                        return;
                    }
                    if let Match::File { path, .. } = &result {
                        cx.emit(SearchEvent::File {
                            path: path.clone(),
                            revision,
                        });
                    }
                });
            },
        );
        crate::context_menu::open(position, vec![open], window, cx);
        cx.notify();
    }
}
impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.results.len();
        div().flex().flex_col().flex_1().min_h_0().min_w_0()
            .child(div().px_3().py_2().flex().items_center().gap_2()
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    let editor = this.editor.read(cx);
                    if search_enter(&event.keystroke, editor.focus_handle.is_focused(window), editor.is_composing(), this.enabled) {
                        this.search(window, cx); cx.stop_propagation();
                    }
                }))
                .child(div().flex_1().min_w_0().child(crate::form_controls::input(self.editor.clone()).h(px(30.)).min_h(px(30.))))
                .child(crate::dialog::button("search-run", "검색", true, self.enabled).h(px(30.)).px_3().text_size(px(12.)).on_click(cx.listener(|this, _, window, cx| this.search(window, cx)))))
            .when(!self.loading.is_pending() && count == 0, |body| body.child(panel_message(self.error.clone().unwrap_or_else(|| if self.searched { "검색 결과가 없습니다.".into() } else { "커밋 메시지·파일 경로·내용을 검색하세요.".into() }))))
            .when(count > 0 || self.loading.is_pending(), |body| body.child(virtual_list(
                "search-results", count, self.selection.selected(count), PANEL,
                cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                    range.filter_map(|index| this.results.get(index).map(|result| {
                        let (marker, color, text) = match result {
                            Match::Commit { title, .. } => ("C", 0xe6bc73, title.clone()),
                            Match::File { path, content } => ("F", ACCENT, format!("{path}{}", if *content { " · 내용 일치" } else { "" })),
                        };
                        div().id(("search-result", index)).h_full().px_2().flex().items_center().gap_2().min_w_0().text_size(px(12.))
                            .child(div().flex_shrink_0().text_color(rgb(color)).child(marker))
                            .child(div().min_w_0().truncate().child(text))
                            .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, window, cx| this.menu(index, event.position, window, cx)))
                    })).collect::<Vec<_>>()
                }),
                cx.processor(|this, index, _, cx| this.select_result(index, cx)),
            ).scroll_handle(self.scroll.clone()).loading(self.loading.clone()).keyboard_selection(self.enabled)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn enter_search_requires_input_focus_enabled_plain_key_and_no_ime_composition() {
        let enter = Keystroke::parse("enter").unwrap();
        assert!(search_enter(&enter, true, false, true));
        assert!(!search_enter(&enter, false, false, true));
        assert!(!search_enter(&enter, true, true, true));
        assert!(!search_enter(&enter, true, false, false));
        for key in [
            "shift-enter",
            "ctrl-enter",
            "alt-enter",
            "cmd-enter",
            "space",
        ] {
            assert!(!search_enter(
                &Keystroke::parse(key).unwrap(),
                true,
                false,
                true
            ));
        }
    }
    #[::core::prelude::v1::test]
    fn commit_selection_navigates_and_file_selection_never_opens_history_automatically() {
        let results = vec![
            Match::Commit {
                id: "first".into(),
                title: "first".into(),
            },
            Match::File {
                path: "file".into(),
                content: true,
            },
            Match::Commit {
                id: "second".into(),
                title: "second".into(),
            },
        ];
        let mut selection = SingleSelection::default();
        assert_eq!(
            select_search_result(&results, &mut selection, 0, 7, true, false),
            Some(SearchEvent::Commit {
                id: "first".into(),
                revision: 7
            })
        );
        assert_eq!(selection.selected(results.len()), Some(0));
        assert_eq!(
            select_search_result(&results, &mut selection, 1, 7, true, false),
            None
        );
        assert_eq!(selection.selected(results.len()), Some(1));
        assert_eq!(
            select_search_result(&results, &mut selection, 2, 7, true, false),
            Some(SearchEvent::Commit {
                id: "second".into(),
                revision: 7
            })
        );
        for (index, enabled, pending) in [(0, false, false), (0, true, true), (99, true, false)] {
            assert_eq!(
                select_search_result(&results, &mut selection, index, 7, enabled, pending),
                None
            );
            assert_eq!(selection.selected(results.len()), Some(2));
        }
    }
}
