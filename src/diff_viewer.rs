use crate::{
    conflict::{self, Choice},
    diff_git,
    diff_model::{CommitDetail, DetailIdentity, Diff, DiffLine, FileChange},
    partial_stage,
    text_input::TextInput,
    text_selection::{Position, Selection},
    theme::*,
    virtual_list::{ListLoading, ListScrollHandle, ROW_HEIGHT, panel_message, virtual_list},
};
use gpui::{prelude::*, *};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

const ADD_BG: u32 = 0x18382c;
const ADD_TEXT: u32 = 0x9be9b1;
const DELETE_BG: u32 = 0x40232b;
const DELETE_TEXT: u32 = 0xffa7b2;
const HEADER_BG: u32 = 0x202e48;
const HEADER_TEXT: u32 = 0x9dc6ff;
const COMMANDS: [(&str, &str, &str); 11] = [
    ("up", "위로", "up"),
    ("down", "아래로", "down"),
    ("left", "왼쪽", "left"),
    ("right", "오른쪽", "right"),
    ("home", "줄 처음", "home"),
    ("end", "줄 끝", "end"),
    ("all", "전체 선택", "ctrl-a"),
    ("copy", "복사", "ctrl-c"),
    ("toggle", "변경 줄 체크", "space"),
    ("stage", "선택 줄 Stage", ""),
    ("refresh", "다시 조회", "f5"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewerTarget {
    Files {
        root: PathBuf,
        files: Vec<FileChange>,
    },
    File {
        root: PathBuf,
        file: FileChange,
        historical: bool,
    },
    Detail {
        root: PathBuf,
        id: String,
    },
    Text {
        identity: String,
        title: String,
        lines: Vec<String>,
    },
}
impl ViewerTarget {
    fn header_title(&self) -> &str {
        let title = match self {
            Self::Files { .. } => "선택한 파일",
            Self::File { file, .. } => file.label.as_str(),
            Self::Detail { .. } => "커밋상세",
            Self::Text { title, .. } => title.as_str(),
        };
        if title.is_empty() {
            "Diff Viewer"
        } else {
            title
        }
    }

    fn details(&self) -> bool {
        !matches!(self, Self::File { .. } | Self::Files { .. })
    }
    fn file_targets(&self) -> Option<Vec<Self>> {
        let Self::Files { root, files } = self else {
            return None;
        };
        Some(
            files
                .iter()
                .map(|file| Self::File {
                    root: root.clone(),
                    file: file.clone(),
                    historical: false,
                })
                .collect(),
        )
    }
    fn same_file(&self, other: &Self) -> bool {
        matches!((self, other), (Self::File {root: a, file: left, ..}, Self::File {root: b, file: right, ..})
            if a == b && left.path == right.path && left.staged == right.staged)
    }
    #[cfg(test)]
    fn load(&self) -> anyhow::Result<(Diff, Option<FileChange>)> {
        self.load_with_encoding(crate::text_encoding::TextEncoding::Utf8)
    }
    fn load_with_encoding(
        &self,
        encoding: crate::text_encoding::TextEncoding,
    ) -> anyhow::Result<(Diff, Option<FileChange>)> {
        match self {
            Self::Files { .. } => anyhow::bail!("여러 파일은 파일별로 조회해야 합니다."),
            Self::File {
                root,
                file,
                historical,
            } => {
                let source = if *historical {
                    Some(file.clone())
                } else {
                    diff_git::load_stage(root)?
                        .into_iter()
                        .find(|current| current.path == file.path && current.staged == file.staged)
                };
                let Some(source) = source else {
                    return Ok((
                        Diff {
                            note: "이 파일의 변경이 더 이상 없습니다. 파일 목록을 새로 확인하세요."
                                .into(),
                            ..Default::default()
                        },
                        None,
                    ));
                };
                let mut diff = diff_git::diff_with_encoding(root, &source, encoding)?;
                if *historical {
                    diff.partial = None;
                    diff.conflict = None;
                }
                Ok((diff, Some(source)))
            }
            Self::Detail { root, id } => crate::backend::run("viewer::load_detail", || {
                let detail = diff_git::load_detail(root, id)?;
                let mut data = text_diff(detail_rows(&detail));
                let source = crate::repository::github_repository(root).ok().flatten();
                data.identities = detail_identities(&detail, source.as_deref());
                Ok((data, None))
            }),
            Self::Text { lines, .. } => Ok((text_diff(lines.clone()), None)),
        }
    }
}
fn text_diff(lines: Vec<String>) -> Diff {
    Diff {
        lines: lines
            .into_iter()
            .map(|text| DiffLine {
                old: None,
                new: None,
                kind: ' ',
                text,
            })
            .collect(),
        ..Default::default()
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct ViewerKeys {
    overrides: HashMap<String, String>,
}
impl ViewerKeys {
    fn binding(&self, command: &str) -> String {
        let binding = self.overrides.get(command).cloned().unwrap_or_else(|| {
            COMMANDS
                .iter()
                .find(|(id, _, _)| *id == command)
                .map_or("", |(_, _, key)| *key)
                .into()
        });
        if binding.trim().eq_ignore_ascii_case("enter") {
            String::new()
        } else {
            binding
        }
    }
    fn validate(entries: &[(String, String)]) -> anyhow::Result<Self> {
        let expected = key_entries(&Self::default());
        if entries.len() != expected.len()
            || expected
                .iter()
                .any(|(id, _)| !entries.iter().any(|(key, _)| key == id))
        {
            anyhow::bail!("단축키 항목이 누락되었거나 잘못되었습니다.");
        }
        let mut seen = BTreeSet::new();
        let mut overrides = HashMap::new();
        for (id, key) in entries {
            let key = key.trim().to_lowercase();
            if key.is_empty() {
                overrides.insert(id.clone(), key);
                continue;
            }
            let parsed = Keystroke::parse(&key).map_err(|_| anyhow::anyhow!("잘못된 키: {key}"))?;
            let key = key_string(&parsed, false);
            let named = [
                "up",
                "down",
                "left",
                "right",
                "home",
                "end",
                "space",
                "enter",
                "tab",
                "escape",
                "backspace",
                "delete",
                "pageup",
                "pagedown",
            ];
            let valid = parsed.key.chars().count() == 1
                || named.contains(&parsed.key.as_str())
                || parsed
                    .key
                    .strip_prefix('f')
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| (1..=24).contains(&n));
            if key == "enter" {
                anyhow::bail!("Enter는 DiffViewer 단축키로 사용할 수 없습니다.");
            }
            if key == "escape" {
                anyhow::bail!("Esc는 설정창 닫기에 사용합니다.");
            }
            if !valid || !seen.insert(key.clone()) {
                anyhow::bail!("비어 있거나 중복된 단축키입니다: {key}");
            }
            overrides.insert(id.clone(), key);
        }
        Ok(Self { overrides })
    }
    fn label(&self, id: &str) -> String {
        let keys = [self.binding(id), self.binding(&format!("{id}-secondary"))]
            .into_iter()
            .filter(|key| !key.is_empty())
            .collect::<Vec<_>>();
        if keys.is_empty() {
            "미지정".into()
        } else {
            keys.join(" / ")
        }
    }
    fn matches(&self, id: &str, key: &str) -> bool {
        self.binding(id) == key || self.binding(&format!("{id}-secondary")) == key
    }
    fn command(&self, key: &Keystroke) -> Option<&'static str> {
        let raw = key_string(key, false);
        if raw == "enter" {
            return None;
        }
        if let Some((id, _, _)) = COMMANDS.iter().find(|(id, _, _)| self.matches(id, &raw)) {
            return Some(id);
        }
        if key.modifiers.shift {
            let base = key_string(key, true);
            if let Some((id, _, _)) = COMMANDS.iter().find(|(id, _, _)| {
                ["up", "down", "left", "right", "home", "end"].contains(id)
                    && self.matches(id, &base)
            }) {
                return Some(id);
            }
        }
        if !key.modifiers.control && !key.modifiers.alt && !key.modifiers.platform {
            match key.key.to_lowercase().as_str() {
                "i" => Some("up"),
                "j" => Some("left"),
                "k" => Some("down"),
                "l" => Some("right"),
                _ => None,
            }
        } else {
            None
        }
    }
}
fn key_string(key: &Keystroke, without_shift: bool) -> String {
    let mut parts = Vec::new();
    if key.modifiers.control {
        parts.push("ctrl");
    }
    if key.modifiers.alt {
        parts.push("alt");
    }
    if key.modifiers.platform {
        parts.push("cmd");
    }
    if key.modifiers.shift && !without_shift {
        parts.push("shift");
    }
    let lower = key.key.to_lowercase();
    parts.push(&lower);
    parts.join("-")
}

#[derive(Clone)]
pub struct ViewerChanged {
    pub root: PathBuf,
}
impl EventEmitter<ViewerChanged> for DiffViewer {}

#[derive(Clone)]
struct ViewerWriteFailed {
    message: String,
}
impl EventEmitter<ViewerWriteFailed> for DiffViewer {}

#[derive(Clone)]
pub struct ViewerDiscardRequest {
    view: WeakEntity<DiffViewer>,
    generation: u64,
    target: ViewerTarget,
    root: PathBuf,
    file: FileChange,
    preview: partial_stage::Preview,
    lines: Vec<DiffLine>,
    rows: Vec<usize>,
}
impl EventEmitter<ViewerDiscardRequest> for DiffViewer {}
impl ViewerDiscardRequest {
    pub fn label(&self) -> &str {
        &self.file.label
    }
    pub fn count(&self) -> usize {
        self.rows.len()
    }
    pub fn execute(self, window: &mut Window, cx: &mut App) -> bool {
        self.view
            .update(cx, |this, cx| {
                if !current_request(
                    this.generation,
                    this.target.as_ref(),
                    self.generation,
                    &self.target,
                ) || this.mutation.is_some()
                    || !this.partial()
                {
                    return false;
                }
                // Workspace의 명시적 확인을 받은 요청이며 모달의 interactive 차단과는 별개입니다.
                let file = self.file;
                let preview = self.preview;
                let lines = self.lines;
                let rows = self.rows;
                this.mutate(self.root, ViewerMutation::StageOrDiscard, window, cx, move |root| {
                    partial_stage::discard(root, &file, &preview, &lines, &rows)
                });
                true
            })
            .unwrap_or(false)
    }
}

struct KeySettings {
    keys: ViewerKeys,
    error: Option<String>,
    revision: u64,
    loading: bool,
    saving: bool,
}
impl Default for KeySettings {
    fn default() -> Self {
        Self {
            keys: ViewerKeys::default(),
            error: None,
            revision: 0,
            loading: true,
            saving: false,
        }
    }
}
impl KeySettings {
    fn finish_load(&mut self, revision: u64, result: anyhow::Result<ViewerKeys>) {
        if revision != self.revision {
            return;
        }
        self.loading = false;
        match result {
            Ok(keys) => {
                self.keys = keys;
                self.error = None;
            }
            Err(error) => {
                self.error = Some(format!(
                    "설정을 읽지 못해 기본 단축키를 사용합니다: {error:#}"
                ))
            }
        }
    }
    fn begin_save(&mut self) -> Option<u64> {
        if self.loading || self.saving {
            return None;
        }
        self.revision = self.revision.wrapping_add(1);
        self.saving = true;
        Some(self.revision)
    }
    fn finish_save(&mut self, revision: u64, keys: ViewerKeys, result: anyhow::Result<()>) -> bool {
        if revision != self.revision {
            return false;
        }
        self.saving = false;
        match result {
            Ok(()) => {
                self.keys = keys;
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(format!("설정을 저장하지 못했습니다: {error:#}"));
                false
            }
        }
    }
}

struct FileViewer {
    target: ViewerTarget,
    view: Entity<DiffViewer>,
    active: bool,
    _events: Subscription,
    _discard: Subscription,
    _errors: Subscription,
    _observer: Subscription,
}

struct CheckMenuRequest {
    generation: u64,
    rows: std::ops::Range<usize>,
}

impl CheckMenuRequest {
    fn rows(
        &self,
        generation: u64,
        data: &Diff,
        available: bool,
    ) -> Option<std::ops::Range<usize>> {
        (available && generation == self.generation && checkable_rows(data, self.rows.clone()))
            .then(|| self.rows.clone())
    }
}

enum ViewerMutation {
    StageOrDiscard,
    ResolveConflict,
}

pub struct DiffViewer {
    reads: crate::backend::ReadSlot,
    file_viewers: Vec<FileViewer>,
    hide_header: bool,
    encoding: crate::text_encoding::TextEncoding,
    encoding_subscription: Option<Subscription>,
    target: Option<ViewerTarget>,
    generation: u64,
    data: Diff,
    active_file: Option<FileChange>,
    loading: bool,
    retaining_content: bool,
    list_loading: ListLoading,
    mutation: Option<ViewerMutation>,
    error: Option<String>,
    focus: FocusHandle,
    focus_subscriptions: Vec<gpui::Subscription>,
    scroll: ListScrollHandle,
    selection: Selection,
    dragging: bool,
    interactive: bool,
    checked: BTreeSet<usize>,
    width: Option<f32>,
    key_settings: KeySettings,
    key_form_revision: u64,
    key_editors: Option<Vec<(String, Entity<TextInput>)>>,
}
impl DiffViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let task = crate::backend::spawn(cx.background_executor(), load_keys);
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.key_settings.finish_load(0, result);
                cx.notify();
            });
        })
        .detach();
        Self {
            reads: crate::backend::ReadSlot::default(),
            file_viewers: Vec::new(),
            hide_header: false,
            encoding: crate::text_encoding::TextEncoding::current(cx),
            encoding_subscription: None,
            target: None,
            generation: 0,
            data: Diff::default(),
            active_file: None,
            loading: false,
            retaining_content: false,
            list_loading: ListLoading::default(),
            mutation: None,
            error: None,
            focus: cx.focus_handle(),
            focus_subscriptions: Vec::new(),
            scroll: ListScrollHandle::default(),
            selection: Selection::default(),
            dragging: false,
            interactive: true,
            checked: BTreeSet::new(),
            width: None,
            key_settings: KeySettings::default(),
            key_form_revision: 0,
            key_editors: None,
        }
    }
    pub fn worktree_write_pending(&self, cx: &App) -> bool {
        matches!(self.mutation, Some(ViewerMutation::StageOrDiscard))
            || self.file_viewers.iter().any(|child| {
                child.view.read(cx).worktree_write_pending(cx)
            })
    }

    pub fn is_busy(&self, cx: &App) -> bool {
        self.mutation.is_some()
            || self
                .file_viewers
                .iter()
                .any(|child| child.view.read(cx).is_busy(cx))
    }

    pub fn set_interactive(&mut self, enabled: bool) {
        self.interactive = enabled;
        if !enabled {
            self.cancel_drag();
        }
    }
    pub fn cancel_drag(&mut self) {
        self.dragging = false;
        self.scroll.cancel_drag();
    }
    pub fn deactivate(&mut self, cx: &mut Context<Self>) {
        for child in &mut self.file_viewers {
            child.active = false;
            child.view.update(cx, |view, cx| view.deactivate(cx));
        }
        self.reads.cancel();
        self.list_loading.reset();
        if self.target.is_none() && !self.dragging && !self.loading {
            return;
        }
        self.cancel_drag();
        self.generation = self.generation.wrapping_add(1);
        self.target = None;
        self.loading = false;
        self.retaining_content = false;
        cx.notify();
    }
    pub fn set_target(
        &mut self,
        target: ViewerTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.load_target(target, false, window, cx);
    }

    pub fn refresh_target(&mut self, target: ViewerTarget, window: &mut Window, cx: &mut Context<Self>) {
        self.load_target(target, true, window, cx);
    }

    fn load_target(
        &mut self,
        target: ViewerTarget,
        force: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let encoding = crate::text_encoding::TextEncoding::current(cx);
        if !force && self.target.as_ref() == Some(&target) && self.encoding == encoding {
            return;
        }
        self.reads.cancel();
        self.list_loading.reset();
        self.retaining_content = prepare_content_transition(
            &mut self.data, self.target.as_ref(), &target, self.encoding == encoding,
        );
        self.encoding = encoding;
        self.target = Some(target.clone());
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.cancel_drag();
        self.checked.clear();
        self.selection = Selection::default();
        self.active_file = None;
        self.error = None;
        if !self.retaining_content {
            self.width = None;
            self.scroll.reset_for_content();
        }
        for child in &mut self.file_viewers {
            child.active = false;
        }
        if let Some(targets) = target.file_targets() {
            for file_target in &targets {
                let position = self
                    .file_viewers
                    .iter()
                    .position(|child| child.target.same_file(file_target));
                let position = position.unwrap_or_else(|| {
                    let view = cx.new(Self::new);
                    let events = cx.subscribe(&view, |_, _, event: &ViewerChanged, cx| {
                        cx.emit(event.clone())
                    });
                    let discard = cx.subscribe(&view, |_, _, event: &ViewerDiscardRequest, cx| {
                        cx.emit(event.clone())
                    });
                    let errors =
                        cx.subscribe(&view, |this, view, event: &ViewerWriteFailed, cx| {
                            if this.file_viewers.iter().any(|child| {
                                child.view.entity_id() == view.entity_id() && !child.active
                            }) {
                                this.error = Some(event.message.clone());
                            }
                            cx.notify();
                        });
                    let observer = cx.observe(&view, |_, _, cx| cx.notify());
                    self.file_viewers.push(FileViewer {
                        target: file_target.clone(),
                        view,
                        active: false,
                        _events: events,
                        _discard: discard,
                        _errors: errors,
                        _observer: observer,
                    });
                    self.file_viewers.len() - 1
                });
                let child = &mut self.file_viewers[position];
                child.active = true;
                child.target = file_target.clone();
                child.view.update(cx, |view, cx| {
                    view.hide_header = true;
                    view.set_interactive(self.interactive);
                    view.load_target(file_target.clone(), force, window, cx);
                });
            }
            self.file_viewers.sort_by_key(|child| {
                targets
                    .iter()
                    .position(|target| child.target.same_file(target))
                    .unwrap_or(usize::MAX)
            });
            self.file_viewers
                .retain(|child| child.active || child.view.read(cx).is_busy(cx));
            self.loading = false;
            cx.notify();
            return;
        }
        self.file_viewers
            .retain(|child| child.view.read(cx).is_busy(cx));
        if matches!(target, ViewerTarget::Text { .. }) {
            self.data = target
                .load_with_encoding(encoding)
                .map(|(data, _)| data)
                .unwrap_or_default();
            self.loading = false;
            cx.notify();
            return;
        }
        self.loading = true;
        self.list_loading
            .begin(generation, std::time::Instant::now());
        let completed = self.list_loading.completion();
        let expected = target.clone();
        let task = self.reads.spawn(cx.background_executor(), move || {
            let result = target.load_with_encoding(encoding);
            completed.store(true, std::sync::atomic::Ordering::Release);
            result
        });
        cx.spawn_in(window, async move |this, cx| {
            let Some(result) = task.await else {
                return;
            };
            let _ = this.update_in(cx, |this, _, cx| {
                if !current_request(this.generation, this.target.as_ref(), generation, &expected)
                    || crate::text_encoding::TextEncoding::current(cx) != encoding
                {
                    return;
                }
                this.loading = false;
                this.retaining_content = false;
                this.list_loading.reset();
                this.width = None;
                this.scroll.reset_for_content();
                match result {
                    Ok((data, file)) => {
                        this.data = data;
                        this.active_file = file;
                    }
                    Err(error) => { this.data = Diff::default(); this.error = Some(format!("조회하지 못했습니다: {error:#}")); },
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(target) = self.target.clone() {
            self.load_target(target, true, window, cx);
        }
    }
    fn details(&self) -> bool {
        self.target.as_ref().is_some_and(ViewerTarget::details)
    }
    fn partial(&self) -> bool {
        self.data.partial.is_some() && !self.loading
    }
    fn text_left(&self) -> f32 {
        if self.details() {
            12.
        } else {
            116. + if self.data.partial.is_some() { 24. } else { 0. }
        }
    }
    fn identity_at(&self, row: usize) -> Option<&DetailIdentity> {
        self.data
            .identities
            .iter()
            .find(|identity| row >= identity.row && row - identity.row < 2)
    }
    fn editor_bounds(&self) -> Bounds<Pixels> {
        editor_viewport(
            self.scroll.viewport_bounds(),
            if self.details() { 0. } else { self.text_left() },
        )
    }
    fn row_text_left(&self, row: usize) -> f32 {
        self.text_left()
            + if self.identity_at(row).is_some() {
                48.
            } else {
                0.
            }
    }
    fn lines(&self) -> Vec<String> {
        self.data
            .lines
            .iter()
            .map(|line| line.text.clone())
            .collect()
    }
    fn check_menu_rows(&self, request: &CheckMenuRequest) -> Option<std::ops::Range<usize>> {
        request.rows(
            self.generation,
            &self.data,
            self.interactive && self.mutation.is_none() && self.key_editors.is_none() && self.partial(),
        )
    }

    fn copy_selected_text(&self, cx: &mut Context<Self>) {
        let text = self.selection.text(&self.lines());
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn open_text_menu(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.interactive || self.loading || self.key_editors.is_some() {
            return;
        }
        self.focus.focus(window);
        let request = CheckMenuRequest {
            generation: self.generation,
            rows: self.selection.rows(self.data.lines.len()),
        };
        let generation = self.generation;
        let owner = cx.weak_entity();
        let mut items = vec![crate::context_menu::MenuItem::new(
            "복사",
            "Ctrl + C",
            !self.selection.text(&self.lines()).is_empty(),
            move |window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if window.is_window_active()
                        && this.interactive
                        && !this.loading
                        && this.generation == generation
                    {
                        this.copy_selected_text(cx);
                    }
                });
            },
        )];
        if self.check_menu_rows(&request).is_some() {
            let owner = cx.weak_entity();
            items.push(crate::context_menu::MenuItem::new(
                "선택한 줄 체크",
                "Space",
                true,
                move |window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if window.is_window_active()
                            && let Some(rows) = this.check_menu_rows(&request)
                        {
                            this.toggle(rows, cx);
                        }
                    });
                },
            ));
        }
        crate::context_menu::open(event.position, items, window, cx);
        cx.stop_propagation();
    }

    fn toggle(&mut self, rows: impl IntoIterator<Item = usize>, cx: &mut Context<Self>) {
        if !self.interactive || !self.partial() || self.mutation.is_some() {
            return;
        }
        toggle_checked_rows(&mut self.checked, &self.data.lines, rows);
        cx.notify();
    }
    fn request_discard_checked(&self, cx: &mut Context<Self>) {
        if !self.interactive || self.mutation.is_some() || !self.partial() || self.checked.is_empty() {
            return;
        }
        let Some(
            target @ ViewerTarget::File {
                historical: false, ..
            },
        ) = self.target.clone()
        else {
            return;
        };
        let ViewerTarget::File { root, .. } = &target else {
            return;
        };
        let (Some(file), Some(preview)) = (self.active_file.clone(), self.data.partial.clone())
        else {
            return;
        };
        cx.emit(ViewerDiscardRequest {
            view: cx.weak_entity(),
            generation: self.generation,
            root: root.clone(),
            target,
            file,
            preview,
            lines: self.data.lines.clone(),
            rows: self.checked.iter().copied().collect(),
        });
    }

    fn stage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.interactive || !self.partial() || self.mutation.is_some() {
            return;
        }
        let Some(ViewerTarget::File {
            root,
            historical: false,
            ..
        }) = self.target.clone()
        else {
            return;
        };
        let Some(file) = self.active_file.clone() else {
            return;
        };
        let rows: Vec<_> = self.checked.iter().copied().collect();
        if rows.is_empty() {
            return;
        }
        let data = self.data.clone();
        self.mutate(root, ViewerMutation::StageOrDiscard, window, cx, move |root| {
            partial_stage::apply(
                root,
                &file,
                data.partial.as_ref().unwrap(),
                &data.lines,
                &rows,
            )
        });
    }
    fn resolve(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        if !self.interactive || self.loading || self.mutation.is_some() {
            return;
        }
        let Some(ViewerTarget::File {
            root,
            historical: false,
            ..
        }) = self.target.clone()
        else {
            return;
        };
        let Some(file) = self.active_file.clone() else {
            return;
        };
        let Some(region) = self.data.conflict_at_row(self.selection.cursor.row) else {
            return;
        };
        let Some(preview) = self.data.conflict.clone() else {
            return;
        };
        self.mutate(root, ViewerMutation::ResolveConflict, window, cx, move |root| {
            conflict::apply(root, &file.path, &preview, region, choice)
        });
    }
    fn mutate(
        &mut self,
        root: PathBuf,
        mutation: ViewerMutation,
        window: &mut Window,
        cx: &mut Context<Self>,
        work: impl FnOnce(&std::path::Path) -> anyhow::Result<()> + Send + 'static,
    ) {
        if self.mutation.is_some() {
            return;
        }
        self.mutation = Some(mutation);
        self.error = None;
        cx.notify();
        let original = root.clone();
        let target = self.target.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || work(&root));
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.mutation = None;
                match result {
                    Ok(()) => {
                        if this.target == target {
                            this.reload(window, cx);
                        }
                        cx.emit(ViewerChanged { root: original });
                    }
                    Err(error) => {
                        this.error = Some(format!(
                            "{}: 변경을 적용하지 못했습니다: {error:#}",
                            crate::path_display::display_path(&original)
                        ));
                        let title = target
                            .as_ref()
                            .map_or("Diff Viewer", ViewerTarget::header_title);
                        cx.emit(ViewerWriteFailed {
                            message: format!("{title}: {}", this.error.as_ref().unwrap()),
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.interactive || self.loading {
            return;
        }
        if self.key_editors.is_some() {
            if event.keystroke.key == "escape" {
                self.key_form_revision = self.key_form_revision.wrapping_add(1);
                self.key_editors = None;
                self.focus.focus(window);
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }
        if !self.focus.is_focused(window) {
            return;
        }
        let Some(command) = self.key_settings.keys.command(&event.keystroke) else {
            return;
        };
        match command {
            "copy" => self.copy_selected_text(cx),
            "all" => {
                if let Some(last) = self.data.lines.len().checked_sub(1) {
                    self.selection.anchor = Position::default();
                    self.selection.cursor = Position {
                        row: last,
                        column: self.data.lines[last].text.chars().count(),
                    };
                }
            }
            "toggle" => self.toggle(self.selection.rows(self.data.lines.len()), cx),
            "stage" => self.stage(window, cx),
            "refresh" => self.reload(window, cx),
            key => {
                self.selection.move_cursor_on_rows(
                    &self.lines(),
                    &(0..self.data.lines.len()).collect::<Vec<_>>(),
                    key,
                    event.keystroke.modifiers.shift,
                );
                self.reveal_cursor(window);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn reveal_cursor(&self, window: &Window) {
        if self.data.lines.is_empty() {
            return;
        }
        self.scroll
            .vertical
            .scroll_to_item(self.selection.cursor.row, ScrollStrategy::Center);
        let raw = &self.data.lines[self.selection.cursor.row].text;
        let shaped = shape_styled_line(window, &raw.replace('\t', "    "), TEXT, 0);
        let x = px(self.row_text_left(self.selection.cursor.row))
            + shaped
                .unwrapped_layout
                .x_for_index(displayed_byte(raw, self.selection.cursor.column));
        let handle = &self.scroll.horizontal;
        let mut offset = handle.offset();
        let width = self.scroll.viewport_bounds().size.width;
        offset.x = px(cursor_horizontal_offset(
            x.into(),
            offset.x.into(),
            width.into(),
            if self.details() { 0. } else { self.text_left() },
            handle.max_offset().width.into(),
        ));
        handle.set_offset(offset);
    }
    fn drag_to(&mut self, event: &MouseMoveEvent, window: &Window, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) {
            self.dragging = false;
            return;
        }
        let frame = self.scroll.viewport_bounds();
        let bounds = self.editor_bounds();
        if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) || self.data.lines.is_empty()
        {
            return;
        }
        let delta = |position: Pixels, start: Pixels, length: Pixels| {
            if position < start {
                (start - position).min(px(48.))
            } else if position > start + length {
                -(position - start - length).min(px(48.))
            } else {
                px(0.)
            }
        };
        let vertical = self.scroll.vertical.0.borrow().base_handle.clone();
        let mut y = vertical.offset();
        y.y = (y.y + delta(event.position.y, bounds.origin.y, bounds.size.height))
            .clamp(-vertical.max_offset().height, px(0.));
        vertical.set_offset(y);
        let mut x = self.scroll.horizontal.offset();
        x.x = (x.x + delta(event.position.x, bounds.origin.x, bounds.size.width))
            .clamp(-self.scroll.horizontal.max_offset().width, px(0.));
        self.scroll.horizontal.set_offset(x);
        let row = (((event.position.y.clamp(
            bounds.origin.y,
            bounds.origin.y + bounds.size.height - px(0.01),
        ) - bounds.origin.y
            - y.y)
            / px(ROW_HEIGHT))
        .max(0.) as usize)
            .min(self.data.lines.len() - 1);
        let raw = &self.data.lines[row].text;
        let shaped = shape_styled_line(window, &raw.replace('\t', "    "), TEXT, 0);
        let local = event.position.x.clamp(
            bounds.origin.x,
            bounds.origin.x + bounds.size.width - px(0.01),
        ) - frame.origin.x
            - x.x
            - px(self.row_text_left(row));
        let byte = shaped
            .unwrapped_layout
            .closest_index_for_x(local.max(px(0.)));
        self.selection.cursor = Position {
            row,
            column: original_column(raw, byte),
        };
        cx.notify();
    }
    fn register_drag_events(&self, window: &mut Window, cx: &Context<Self>) {
        let weak = cx.weak_entity();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Capture {
                let _ = weak.update(cx, |this, cx| {
                    if this.dragging {
                        this.drag_to(event, window, cx);
                        cx.stop_propagation();
                    }
                });
            }
        });
        let weak = cx.weak_entity();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                let _ = weak.update(cx, |this, cx| {
                    if this.dragging {
                        this.dragging = false;
                        cx.notify();
                    }
                });
            }
        });
    }
    fn selectable_text(&self, index: usize, raw: String, color: u32, cx: &Context<Self>) -> Div {
        let selection = self.selection.clone();
        let focus = self.focus.clone();
        let horizontal = self.scroll.horizontal.clone();
        let fixed = !self.details();
        let eof = fixed && self.data.eof_rows.contains(&index);
        let view = cx.weak_entity();
        let bold = if self.details()
            && index
                < self
                    .data
                    .lines
                    .iter()
                    .position(|line| line.text.is_empty())
                    .unwrap_or(self.data.lines.len())
        {
            detail_label_len(&raw)
        } else {
            0
        };
        div()
            .h_full()
            .flex_1()
            .min_w_0()
            .cursor(CursorStyle::IBeam)
            .overflow_hidden()
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        let offset = if fixed { horizontal.offset().x } else { px(0.) };
                        let text_origin = bounds.origin + point(offset, px(0.));
                        let displayed = raw.replace('\t', "    ");
                        let shaped = shape_styled_line(window, &displayed, color, bold);
                        let x_for = |column| {
                            shaped
                                .unwrapped_layout
                                .x_for_index(displayed_byte(&raw, column))
                        };
                        let (start, end) = selection.range();
                        if index >= start.row && index <= end.row && start != end {
                            let from = if index == start.row { start.column } else { 0 };
                            let to = if index == end.row {
                                end.column
                            } else {
                                raw.chars().count()
                            };
                            window.paint_quad(fill(
                                Bounds::new(
                                    text_origin + point(x_for(from), px(0.)),
                                    size(
                                        (x_for(to) - x_for(from)
                                            + if index < end.row { px(7.) } else { px(0.) })
                                        .max(px(0.)),
                                        px(ROW_HEIGHT),
                                    ),
                                ),
                                rgb(SELECTED),
                            ));
                        }
                        if window.is_window_active()
                            && focus.is_focused(window)
                            && selection.cursor.row == index
                        {
                            window.paint_quad(fill(
                                Bounds::new(
                                    text_origin + point(x_for(selection.cursor.column), px(2.)),
                                    size(px(1.), px(ROW_HEIGHT - 4.)),
                                ),
                                rgb(ACCENT),
                            ));
                        }
                        let _ = shaped.paint(
                            text_origin,
                            px(ROW_HEIGHT),
                            TextAlign::Left,
                            None,
                            window,
                            cx,
                        );
                        if fixed {
                            let dot = shape_styled_line(window, "·", MUTED, 0);
                            let arrow = shape_styled_line(window, "→", MUTED, 0);
                            let visible_start = shaped
                                .unwrapped_layout
                                .index_for_x((-offset).max(px(0.)))
                                .unwrap_or(displayed.len());
                            let visible_end = shaped
                                .unwrapped_layout
                                .closest_index_for_x((bounds.size.width - offset).max(px(0.)));
                            for (start, end, marker) in invisible_markers(&raw)
                                .skip_while(|(_, end, _)| *end < visible_start)
                                .take_while(|(start, _, _)| *start <= visible_end)
                            {
                                let marker = if marker == "·" { &dot } else { &arrow };
                                let x = (shaped.unwrapped_layout.x_for_index(start)
                                    + shaped.unwrapped_layout.x_for_index(end)
                                    - marker.unwrapped_layout.width)
                                    / 2.;
                                let _ = marker.paint(
                                    text_origin + point(x, px(0.)),
                                    px(ROW_HEIGHT),
                                    TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                );
                            }
                            if eof {
                                let marker = shape_styled_line(window, " <EOF>", MUTED, 0);
                                let _ = marker.paint(
                                    text_origin + point(x_for(raw.chars().count()), px(0.)),
                                    px(ROW_HEIGHT),
                                    TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                );
                            }
                        }
                        let weak = view.clone();
                        let raw = raw.clone();
                        let layout = shaped.clone();
                        let displayed_len = displayed.len();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture
                                || !matches!(event.button, MouseButton::Left | MouseButton::Right)
                                || crate::context_menu::active(cx).is_some()
                                || !bounds.contains(&event.position)
                            {
                                return;
                            }
                            let _ = weak.update(cx, |this, cx| {
                                if !this.interactive
                                    || !this.scroll.viewport_bounds().contains(&event.position)
                                    || !this.scroll.is_visible_at(event.position)
                                    || this.scroll.is_scrollbar_at(event.position)
                                    || this.loading
                                    || this.key_editors.is_some()
                                {
                                    return;
                                }
                                if event.button == MouseButton::Right {
                                    if text_menu_hit(
                                        event.position,
                                        bounds,
                                        this.editor_bounds(),
                                        text_origin,
                                        layout.unwrapped_layout.width,
                                        raw.trim().is_empty(),
                                    ) {
                                        this.open_text_menu(event, window, cx);
                                    }
                                    return;
                                }
                                let byte = layout
                                    .unwrapped_layout
                                    .closest_index_for_x(event.position.x - text_origin.x);
                                let pos = Position {
                                    row: index,
                                    column: original_column(&raw, byte),
                                };
                                if event.click_count == 2 {
                                    let byte = layout
                                        .unwrapped_layout
                                        .index_for_x(event.position.x - text_origin.x)
                                        .unwrap_or(displayed_len);
                                    this.selection.select_word(
                                        &raw,
                                        Position {
                                            row: index,
                                            column: original_character_column(&raw, byte),
                                        },
                                    );
                                    this.dragging = false;
                                } else {
                                    if !event.modifiers.shift {
                                        this.selection.anchor = pos;
                                    }
                                    this.selection.cursor = pos;
                                    this.dragging = true;
                                }
                                this.focus.focus(window);
                                cx.stop_propagation();
                                cx.notify();
                            });
                        });
                    },
                )
                .size_full(),
            )
    }
    fn gutter_text(text: String, color: u32) -> impl IntoElement {
        canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                let shaped = shape_styled_line(window, &text, color, 0);
                let origin = bounds.origin
                    + point(
                        (bounds.size.width - shaped.unwrapped_layout.width) / 2.,
                        px(0.),
                    );
                let _ = shaped.paint(origin, px(ROW_HEIGHT), TextAlign::Left, None, window, cx);
            },
        )
        .size_full()
    }

    fn render_files(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.is_busy(cx);
        for child in &self.file_viewers {
            let own_busy = child.view.read(cx).is_busy(cx);
            child.view.update(cx, |view, _| {
                view.set_interactive(self.interactive && child.active && (!busy || own_busy))
            });
        }
        let sections = self
            .file_viewers
            .iter()
            .filter(|child| child.active)
            .map(|child| {
                div()
                    .flex()
                    .flex_col()
                    .h(px(440.))
                    .flex_shrink_0()
                    .min_w_0()
                    .min_h_0()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .child(child.view.clone())
            });
        div()
            .id("multi-file-viewer")
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(rgb(PANEL))
            .track_focus(&self.focus)
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, window, _| {
                if event.button == MouseButton::Left && this.interactive {
                    this.focus.focus(window);
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(36.))
                    .px_5()
                    .gap_3()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .text_color(rgb(TEXT))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!(
                                "선택한 파일 {}개",
                                self.file_viewers
                                    .iter()
                                    .filter(|child| child.active)
                                    .count()
                            )),
                    ),
            )
            .when_some(self.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .px_3()
                        .py_1()
                        .text_size(px(12.))
                        .text_color(rgb(DELETE_TEXT))
                        .child(error),
                )
            })
            .child(
                div()
                    .id("multi-file-stack")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(sections),
            )
    }

    fn empty_eof_row(&self, cx: &Context<Self>) -> Div {
        let view = cx.weak_entity();
        let horizontal = self.scroll.horizontal.clone();
        div()
            .flex()
            .w_full()
            .h_full()
            .child(div().w(px(self.text_left())).flex_shrink_0())
            .child(
                div()
                    .id("empty-eof-editor")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            if this.scroll.is_visible_at(event.position)
                                && this.editor_bounds().contains(&event.position)
                                && !this.scroll.is_scrollbar_at(event.position)
                            {
                                this.open_text_menu(event, window, cx);
                            }
                        });
                    })
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, cx| {
                                let marker = shape_styled_line(window, "<EOF>", MUTED, 0);
                                let _ = marker.paint(
                                    bounds.origin + point(horizontal.offset().x, px(0.)),
                                    px(ROW_HEIGHT),
                                    TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                );
                            },
                        )
                        .size_full(),
                    ),
            )
    }

    fn diff_row(&self, index: usize, width: f32, cx: &Context<Self>) -> Div {
        let line = &self.data.lines[index];
        let (bg, fg) = line_colors(line.kind);
        let partial = self.data.partial.is_some();
        let row = div()
            .flex()
            .w_full()
            .min_w(px(if self.details() { width } else { 0. }))
            .h_full()
            .whitespace_nowrap()
            .bg(rgb(if self.details() { PANEL } else { bg }))
            .text_size(px(12.))
            .font_family("Cascadia Mono");
        if self.details() {
            return row
                .px_3()
                .when_some(self.identity_at(index), |row, identity| {
                    row.child(identity_avatar(identity, index))
                })
                .child(self.selectable_text(index, line.text.clone(), TEXT, cx));
        }
        let cell = |width: f32| {
            div()
                .w(px(width))
                .h_full()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(RAISED))
                .text_color(rgb(MUTED))
                .border_r_1()
                .border_color(rgb(LINE))
        };
        row.when(partial, |row| {
            row.child(cell(24.).when(matches!(line.kind, '+' | '-'), |cell| {
                cell.child(
                    div()
                        .id(("check", index))
                        .cursor_pointer()
                        .child(crate::form_controls::checkbox_mark(
                            self.checked.contains(&index),
                        ))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle([index], cx);
                                cx.stop_propagation();
                            }),
                        ),
                )
            }))
        })
        .when(partial && line.kind == '@', |row| {
            row.child(
                cell(96.).child(
                    div()
                        .id(("hunk", index))
                        .cursor_pointer()
                        .text_color(rgb(ACCENT))
                        .child("구간 체크")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle(partial_stage::hunk_rows(&this.data.lines, index), cx);
                                cx.stop_propagation();
                            }),
                        ),
                ),
            )
        })
        .when(!(partial && line.kind == '@'), |row| {
            row.child(cell(48.).child(Self::gutter_text(
                line.old.map(|n| n.to_string()).unwrap_or_default(),
                MUTED,
            )))
            .child(cell(48.).child(Self::gutter_text(
                line.new.map(|n| n.to_string()).unwrap_or_default(),
                MUTED,
            )))
        })
        .child(cell(20.).child(Self::gutter_text(line.kind.to_string(), fg)))
        .child(self.selectable_text(index, line.text.clone(), fg, cx))
    }
    fn save_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editors) = &self.key_editors else {
            return;
        };
        let entries = editors
            .iter()
            .map(|(id, editor)| (id.clone(), editor.read(cx).text()))
            .collect::<Vec<_>>();
        let keys = match ViewerKeys::validate(&entries) {
            Ok(keys) => keys,
            Err(error) => {
                self.key_settings.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let Some(revision) = self.key_settings.begin_save() else {
            return;
        };
        let form_revision = self.key_form_revision;
        let submitted = keys.clone();
        let task = crate::backend::spawn(cx.background_executor(), move || save_keys(&submitted));
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let unchanged = this.key_form_revision == form_revision
                    && this.key_editors.as_ref().is_some_and(|editors| {
                        editors
                            .iter()
                            .map(|(id, editor)| (id.clone(), editor.read(cx).text()))
                            .collect::<Vec<_>>()
                            == entries
                    });
                if this.key_settings.finish_save(revision, keys, result) && unchanged {
                    this.key_editors = None;
                    this.focus.focus(window);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
fn line_colors(kind: char) -> (u32, u32) {
    match kind {
        '+' => (ADD_BG, ADD_TEXT),
        '-' => (DELETE_BG, DELETE_TEXT),
        '@' => (HEADER_BG, HEADER_TEXT),
        _ => (PANEL, TEXT),
    }
}
fn control(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .py_1()
        .text_size(px(11.))
        .rounded_sm()
        .border_1()
        .border_color(rgb(LINE))
        .bg(rgb(RAISED))
        .cursor_pointer()
        .when(!enabled, |el| el.opacity(0.4))
        .child(text.into())
}
impl Render for DiffViewer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.encoding_subscription.is_none() {
            self.encoding_subscription =
                Some(cx.observe_global_in::<crate::text_encoding::TextEncoding>(
                    window,
                    |this, window, cx| {
                        if this.encoding != crate::text_encoding::TextEncoding::current(cx) {
                            this.reload(window, cx);
                        }
                    },
                ));
        }
        if self.encoding != crate::text_encoding::TextEncoding::current(cx) {
            self.reload(window, cx);
        }
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions = vec![
                cx.on_focus(&self.focus, window, |_, _, cx| cx.notify()),
                cx.on_blur(&self.focus, window, |this, _, cx| {
                    this.cancel_drag();
                    cx.notify();
                }),
                cx.observe_window_activation(window, |this, window, cx| {
                    if !window.is_window_active() {
                        this.cancel_drag();
                    }
                    cx.notify();
                }),
            ];
        }

        self.file_viewers
            .retain(|child| child.active || child.view.read(cx).is_busy(cx));
        if matches!(self.target, Some(ViewerTarget::Files { .. })) {
            return self.render_files(cx).into_any_element();
        }
        for child in &self.file_viewers {
            child.view.update(cx, |view, _| view.set_interactive(false));
        }
        self.scroll
            .set_suspended(self.key_editors.is_some() || !self.interactive);
        if self.width.is_none() && !self.loading {
            let measured: Vec<_> = self
                .data
                .lines
                .iter()
                .enumerate()
                .map(|(row, line)| {
                    if !self.details() && self.data.eof_rows.contains(&row) {
                        std::borrow::Cow::Owned(format!("{} <EOF>", line.text))
                    } else {
                        std::borrow::Cow::Borrowed(line.text.as_str())
                    }
                })
                .chain(
                    (!self.details() && self.data.empty_eof)
                        .then_some(std::borrow::Cow::Borrowed("<EOF>")),
                )
                .collect();
            self.width = Some(measure_text_width(
                window,
                measured.iter().map(|line| line.as_ref()),
                self.text_left()
                    + 16.
                    + if self.data.identities.is_empty() {
                        0.
                    } else {
                        48.
                    },
                self.details(),
            ));
        }
        let width = self.width.unwrap_or(0.);
        let partial = self.data.partial.is_some();
        let count = self.data.lines.len();
        let empty_eof = !self.details() && self.data.empty_eof;
        let show_empty_message = !self.loading && count == 0 && !empty_eof;
        let busy = self.mutation.is_some();
        let has_conflicts = self.data.conflict.is_some();
        let region = self.data.conflict_at_row(self.selection.cursor.row);
        let title = self
            .target
            .as_ref()
            .map_or("Diff Viewer", ViewerTarget::header_title)
            .to_owned();
        let weak = cx.weak_entity();
        let mut panel = div()
            .id("diff-viewer")
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .bg(rgb(PANEL))
            .text_color(rgb(TEXT))
            .track_focus(&self.focus)
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, window, _| {
                if event.button == MouseButton::Left && this.interactive {
                    this.focus.focus(window);
                }
            }))
            .key_context("DiffViewer")
            .on_key_down(cx.listener(Self::key_down))
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, cx| {
                        let _ = weak.update(cx, |this, cx| this.register_drag_events(window, cx));
                    },
                )
                .absolute()
                .size_full(),
            )
            .when(!self.hide_header, |panel| {
                panel.child(
                    div()
                        .flex()
                        .items_center()
                        .h(px(36.))
                        .px_5()
                        .gap_3()
                        .flex_shrink_0()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        ),
                )
            })
            .when(partial, |panel| {
                panel.child(
                    div()
                        .flex()
                        .items_center()
                        .flex_wrap()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .text_size(px(11.))
                        .flex_shrink_0()
                        .child(format!("변경 줄 {}개 선택", self.checked.len()))
                        .child(
                            control(
                                "stage-lines",
                                stage_button_label(&self.key_settings.keys),
                                !busy && !self.checked.is_empty(),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.stage(window, cx);
                                    cx.stop_propagation();
                                }),
                            ),
                        )
                        .child(
                            control(
                                "discard-lines",
                                "체크한 줄 폐기",
                                !busy && !self.checked.is_empty(),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.request_discard_checked(cx);
                                    cx.stop_propagation();
                                }),
                            ),
                        ),
                )
            })
            .when(has_conflicts, |panel| {
                let mut toolbar = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .flex_shrink_0()
                    .child(region.map_or_else(
                        || "충돌 구간에 커서를 놓으세요".into(),
                        |r| {
                            format!(
                                "충돌 {} / {}",
                                r + 1,
                                self.data.conflict.as_ref().unwrap().regions.len()
                            )
                        },
                    ));
                for (id, label, choice) in [
                    ("current", "위쪽(현재) 변경 사용", Choice::Current),
                    ("incoming", "아래쪽(대상) 변경 사용", Choice::Incoming),
                    ("both", "양쪽 모두 사용", Choice::Both),
                ] {
                    toolbar =
                        toolbar.child(control(id, label, !busy && region.is_some()).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                this.resolve(choice, window, cx);
                                cx.stop_propagation();
                            }),
                        ));
                }
                panel.child(toolbar)
            })
            .when(busy, |panel| {
                panel.child(
                    div()
                        .px_3()
                        .text_size(px(11.))
                        .child("변경을 적용하는 중입니다…"),
                )
            })
            .when_some(self.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .px_3()
                        .py_1()
                        .text_size(px(12.))
                        .text_color(rgb(DELETE_TEXT))
                        .child(error),
                )
            })
            .when(show_empty_message, |panel| {
                panel.child(panel_message(if self.error.is_some() {
                    "오류 내용을 확인한 뒤 다시 조회하세요."
                } else if !self.data.note.is_empty() {
                    "표시할 텍스트 차이가 없습니다."
                } else {
                    "파일을 선택해 변경 내용을 확인하세요."
                }))
            })
            .when(!show_empty_message, |panel| {
                panel.child(
                    virtual_list(
                        "viewer-lines",
                        count + usize::from(empty_eof),
                        None,
                        PANEL,
                        cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                            range
                                .map(|i| {
                                    if i == this.data.lines.len() {
                                        this.empty_eof_row(cx)
                                    } else {
                                        this.diff_row(i, width, cx)
                                    }
                                })
                                .collect::<Vec<_>>()
                        }),
                        |_, _, _| {},
                    )
                    .keyboard_selection(false)
                    .scroll_handle(self.scroll.clone())
                    .loading(self.list_loading.clone())
                    .keep_content_while_loading(true)
                    .min_content_width(width)
                    .fixed_left_width(if self.details() { 0. } else { self.text_left() }),
                )
            })
            .when(!self.data.note.is_empty(), |panel| {
                panel.child(
                    div()
                        .px_3()
                        .py_2()
                        .flex_shrink_0()
                        .text_size(px(11.))
                        .text_color(rgb(MUTED))
                        .child(self.data.note.clone()),
                )
            });
        if let Some(editors) = &self.key_editors {
            let form = div()
                .id("viewer-key-form")
                .occlude()
                .absolute()
                .inset_0()
                .bg(rgb(PANEL))
                .p_3()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_2()
                .child("뷰어 단축키 · Shift로 범위 확장 · Esc로 닫기")
                .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(
                    "기본/보조 키를 지정하세요 (예: ctrl-c, alt-y). 비우면 해당 키를 해제합니다.",
                ))
                .children(editors.iter().map(|(id, editor)| {
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().w(px(130.)).text_size(px(12.)).child(format!(
                                "{}{}",
                                COMMANDS
                                    .iter()
                                    .find(|(key, _, _)| *key
                                        == id.strip_suffix("-secondary").unwrap_or(id))
                                    .unwrap()
                                    .1,
                                if id.ends_with("-secondary") {
                                    " (보조)"
                                } else {
                                    ""
                                }
                            )))
                        .child(
                            div()
                                .flex_1()
                                .child(crate::form_controls::input(editor.clone())),
                        )
                }))
                .when_some(self.key_settings.error.clone(), |el, error| {
                    el.child(div().text_color(rgb(DELETE_TEXT)).child(error))
                })
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            crate::dialog::button("save-keys", "적용", true, true).on_click(
                                cx.listener(|this, _, window, cx| this.save_keys(window, cx)),
                            ),
                        )
                        .child(
                            crate::dialog::button("reset-keys", "기본값", false, true).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.key_form_revision = this.key_form_revision.wrapping_add(1);
                                    if let Some(editors) = &this.key_editors {
                                        for (id, editor) in editors {
                                            editor.update(cx, |input, cx| {
                                                input.set_text(
                                                    &ViewerKeys::default().binding(id),
                                                    cx,
                                                )
                                            });
                                        }
                                    }
                                    this.key_settings.error = None;
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(
                            crate::dialog::button("close-keys", "닫기", false, true).on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.key_form_revision = this.key_form_revision.wrapping_add(1);
                                    this.key_editors = None;
                                    this.focus.focus(window);
                                    cx.notify();
                                }),
                            ),
                        ),
                );
            panel = panel.child(form);
        }
        panel.into_any_element()
    }
}

fn detail_label_len(text: &str) -> usize {
    text.split_once(':')
        .filter(|(label, _)| {
            [
                "커밋",
                "부모 커밋",
                "작성자",
                "작성시간",
                "커미터",
                "커밋시간",
                "authored",
                "committed",
            ]
            .contains(label)
        })
        .map_or(0, |(label, _)| label.len() + 1)
}

fn styled_runs(text: &str, color: u32, bold_len: usize) -> Vec<TextRun> {
    let make_run = |len, bold| {
        let mut face = font("Cascadia Mono");
        face.weight = if bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        };
        TextRun {
            len,
            font: face,
            color: rgb(color).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    };
    if bold_len > 0 {
        vec![
            make_run(bold_len, true),
            make_run(text.len() - bold_len, false),
        ]
    } else {
        vec![make_run(text.len(), false)]
    }
}

fn shape_styled_line(window: &Window, text: &str, color: u32, bold_len: usize) -> WrappedLine {
    let runs = styled_runs(text, color, bold_len);
    // shape_line merges equally colored runs before checking their fonts in GPUI 0.2.2.
    // shape_text compares font IDs and keeps bold labels separate from normal values.
    window
        .text_system()
        .shape_text(
            text.to_owned().into(),
            window.rem_size() * 0.75,
            &runs,
            None,
            None,
        )
        .unwrap_or_default()
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn measure_text_width<'a>(
    window: &Window,
    lines: impl Iterator<Item = &'a str>,
    padding: f32,
    bold_labels: bool,
) -> f32 {
    let mut metadata = bold_labels;
    lines
        .map(|line| {
            let text = line.replace('\t', "    ");
            let len = text.len();
            if text.is_empty() {
                metadata = false;
            }
            let shaped = shape_styled_line(
                window,
                &text,
                TEXT,
                if metadata { detail_label_len(&text) } else { 0 },
            );
            f32::from(shaped.unwrapped_layout.x_for_index(len))
        })
        .fold(0., f32::max)
        + padding
}

fn identity_avatar(identity: &DetailIdentity, row: usize) -> Div {
    let photo = div()
        .absolute()
        .left_0()
        .top(px(4. - (row - identity.row) as f32 * ROW_HEIGHT))
        .size(px(40.))
        .rounded_full()
        .overflow_hidden()
        .bg(rgb(RAISED))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .font_family("Segoe MDL2 Assets")
                .text_size(px(24.))
                .text_color(rgb(MUTED))
                .child("\u{E77B}"),
        )
        .when_some(identity.avatar_url.clone(), |photo, url| {
            photo.child(
                img(url)
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .with_fallback(|| div().into_any_element()),
            )
        });
    div()
        .relative()
        .w(px(48.))
        .h_full()
        .flex_shrink_0()
        .overflow_hidden()
        .child(photo)
}

fn detail_identities(detail: &CommitDetail, repository: Option<&str>) -> Vec<DetailIdentity> {
    let row = 1 + detail.parent_ids.len().max(1);
    vec![
        DetailIdentity {
            row,
            avatar_url: crate::avatar::commit_url(
                repository,
                &detail.id,
                false,
                &detail.author_email,
            ),
        },
        DetailIdentity {
            row: row + 2,
            avatar_url: crate::avatar::commit_url(
                repository,
                &detail.id,
                true,
                &detail.committer_email,
            ),
        },
    ]
}

fn detail_rows(detail: &CommitDetail) -> Vec<String> {
    let mut rows = vec![format!("커밋: {}", detail.id)];
    if detail.parent_ids.is_empty() {
        rows.push("부모 커밋: 없음 · 초기 커밋".into());
    } else {
        rows.extend(
            detail
                .parent_ids
                .iter()
                .map(|id| format!("부모 커밋: {id}")),
        );
    }
    rows.extend([
        format!("{} <{}>", detail.author, detail.author_email),
        format!(
            "authored: {}",
            format_detail_time(detail.author_time, detail.author_offset)
        ),
        format!("{} <{}>", detail.committer, detail.committer_email),
        format!(
            "committed: {}",
            format_detail_time(detail.commit_time, detail.commit_offset)
        ),
        String::new(),
    ]);
    rows.extend(
        detail
            .message
            .split('\n')
            .map(|line| line.trim_end_matches('\r').to_owned()),
    );
    rows
}

fn format_detail_time(seconds: i64, offset: i32) -> String {
    gix::date::Time::new(seconds, offset)
        .format(gix::date::time::format::ISO8601)
        .map(|time| time.chars().take(19).collect::<String>().replace('T', " "))
        .unwrap_or_else(|_| seconds.to_string())
}
fn invisible_markers(raw: &str) -> impl Iterator<Item = (usize, usize, &'static str)> + '_ {
    let mut byte = 0;
    raw.chars().filter_map(move |ch| {
        let start = byte;
        byte += if ch == '\t' { 4 } else { ch.len_utf8() };
        match ch {
            ' ' => Some((start, byte, "·")),
            '\t' => Some((start, byte, "→")),
            _ => None,
        }
    })
}

fn displayed_byte(raw: &str, column: usize) -> usize {
    raw.chars()
        .take(column)
        .map(|character| {
            if character == '\t' {
                4
            } else {
                character.len_utf8()
            }
        })
        .sum()
}

fn text_menu_hit(
    position: Point<Pixels>,
    line: Bounds<Pixels>,
    editor: Bounds<Pixels>,
    origin: Point<Pixels>,
    width: Pixels,
    blank: bool,
) -> bool {
    line.contains(&position)
        && editor.contains(&position)
        && position.x >= origin.x
        && (blank || width > px(0.) && position.x < origin.x + width)
}

fn checkable_rows(data: &Diff, rows: std::ops::Range<usize>) -> bool {
    data.partial.is_some()
        && rows.into_iter().any(|row| {
            data.lines
                .get(row)
                .is_some_and(|line| matches!(line.kind, '+' | '-'))
        })
}

fn toggle_checked_rows(
    checked: &mut BTreeSet<usize>,
    lines: &[DiffLine],
    rows: impl IntoIterator<Item = usize>,
) {
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|row| {
            lines
                .get(*row)
                .is_some_and(|line| matches!(line.kind, '+' | '-'))
        })
        .collect();
    if rows.iter().all(|row| checked.contains(row)) {
        for row in rows {
            checked.remove(&row);
        }
    } else {
        checked.extend(rows);
    }
}

fn editor_viewport(mut bounds: Bounds<Pixels>, fixed_left: f32) -> Bounds<Pixels> {
    let inset = px(fixed_left.max(0.)).min(bounds.size.width.max(px(0.)));
    bounds.origin.x += inset;
    bounds.size.width = (bounds.size.width - inset).max(px(0.));
    bounds
}

fn cursor_horizontal_offset(
    cursor: f32,
    mut offset: f32,
    viewport: f32,
    inset: f32,
    maximum: f32,
) -> f32 {
    if viewport > inset {
        let right = (viewport - 8.).max(inset);
        if cursor + offset < inset {
            offset = inset - cursor;
        } else if cursor + offset > right {
            offset = right - cursor;
        }
    }
    offset.clamp(-maximum.max(0.), 0.)
}

fn stage_button_label(keys: &ViewerKeys) -> String {
    let label = keys.label("stage");
    if label == "미지정" {
        "체크한 줄 스테이지".into()
    } else {
        format!("체크한 줄 스테이지 ({label})")
    }
}

fn original_character_column(raw: &str, byte: usize) -> usize {
    let mut offset = 0;
    for (column, character) in raw.chars().enumerate() {
        offset += if character == '\t' {
            4
        } else {
            character.len_utf8()
        };
        if byte < offset {
            return column;
        }
    }
    raw.chars().count()
}

fn original_column(raw: &str, byte: usize) -> usize {
    let mut offset = 0;
    for (column, character) in raw.chars().enumerate() {
        let next = offset
            + if character == '\t' {
                4
            } else {
                character.len_utf8()
            };
        if byte < next {
            return column + usize::from(byte - offset > (next - offset) / 2);
        }
        offset = next;
    }
    raw.chars().count()
}

fn keys_path() -> anyhow::Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .ok_or_else(|| anyhow::anyhow!("사용자 설정 경로를 찾을 수 없습니다."))?;
    Ok(base.join("feather-git").join("viewer-shortcuts.json"))
}
fn read_keys(path: &std::path::Path) -> anyhow::Result<ViewerKeys> {
    let bytes = match crate::session::read_bytes(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ViewerKeys::default());
        }
        Err(error) => return Err(error.into()),
    };
    let keys: ViewerKeys = serde_json::from_slice(&bytes)?;
    ViewerKeys::validate(&key_entries(&keys))
}
fn load_keys() -> anyhow::Result<ViewerKeys> {
    read_keys(&keys_path()?)
}
fn save_keys(keys: &ViewerKeys) -> anyhow::Result<()> {
    write_keys(&keys_path()?, keys)
}
fn write_keys(path: &std::path::Path, keys: &ViewerKeys) -> anyhow::Result<()> {
    crate::session::write_bytes(path, &serde_json::to_vec_pretty(keys)?)
}

fn prepare_content_transition(
    data: &mut Diff,
    previous: Option<&ViewerTarget>,
    next: &ViewerTarget,
    same_encoding: bool,
) -> bool {
    let retain = same_encoding && !data.lines.is_empty()
        && matches!((previous, next),
            (Some(ViewerTarget::Detail { root: previous, .. }), ViewerTarget::Detail { root: next, .. })
            | (Some(ViewerTarget::File { root: previous, .. }), ViewerTarget::File { root: next, .. })
                if previous == next);
    if !retain {
        *data = Diff::default();
    }
    retain
}

fn current_request(
    generation: u64,
    target: Option<&ViewerTarget>,
    requested_generation: u64,
    requested_target: &ViewerTarget,
) -> bool {
    generation == requested_generation && target == Some(requested_target)
}

fn key_entries(keys: &ViewerKeys) -> Vec<(String, String)> {
    COMMANDS
        .iter()
        .flat_map(|(id, _, _)| {
            let secondary = format!("{id}-secondary");
            [
                (id.to_string(), keys.binding(id)),
                (secondary.clone(), keys.binding(&secondary)),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn commit_detail_transition_preserves_display_until_current_result_and_clears_other_contexts() {
        use crate::diff_model::{Diff, DiffLine};
        use super::prepare_content_transition;
        let detail = |root: &str, id: &str| ViewerTarget::Detail { root: root.into(), id: id.into() };
        let a = detail("repo", "a");
        let b = detail("repo", "b");
        let c = detail("repo", "c");
        let old = Diff { lines: vec![DiffLine { old: None, new: None, kind: ' ', text: "old commit".into() }], ..Default::default() };
        let mut data = old.clone();
        assert!(prepare_content_transition(&mut data, Some(&a), &b, true));
        assert_eq!(data.lines[0].text, "old commit");
        assert!(prepare_content_transition(&mut data, Some(&b), &c, true));
        assert_eq!(data.lines[0].text, "old commit");
        assert!(!super::current_request(3, Some(&c), 2, &b));
        assert!(super::current_request(3, Some(&c), 3, &c));
        let text = ViewerTarget::Text { identity: "empty".into(), title: "empty".into(), lines: vec![] };
        for (previous, next, same_encoding) in [
            (Some(a.clone()), detail("other", "c"), true),
            (Some(a.clone()), b.clone(), false),
            (Some(text.clone()), b.clone(), true),
            (Some(a.clone()), text, true),
            (None, b.clone(), true),
        ] {
            let mut data = old.clone();
            assert!(!prepare_content_transition(&mut data, previous.as_ref(), &next, same_encoding));
            assert!(data.lines.is_empty());
        }
        assert!(!prepare_content_transition(&mut Diff::default(), Some(&a), &b, true));
    }

    #[test]
    fn file_transition_preserves_display_only_within_repository_and_encoding() {
        let file = |root: &str, path: &str| ViewerTarget::File {
            root: root.into(),
            historical: false,
            file: crate::diff_model::FileChange {
                path: path.as_bytes().to_vec(), label: path.into(), status: "M".into(), staged: false,
                before: crate::diff_model::Content::Empty, after: crate::diff_model::Content::Worktree,
            },
        };
        let mut old = super::text_diff(vec!["previous file".into()]);
        old.partial = Some(crate::partial_stage::Preview { before: "before".into(), after: "after".into() });
        let a = file("repo", "a");
        let b = file("repo", "b");
        let mut data = old.clone();
        assert!(super::prepare_content_transition(&mut data, Some(&a), &a, true));
        assert!(super::prepare_content_transition(&mut data, Some(&a), &b, true));
        assert_eq!(data.lines[0].text, "previous file");
        assert!(data.partial.is_some());
        assert!(super::prepare_content_transition(&mut data, Some(&b), &file("repo", "c"), true));
        for (next, same_encoding) in [
            (file("other", "b"), true), (b, false),
            (ViewerTarget::Detail { root: "repo".into(), id: "commit".into() }, true),
        ] {
            let mut data = old.clone();
            assert!(!super::prepare_content_transition(&mut data, Some(&a), &next, same_encoding));
            assert!(data.lines.is_empty());
        }
    }

    #[test]
    fn selected_target_supplies_title_without_waiting_for_backend() {
        use crate::diff_model::{Content, FileChange};
        let mut target = ViewerTarget::File {
            root: std::path::PathBuf::from("missing-repository-no-io"),
            file: FileChange {
                path: b"src/first.rs".to_vec(),
                label: "src/first.rs".into(),
                status: "M".into(),
                staged: false,
                before: Content::Empty,
                after: Content::Worktree,
            },
            historical: false,
        };
        assert_eq!(target.header_title(), "src/first.rs");
        if let ViewerTarget::File { file, .. } = &mut target {
            file.label = "한글/second.rs".into();
        }
        assert_eq!(target.header_title(), "한글/second.rs");
        let detail = ViewerTarget::Detail {
            root: "missing".into(),
            id: "missing".into(),
        };
        assert_eq!(detail.header_title(), "커밋상세");
        let text = ViewerTarget::Text {
            identity: "empty".into(),
            title: String::new(),
            lines: Vec::new(),
        };
        assert_eq!(text.header_title(), "Diff Viewer");
    }

    #[test]
    fn shortcut_initialization_and_saves_are_versioned_and_do_not_overwrite_newer_settings() {
        let mut state = super::KeySettings::default();
        assert!(state.begin_save().is_none());
        let mut first = super::ViewerKeys::default();
        first.overrides.insert("copy".into(), "alt-y".into());
        state.finish_load(0, Ok(first.clone()));
        assert!(!state.loading);
        assert_eq!(state.keys.binding("copy"), "alt-y");
        let revision = state.begin_save().unwrap();
        assert!(state.begin_save().is_none());
        state.finish_load(0, Ok(super::ViewerKeys::default()));
        assert_eq!(state.keys.binding("copy"), "alt-y");
        assert!(state.saving);
        assert!(!state.finish_save(
            revision,
            super::ViewerKeys::default(),
            Err(anyhow::anyhow!("save failure"))
        ));
        assert!(!state.saving && state.error.is_some());
        assert_eq!(state.keys.binding("copy"), "alt-y");
        let next = state.begin_save().unwrap();
        assert!(!state.finish_save(revision, super::ViewerKeys::default(), Ok(())));
        assert!(state.saving);
        assert!(state.finish_save(next, super::ViewerKeys::default(), Ok(())));
        assert!(state.error.is_none() && !state.saving);
        assert_eq!(state.keys.binding("copy"), "ctrl-c");
    }

    #[test]
    fn shortcut_load_failure_keeps_defaults_and_allows_later_save() {
        let mut state = super::KeySettings::default();
        state.finish_load(0, Err(anyhow::anyhow!("read failure")));
        assert!(!state.loading && state.error.is_some());
        assert_eq!(state.keys.binding("copy"), "ctrl-c");
        assert!(state.begin_save().is_some());
    }

    use super::{
        CommitDetail, Keystroke, Position, Selection, ViewerKeys, ViewerTarget, current_request,
        detail_label_len, detail_rows, displayed_byte, line_colors, original_column, read_keys,
    };
    #[test]
    fn default_shortcuts_shift_extension_and_custom_mapping_are_resolved() {
        let keys = ViewerKeys::default();
        for (key, command) in [
            ("ctrl-c", "copy"),
            ("ctrl-a", "all"),
            ("shift-home", "home"),
            ("shift-left", "left"),
            ("I", "up"),
            ("j", "left"),
            ("k", "down"),
            ("l", "right"),
            ("space", "toggle"),
        ] {
            assert_eq!(
                keys.command(&Keystroke::parse(key).unwrap()),
                Some(command),
                "{key}"
            );
        }
        let mut entries = super::key_entries(&keys);
        entries.iter_mut().find(|(id, _)| id == "copy").unwrap().1 = "alt-y".into();
        entries
            .iter_mut()
            .find(|(id, _)| id == "copy-secondary")
            .unwrap()
            .1 = "ctrl-alt-y".into();
        let remapped = ViewerKeys::validate(&entries).unwrap();
        assert_eq!(
            remapped.command(&Keystroke::parse("ctrl-alt-y").unwrap()),
            Some("copy")
        );
        assert_eq!(
            remapped.command(&Keystroke::parse("alt-y").unwrap()),
            Some("copy")
        );
        assert_eq!(remapped.command(&Keystroke::parse("ctrl-c").unwrap()), None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.json");
        super::write_keys(&path, &remapped).unwrap();
        super::write_keys(&path, &remapped).unwrap();
        assert_eq!(read_keys(&path).unwrap().binding("copy"), "alt-y");
        entries[0].1 = "alt-y".into();
        assert!(ViewerKeys::validate(&entries).is_err());
        entries[0].1 = "nonsense".into();
        assert!(ViewerKeys::validate(&entries).is_err());
        assert!(ViewerKeys::validate(&entries[..2]).is_err());
    }
    #[test]
    fn multiple_file_targets_keep_order_stage_and_independent_identity() {
        let file = super::FileChange {
            path: b"same.txt".to_vec(),
            label: "same.txt".into(),
            status: "M".into(),
            staged: true,
            before: crate::diff_model::Content::Empty,
            after: crate::diff_model::Content::Empty,
        };
        let mut unstaged = file.clone();
        unstaged.staged = false;
        let target = ViewerTarget::Files {
            root: "repository".into(),
            files: vec![file.clone(), unstaged.clone()],
        };
        let targets = target.file_targets().unwrap();
        assert_eq!(targets.len(), 2);
        assert!(!target.details());
        assert!(
            matches!(&targets[0], ViewerTarget::File { file: source, historical: false, .. } if source == &file)
        );
        assert!(
            matches!(&targets[1], ViewerTarget::File { file: source, historical: false, .. } if source == &unstaged)
        );
        assert!(!targets[0].same_file(&targets[1]));
        assert!(targets[0].same_file(&targets[0]));
        let other = ViewerTarget::File {
            root: "other".into(),
            file,
            historical: false,
        };
        assert!(!targets[0].same_file(&other));
        assert!(targets[0].file_targets().is_none());
    }

    #[test]
    fn whitespace_decorations_keep_original_unicode_tab_positions_and_copy() {
        let raw = "한 \t끝  ";
        assert_eq!(
            super::invisible_markers(raw).collect::<Vec<_>>(),
            [(3, 4, "·"), (4, 8, "→"), (11, 12, "·"), (12, 13, "·")]
        );
        let selection = Selection {
            anchor: Position { row: 0, column: 0 },
            cursor: Position {
                row: 0,
                column: raw.chars().count(),
            },
        };
        assert_eq!(selection.text(&[raw.into()]), raw);
        assert_eq!(displayed_byte(raw, 3), 8);
        assert_eq!(original_column(raw, 8), 3);
        assert_eq!(super::original_character_column(raw, 7), 2);
        assert_eq!(super::invisible_markers("").count(), 0);
        assert_eq!(super::invisible_markers("abc한글").count(), 0);
        assert_eq!(
            super::invisible_markers("\t\t ").collect::<Vec<_>>(),
            [(0, 4, "→"), (4, 8, "→"), (8, 9, "·")]
        );
    }

    #[test]
    fn text_menu_hit_allows_blank_rows_but_excludes_gutter_and_nonblank_trailing_space() {
        use gpui::{Bounds, point, px, size};
        let line = Bounds::new(point(px(10.), px(20.)), size(px(300.), px(20.)));
        let editor = Bounds::new(point(px(100.), px(0.)), size(px(200.), px(200.)));
        let origin = point(px(100.), px(20.));
        assert!(super::text_menu_hit(
            point(px(110.), px(25.)),
            line,
            editor,
            origin,
            px(50.),
            false
        ));
        assert!(!super::text_menu_hit(
            point(px(180.), px(25.)),
            line,
            editor,
            origin,
            px(50.),
            false
        ));
        assert!(super::text_menu_hit(
            point(px(180.), px(25.)),
            line,
            editor,
            origin,
            px(0.),
            true
        ));
        assert!(super::text_menu_hit(
            point(px(180.), px(25.)),
            line,
            editor,
            origin,
            px(20.),
            true
        ));
        for blank in [false, true] {
            assert!(!super::text_menu_hit(
                point(px(99.), px(25.)),
                line,
                editor,
                origin,
                px(50.),
                blank
            ));
            assert!(!super::text_menu_hit(
                point(px(110.), px(41.)),
                line,
                editor,
                origin,
                px(50.),
                blank
            ));
        }
        assert!(!super::text_menu_hit(
            point(px(110.), px(25.)),
            line,
            editor,
            origin,
            px(0.),
            false
        ));
    }

    #[test]
    fn check_menu_requires_changed_rows_in_the_current_range() {
        let mut data = super::text_diff(vec!["context".into(), "added".into()]);
        assert!(!super::checkable_rows(&data, 0..2));
        data.partial = Some(crate::partial_stage::Preview {
            before: String::new(),
            after: "added".into(),
        });
        assert!(!super::checkable_rows(&data, 0..2));
        data.lines[1].kind = '+';
        assert!(super::checkable_rows(&data, 1..2));
        assert!(!super::checkable_rows(&data, 0..1));
        assert!(!super::checkable_rows(&data, 2..99));
        assert!(!super::checkable_rows(&data, 0..0));
        data.lines[1].kind = '-';
        assert!(super::checkable_rows(&data, 0..2));
        let request = super::CheckMenuRequest {
            generation: 7,
            rows: 1..2,
        };
        assert_eq!(request.rows(7, &data, true), Some(1..2));
        assert_eq!(request.rows(8, &data, true), None);
        assert_eq!(request.rows(7, &data, false), None);
    }

    #[test]
    fn hunk_selection_toggles_only_changed_rows_and_preserves_other_hunks() {
        let lines: Vec<_> = ['@', '-', '+', ' ', '@', '-', '+', ' ', '@', ' ']
            .into_iter()
            .map(|kind| super::DiffLine {
                kind,
                text: "line".into(),
                old: None,
                new: None,
            })
            .collect();
        let mut checked = std::collections::BTreeSet::from([5]);
        let hunk = |header| crate::partial_stage::hunk_rows(&lines, header);
        super::toggle_checked_rows(&mut checked, &lines, hunk(0));
        assert_eq!(checked, [1, 2, 5].into());
        super::toggle_checked_rows(&mut checked, &lines, hunk(0));
        assert_eq!(checked, [5].into());
        checked.insert(1);
        super::toggle_checked_rows(&mut checked, &lines, hunk(0));
        assert_eq!(checked, [1, 2, 5].into());
        super::toggle_checked_rows(&mut checked, &lines, hunk(4));
        assert_eq!(checked, [1, 2, 5, 6].into());
        super::toggle_checked_rows(&mut checked, &lines, hunk(4));
        assert_eq!(checked, [1, 2].into());
        for header in [1, 8, 99] {
            super::toggle_checked_rows(&mut checked, &lines, hunk(header));
            assert_eq!(checked, [1, 2].into());
        }
        super::toggle_checked_rows(&mut checked, &lines, [0, 3, 7, 99]);
        assert_eq!(checked, [1, 2].into());
    }

    #[test]
    fn enter_is_removed_from_defaults_saved_settings_and_stage_label() {
        let keys = ViewerKeys::default();
        let enter = Keystroke::parse("enter").unwrap();
        assert_eq!(keys.command(&enter), None);
        assert_eq!(super::stage_button_label(&keys), "체크한 줄 스테이지");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.json");
        std::fs::write(
            &path,
            r#"{"overrides":{"stage":"enter","stage-secondary":"ENTER","copy":"alt-y"}}"#,
        )
        .unwrap();
        let saved = read_keys(&path).unwrap();
        assert_eq!(saved.command(&enter), None);
        assert_eq!(saved.binding("stage"), "");
        assert_eq!(saved.binding("stage-secondary"), "");
        assert_eq!(
            saved.command(&Keystroke::parse("alt-y").unwrap()),
            Some("copy")
        );
        assert_eq!(super::stage_button_label(&saved), "체크한 줄 스테이지");
        let mut entries = super::key_entries(&keys);
        entries.iter_mut().find(|(id, _)| id == "stage").unwrap().1 = "enter".into();
        assert!(ViewerKeys::validate(&entries).is_err());
        entries.iter_mut().find(|(id, _)| id == "stage").unwrap().1 = "ctrl-s".into();
        let custom = ViewerKeys::validate(&entries).unwrap();
        assert_eq!(
            custom.command(&Keystroke::parse("ctrl-s").unwrap()),
            Some("stage")
        );
        assert_eq!(
            super::stage_button_label(&custom),
            "체크한 줄 스테이지 (ctrl-s)"
        );
    }

    #[test]
    fn fixed_editor_viewport_and_reveal_keep_cursor_out_of_gutter() {
        let bounds = gpui::Bounds::new(
            gpui::point(gpui::px(30.), gpui::px(50.)),
            gpui::size(gpui::px(500.), gpui::px(200.)),
        );
        for gutter in [116., 140.] {
            let editor = super::editor_viewport(bounds, gutter);
            assert_eq!(editor.origin.x, gpui::px(30. + gutter));
            assert_eq!(editor.size.width, gpui::px(500. - gutter));
            assert_eq!(editor.origin.y, bounds.origin.y);
            assert_eq!(editor.size.height, bounds.size.height);
            // Home은 본문 시작을 노출하고 End는 오른쪽 경계 안으로 옮긴다.
            assert_eq!(
                super::cursor_horizontal_offset(gutter, -200., 500., gutter, 600.),
                0.
            );
            let end = super::cursor_horizontal_offset(gutter + 800., 0., 500., gutter, 600.);
            assert_eq!(gutter + 800. + end, 492.);
            // 가로 스크롤 후 본문 클릭 위치는 원문의 같은 문자로 복원된다.
            let offset = -80.;
            let column_x = 120.;
            let screen_x = editor.origin.x + gpui::px(column_x + offset);
            assert_eq!(
                f32::from(screen_x - bounds.origin.x) - gutter - offset,
                column_x
            );
        }
        let narrow = super::editor_viewport(bounds, 1000.);
        assert_eq!(narrow.size.width, gpui::px(0.));
        assert_eq!(
            super::cursor_horizontal_offset(140., -40., 60., 140., 90.),
            -40.
        );
        assert_eq!(super::cursor_horizontal_offset(12., -20., 500., 0., 0.), 0.);
        assert_eq!(super::editor_viewport(bounds, 0.), bounds);
    }

    #[test]
    fn old_request_is_rejected_on_switch_retry_or_deactivation() {
        let a = ViewerTarget::Text {
            identity: "project-1-file-1".into(),
            title: "a".into(),
            lines: vec![],
        };
        let b = ViewerTarget::Text {
            identity: "project-2-file-1".into(),
            title: "a".into(),
            lines: vec![],
        };
        assert!(current_request(5, Some(&a), 5, &a));
        assert!(!current_request(6, Some(&a), 5, &a));
        assert!(!current_request(5, Some(&b), 5, &a));
        assert!(!current_request(5, None, 5, &a));
    }
    #[test]
    fn double_click_hit_columns_keep_expanded_tabs_and_unicode_in_their_character() {
        let raw = "한\tfoo";
        for byte in 3..7 {
            assert_eq!(super::original_character_column(raw, byte), 1);
        }
        assert_eq!(super::original_character_column(raw, 0), 0);
        assert_eq!(super::original_character_column(raw, 7), 2);
        assert_eq!(super::original_character_column(raw, 10), 5);
        assert_eq!(super::original_character_column("", 100), 0);
        let mut selection = Selection::default();
        selection.select_word(
            raw,
            Position {
                row: 0,
                column: super::original_character_column(raw, 9),
            },
        );
        assert_eq!(selection.text(&[raw.into()]), "foo");
    }

    #[test]
    fn character_positions_preserve_unicode_tabs_and_reverse_selection() {
        let raw = "a한\t끝";
        assert_eq!(displayed_byte(raw, 3), 8);
        assert_eq!(original_column(raw, 8), 3);
        assert_eq!(original_column(raw, 5), 2);
        let selection = Selection {
            anchor: Position { row: 1, column: 1 },
            cursor: Position { row: 0, column: 1 },
        };
        assert_eq!(selection.text(&[raw.into(), "끝줄".into()]), "한\t끝\n끝");
        assert_ne!(line_colors('+'), line_colors('-'));
        assert_ne!(line_colors('@'), line_colors(' '));
    }
    #[test]
    fn detail_has_all_metadata_parents_and_full_message_with_bold_labels() {
        let detail = CommitDetail {
            id: "full-id".into(),
            parent_ids: vec!["p1".into(), "p2".into()],
            author: "A".into(),
            author_email: "a@a".into(),
            author_time: 0,
            author_offset: 32400,
            committer: "C".into(),
            committer_email: "c@c".into(),
            commit_time: 60,
            commit_offset: 0,
            message: "title\n\nbody\t한글\n".into(),
        };
        let rows = detail_rows(&detail);
        assert!(rows.contains(&"부모 커밋: p1".into()));
        assert!(rows.contains(&"부모 커밋: p2".into()));
        assert!(rows.contains(&"A <a@a>".into()));
        assert!(rows.contains(&"C <c@c>".into()));
        assert!(rows.contains(&"authored: 1970-01-01 09:00:00".into()));
        assert!(rows.contains(&"committed: 1970-01-01 00:01:00".into()));
        for parent_count in 0..=3 {
            let mut detail = detail.clone();
            detail.parent_ids = vec!["parent".into(); parent_count];
            let rows = detail_rows(&detail);
            let people = super::detail_identities(&detail, None);
            assert_eq!(rows[people[0].row], "A <a@a>");
            assert!(rows[people[0].row + 1].starts_with("authored: "));
            assert_eq!(rows[people[1].row], "C <c@c>");
            assert!(rows[people[1].row + 1].starts_with("committed: "));
            assert_ne!(people[0].avatar_url, people[1].avatar_url);
        }
        assert!(rows.contains(&"body\t한글".into()));
        assert!(detail_label_len(&rows[0]) > 0);
        assert_eq!(detail_label_len("title"), 0);
    }
    #[test]
    fn refresh_reads_latest_index_source_and_handles_disappeared_changes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        gix::init(root).unwrap();
        std::fs::write(root.join("new.txt"), "one\n").unwrap();
        let file = crate::diff_git::load_stage(root).unwrap().remove(0);
        let target = ViewerTarget::File {
            root: root.into(),
            file,
            historical: false,
        };
        crate::diff_git::set_index_content(root, b"new.txt", Some(b"one\n"), 0o100644, None)
            .unwrap();
        std::fs::write(root.join("new.txt"), "two\n").unwrap();
        let (data, source) = target.load().unwrap();
        let source = source.unwrap();
        assert!(
            data.lines
                .iter()
                .any(|line| line.kind == '-' && line.text == "one")
        );
        assert!(
            data.lines
                .iter()
                .any(|line| line.kind == '+' && line.text == "two")
        );
        let crate::diff_model::Content::Object(id, mode) = source.before else {
            panic!("현재 인덱스 원본이어야 합니다")
        };
        crate::diff_git::set_index_content(
            root,
            b"new.txt",
            None,
            0o100644,
            Some((id, u32::from(mode))),
        )
        .unwrap();
        std::fs::remove_file(root.join("new.txt")).unwrap();
        let (data, source) = target.load().unwrap();
        assert!(source.is_none());
        assert!(data.lines.is_empty());
        assert!(data.note.contains("더 이상"));
    }
}
