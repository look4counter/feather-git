use crate::settings_theme as theme;
use crate::{
    auth_settings::AuthPanel,
    settings_git::{GitContext, Identity, RepositoryInfo, SettingsSnapshot, display_url},
    text_input::TextInput as CommitEditor,
};
use gpui::{prelude::*, *};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Work {
    Loading,
    Saving,
}

pub enum SettingsEvent {
    Confirmed,
    #[cfg(windows)]
    CheckForUpdates,
}
impl EventEmitter<SettingsEvent> for SettingsPanel {}

pub struct SettingsPanel {
    context: GitContext,
    snapshot: Option<SettingsSnapshot>,
    name: Entity<CommitEditor>,
    email: Entity<CommitEditor>,
    auth: Entity<AuthPanel>,
    identity_focus: FocusHandle,
    #[cfg(windows)]
    update_focus: FocusHandle,
    remote_focus: FocusHandle,
    remote_delete_focus: BTreeMap<String, FocusHandle>,
    row_focus: BTreeMap<String, FocusHandle>,
    list_scroll: ScrollHandle,
    remote_name: Entity<CommitEditor>,
    remote_url: Entity<CommitEditor>,
    remote_dialog: bool,
    restore_remote_focus: bool,
    use_global: bool,
    local_draft: Identity,
    work: Option<Work>,
    error: Option<String>,
    error_revision: u64,
    notice: Option<String>,
    cancel: Arc<AtomicBool>,
}

impl Drop for SettingsPanel {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl SettingsPanel {
    fn error_occurrence(&mut self, message: String) -> Option<String> {
        self.error_revision += 1;
        Some(message)
    }

    pub fn new(repository: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let list_scroll = ScrollHandle::new();
        let auth = cx
            .new(|cx| AuthPanel::new(GitContext::new(repository.clone()), list_scroll.clone(), cx));
        cx.observe(&auth, |_, _, cx| cx.notify()).detach();
        let mut panel = Self {
            context: GitContext::new(repository),
            snapshot: None,
            name: cx.new(|cx| CommitEditor::single_line("", cx)),
            email: cx.new(|cx| CommitEditor::single_line("", cx)),
            auth,
            identity_focus: cx.focus_handle(),
            #[cfg(windows)]
            update_focus: cx.focus_handle(),
            remote_focus: cx.focus_handle(),
            remote_delete_focus: BTreeMap::new(),
            row_focus: BTreeMap::new(),
            list_scroll,
            remote_name: cx.new(|cx| CommitEditor::single_line("", cx)),
            remote_url: cx.new(|cx| CommitEditor::single_line("", cx)),
            remote_dialog: false,
            restore_remote_focus: false,
            use_global: true,
            local_draft: Identity::default(),
            work: None,
            error: None,
            error_revision: 0,
            notice: None,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        panel.load(cx);
        panel
    }

    pub fn can_close(&self, cx: &App) -> bool {
        self.work != Some(Work::Saving) && self.auth.read(cx).can_close()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_composing(&self, cx: &App) -> bool {
        if self.auth.read(cx).popup_open() {
            return self.auth.read(cx).is_composing(cx);
        }
        let editors = if self.remote_dialog {
            vec![&self.remote_name, &self.remote_url]
        } else {
            vec![&self.name, &self.email]
        };
        editors.iter().any(|editor| editor.read(cx).is_composing())
    }

    fn show_identity(&mut self, identity: &Identity, cx: &mut Context<Self>) {
        for (editor, value) in [(&self.name, &identity.name), (&self.email, &identity.email)] {
            editor.update(cx, |editor, cx| {
                editor.set_text(value, cx);
                editor.set_read_only(self.use_global, cx);
            });
        }
    }

    fn toggle_global(&mut self, cx: &mut Context<Self>) {
        if self.work.is_some()
            || self.snapshot.is_none()
            || self.context.repository.is_none()
            || self.is_composing(cx)
        {
            return;
        }
        if !self.use_global {
            self.local_draft = Identity {
                name: self.name.read(cx).message().into(),
                email: self.email.read(cx).message().into(),
            };
        }
        self.use_global = !self.use_global;
        let identity = if self.use_global {
            self.snapshot.as_ref().unwrap().global.clone()
        } else {
            self.local_draft.clone()
        };
        self.show_identity(&identity, cx);
        self.notice = None;
        cx.notify();
    }

    fn set_work(&mut self, work: Option<Work>, cx: &mut Context<Self>) {
        self.work = work;
        for editor in [&self.name, &self.email, &self.remote_name, &self.remote_url] {
            editor.update(cx, |editor, cx| {
                editor.enabled = work.is_none() && self.snapshot.is_some();
                cx.notify();
            });
        }
        cx.notify();
    }

    fn apply(&mut self, snapshot: SettingsSnapshot, cx: &mut Context<Self>) {
        self.use_global = snapshot.use_global;
        self.local_draft = snapshot.effective.clone();
        let identity = if self.use_global {
            &snapshot.global
        } else {
            &snapshot.effective
        };
        self.show_identity(identity, cx);
        self.snapshot = Some(snapshot);
        self.sync_remote_focus(cx);
    }

    fn sync_remote_focus(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let urls = snapshot
            .info
            .remotes
            .iter()
            .flat_map(|remote| remote.fetch_urls.iter().chain(remote.push_urls.iter()))
            .cloned()
            .collect();
        self.auth
            .update(cx, |auth, cx| auth.set_remote_urls(urls, cx));
        self.remote_delete_focus.retain(|name, _| {
            snapshot
                .info
                .remotes
                .iter()
                .any(|remote| &remote.name == name)
        });
        self.row_focus.retain(|name, _| {
            snapshot
                .info
                .remotes
                .iter()
                .any(|remote| &remote.name == name)
        });
        for remote in &snapshot.info.remotes {
            self.row_focus
                .entry(remote.name.clone())
                .or_insert_with(|| cx.focus_handle());
            self.remote_delete_focus
                .entry(remote.name.clone())
                .or_insert_with(|| cx.focus_handle());
        }
    }

    fn update_remote_info(&mut self, info: RepositoryInfo, cx: &mut Context<Self>) {
        if let Some(snapshot) = &mut self.snapshot {
            snapshot.info = info;
        }
        self.sync_remote_focus(cx);
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        if self.work.is_some() {
            return;
        }
        self.error = None;
        self.set_work(Some(Work::Loading), cx);
        let context = self.context.clone();
        let task = cx
            .background_executor()
            .spawn(async move { context.load() });
        cx.spawn(async move |this, cx| {
            let snapshot = task.await;
            this.update(cx, |this, cx| {
                match snapshot {
                    Ok(snapshot) => this.apply(snapshot, cx),
                    Err(error) => this.error = this.error_occurrence(error.to_string()),
                }
                this.set_work(None, cx);
            })
            .ok();
        })
        .detach();
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        if self.auth.read(cx).popup_open() {
            self.auth.update(cx, |auth, cx| auth.submit(cx));
            return;
        }
        if self.remote_dialog {
            self.save_remote(cx);
            return;
        }
        if self.context.repository.is_none() && self.work.is_none() && self.snapshot.is_some() {
            cx.emit(SettingsEvent::Confirmed);
            return;
        }
        if self.work.is_some()
            || self.snapshot.is_none()
            || self.context.repository.is_none()
            || self.is_composing(cx)
        {
            return;
        }
        let identity = Identity {
            name: self.name.read(cx).message().into(),
            email: self.email.read(cx).message().into(),
        };
        let context = self.context.clone();
        let global = self.use_global;
        self.error = None;
        self.notice = None;
        self.set_work(Some(Work::Saving), cx);
        let task = cx.background_executor().spawn(async move {
            if global {
                context.use_global_identity()?;
            } else {
                context.save_identity(&identity, false)?;
            }
            context.load()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                let confirmed = result.is_ok();
                match result {
                    Ok(snapshot) => {
                        this.apply(snapshot, cx);
                        this.notice = Some(
                            if global {
                                "현재 저장소에 전역 작성자 설정을 적용했습니다."
                            } else {
                                "현재 저장소에 저장했습니다."
                            }
                            .into(),
                        );
                    }
                    Err(error) => this.error = this.error_occurrence(error.to_string()),
                }
                this.set_work(None, cx);
                if confirmed {
                    cx.emit(SettingsEvent::Confirmed);
                }
            })
            .ok();
        })
        .detach();
    }

    fn identity_section(&self, cx: &Context<Self>) -> Div {
        let enabled = self.work.is_none() && self.snapshot.is_some();
        let available = enabled && self.context.repository.is_some();
        let checkbox = crate::form_controls::checkbox(
            "settings-global",
            self.use_global,
            "전역 설정 사용",
            available,
        )
        .track_focus(&self.identity_focus)
        .on_click(cx.listener(move |this, event, window, cx| {
            if !available || !crate::form_controls::checkbox_click(event) {
                return;
            }
            this.identity_focus.focus(window);
            this.toggle_global(cx);
        }));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(section_heading("커밋 작성자"))
            .child(crate::form_controls::labeled_input(
                "이름",
                self.name.clone(),
            ))
            .child(crate::form_controls::labeled_input(
                "이메일",
                self.email.clone(),
            ))
            .child(checkbox)
    }

    pub fn save_button(&self, cx: &Context<Self>) -> Stateful<Div> {
        let available = self.work.is_none() && self.snapshot.is_some();
        action(
            if self.remote_dialog {
                "settings-save-remote"
            } else {
                "settings-save-identity"
            },
            "확인",
            available,
        )
        .on_click(cx.listener(move |this, event, _, cx| {
            if !available || !crate::form_controls::checkbox_click(event) {
                return;
            }
            this.save(cx);
        }))
    }

    pub fn child_popup_open(&self, cx: &App) -> bool {
        self.remote_dialog || self.auth.read(cx).popup_open()
    }

    pub fn close_child_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.auth.read(cx).popup_open() {
            return self
                .auth
                .update(cx, |auth, cx| auth.close_popup(window, cx));
        }
        if !self.remote_dialog {
            return false;
        }
        if self.work.is_none() && !self.is_composing(cx) {
            self.remote_dialog = false;
            self.error = None;
            self.remote_focus.focus(window);
            cx.notify();
        }
        true
    }

    fn open_remote(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.work.is_some()
            || self.snapshot.is_none()
            || self.context.repository.is_none()
            || self.remote_dialog
        {
            return;
        }
        self.remote_name
            .update(cx, |editor, cx| editor.set_text("", cx));
        self.remote_url
            .update(cx, |editor, cx| editor.set_text("", cx));
        self.remote_dialog = true;
        self.error = None;
        self.notice = None;
        self.remote_name.read(cx).focus_handle.focus(window);
        cx.notify();
    }

    fn save_remote(&mut self, cx: &mut Context<Self>) {
        if self.work.is_some() || self.is_composing(cx) {
            return;
        }
        let name = self.remote_name.read(cx).message().to_owned();
        let url = self.remote_url.read(cx).message().to_owned();
        let context = self.context.clone();
        self.error = None;
        self.set_work(Some(Work::Saving), cx);
        let task = cx.background_executor().spawn(async move {
            context.add_remote(&name, &url)?;
            context.remote_info()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(info) => {
                        this.update_remote_info(info, cx);
                        this.remote_dialog = false;
                        this.notice = Some("원격 저장소를 추가했습니다.".into());
                        this.restore_remote_focus = true;
                    }
                    Err(error) => this.error = this.error_occurrence(error.to_string()),
                }
                this.set_work(None, cx);
            })
            .ok();
        })
        .detach();
    }

    fn remove_remote(&mut self, name: String, cx: &mut Context<Self>) {
        if self.work.is_some() || self.remote_dialog || self.is_composing(cx) {
            return;
        }
        self.error = None;
        self.notice = None;
        self.set_work(Some(Work::Saving), cx);
        let context = self.context.clone();
        let task = cx.background_executor().spawn(async move {
            context.remove_remote(&name)?;
            context.remote_info()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(info) => {
                        this.update_remote_info(info, cx);
                        this.notice = Some("원격 저장소 연결을 삭제했습니다.".into());
                        this.restore_remote_focus = true;
                    }
                    Err(error) => this.error = this.error_occurrence(error.to_string()),
                }
                this.set_work(None, cx);
            })
            .ok();
        })
        .detach();
    }

    pub fn child_popup(&mut self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        if self.auth.read(cx).popup_open() {
            return self.auth.update(cx, |auth, cx| auth.popup(window, cx));
        }
        let body = div()
            .flex()
            .flex_col()
            .gap_3()
            .pb_1()
            .child(
                crate::form_controls::labeled_input("원격 이름", self.remote_name.clone())
                    .flex_shrink_0(),
            )
            .child(
                crate::form_controls::labeled_input("URL", self.remote_url.clone()).flex_shrink_0(),
            )
            .when(self.work == Some(Work::Saving), |body| {
                body.child("저장하고 있습니다…")
            })
            .when_some(self.error.clone(), |body, error| {
                body.child(div().text_sm().text_color(rgb(theme::ERROR)).child(error))
            });
        crate::dialog::Dialog::new("settings-remote-overlay", "원격 저장소 추가")
            .width(px(540.))
            .body(body)
            .footer(div().flex().gap_2().child(self.save_button(cx)).child(
                action("remote-cancel", "취소", self.work.is_none()).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.close_child_popup(window, cx);
                    },
                )),
            ))
            .build()
    }

    #[cfg(windows)]
    fn update_section(&self, cx: &Context<Self>) -> Div {
        let enabled = self.can_close(cx) && !self.child_popup_open(cx) && !self.is_composing(cx);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_heading("업데이트"))
            .child(div().flex().child(
                action("settings-check-update", "업데이트 확인", enabled)
                    .track_focus(&self.update_focus)
                    .on_click(cx.listener(move |this, event, window, cx| {
                        if enabled && crate::form_controls::checkbox_click(event) {
                            this.update_focus.focus(window);
                            cx.emit(SettingsEvent::CheckForUpdates);
                        }
                    })),
            ))
    }

    fn remote_section(&self, window: &Window, cx: &Context<Self>) -> Div {
        let available =
            self.work.is_none() && self.snapshot.is_some() && self.context.repository.is_some();
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_heading("원격 저장소"));
        if let Some(snapshot) = &self.snapshot {
            for (index, remote) in snapshot.info.remotes.iter().enumerate() {
                let mut urls = remote.fetch_urls.clone();
                for url in &remote.push_urls {
                    if !urls.contains(url) {
                        urls.push(url.clone());
                    }
                }
                if urls.is_empty() {
                    urls.push("URL 없음".into());
                }
                let mut entries = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(crate::form_controls::list_title(remote.name.clone()));
                for url in &urls {
                    entries = entries.child(crate::form_controls::list_detail(display_url(url)));
                }
                let name = remote.name.clone();
                let focus = self.remote_delete_focus[&name].clone();
                let deletable = available && !self.remote_dialog;
                body = body.child(
                    crate::form_controls::navigable_list_row(
                        ("settings-remote", index),
                        index,
                        snapshot
                            .info
                            .remotes
                            .iter()
                            .map(|remote| self.row_focus[&remote.name].clone())
                            .collect(),
                        window,
                        available && !self.remote_dialog,
                        self.list_scroll.clone(),
                    )
                    .child(entries)
                    .child(
                        crate::form_controls::list_delete("settings-delete-remote", deletable)
                            .track_focus(&focus)
                            .on_click(cx.listener(move |this, event, window, cx| {
                                if deletable && crate::form_controls::checkbox_click(event) {
                                    focus.focus(window);
                                    this.remove_remote(name.clone(), cx);
                                }
                            })),
                    ),
                );
            }
            if snapshot.info.remotes.is_empty() {
                body = body.child("등록된 원격 저장소가 없습니다.");
            }
        }
        body.child(
            div().flex().child(
                action("settings-add-remote", "추가", available)
                    .track_focus(&self.remote_focus)
                    .when(self.remote_focus.is_focused(window), |button| {
                        button
                            .bg(rgb(theme::SELECT))
                            .border_1()
                            .border_color(rgb(theme::ACCENT))
                    })
                    .on_click(cx.listener(move |this, event, window, cx| {
                        if available && crate::form_controls::checkbox_click(event) {
                            this.open_remote(window, cx);
                        }
                    })),
            ),
        )
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.restore_remote_focus {
            self.restore_remote_focus = false;
            self.remote_focus.focus(window);
        }
        let body = div()
            .flex()
            .flex_col()
            .gap_5()
            .child(self.identity_section(cx))
            .child(self.remote_section(window, cx))
            .child(self.auth.clone());
        #[cfg(windows)]
        let body = body.child(self.update_section(cx));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .h(px(570.))
            .max_h(window.viewport_size().height - px(180.))
            .min_h_0()
            .child(
                div()
                    .id("settings-body")
                    .track_scroll(&self.list_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pr_2()
                    .child(body),
            )
            .when_some(self.work.filter(|_| !self.remote_dialog), |panel, work| {
                panel.child(match work {
                    Work::Loading => "설정을 읽고 있습니다…",
                    Work::Saving => "저장하고 있습니다…",
                })
            })
            .when_some(
                self.error.clone().filter(|_| !self.remote_dialog),
                |panel, error| {
                    panel.child(div().text_sm().text_color(rgb(theme::ERROR)).child(error))
                },
            )
            .when_some(self.notice.clone(), |panel, notice| {
                panel.child(div().text_sm().text_color(rgb(theme::ACCENT)).child(notice))
            })
            .when(self.snapshot.is_none() && self.work.is_none(), |panel| {
                panel.child(
                    action("settings-retry", "다시 읽기", true)
                        .on_click(cx.listener(|this, _, _, cx| this.load(cx))),
                )
            })
    }
}

fn section_heading(title: &str) -> Div {
    div()
        .text_sm()
        .text_color(rgb(theme::ACCENT))
        .font_weight(FontWeight::SEMIBOLD)
        .child(title.to_owned())
}

fn action(id: &'static str, label: &str, enabled: bool) -> Stateful<Div> {
    crate::dialog::button(
        id,
        label.to_owned(),
        matches!(id, "settings-save-remote" | "settings-save-identity"),
        enabled,
    )
}
