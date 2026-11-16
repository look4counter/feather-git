use crate::auth_profiles::{self as profiles, AuthMethod, AuthProfile, Secret};
use crate::{
    credentials::{self, Gcm, LoginTarget},
    settings_git::GitContext,
    settings_theme as theme,
    text_input::TextInput as CommitEditor,
};
use gpui::{prelude::*, *};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Busy {
    Loading,
    Waiting,
    Saving,
}

pub struct AuthPanel {
    profiles: Vec<AuthProfile>,
    all_profiles: Vec<AuthProfile>,
    remote_urls: Vec<String>,
    gcm: Option<Gcm>,
    add_focus: FocusHandle,
    delete_focus: BTreeMap<String, FocusHandle>,
    row_focus: BTreeMap<String, FocusHandle>,
    list_scroll: ScrollHandle,
    methods: [FocusHandle; 2],
    method: AuthMethod,
    url: Entity<CommitEditor>,
    username: Entity<CommitEditor>,
    password: Entity<CommitEditor>,
    popup: bool,
    busy: Option<Busy>,
    cancel: Arc<AtomicBool>,
    error: Option<String>,
    notice: Option<String>,
    restore_focus: bool,
}

impl Drop for AuthPanel {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl AuthPanel {
    pub fn new(context: GitContext, list_scroll: ScrollHandle, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            profiles: vec![],
            all_profiles: vec![],
            remote_urls: vec![],
            gcm: None,
            add_focus: cx.focus_handle(),
            delete_focus: BTreeMap::new(),
            row_focus: BTreeMap::new(),
            list_scroll,
            methods: std::array::from_fn(|_| cx.focus_handle()),
            method: AuthMethod::BrowserGcm,
            url: cx.new(|cx| CommitEditor::single_line("", cx)),
            username: cx.new(|cx| CommitEditor::single_line("", cx)),
            password: cx.new(CommitEditor::password),
            popup: false,
            busy: Some(Busy::Loading),
            cancel: Arc::new(AtomicBool::new(false)),
            error: None,
            notice: None,
            restore_focus: false,
        };
        let task = cx
            .background_executor()
            .spawn(async move { (profiles::load_with_existing(), Gcm::detect(&context)) });
        cx.spawn(async move |this, cx| {
            let (result, gcm) = task.await;
            this.update(cx, |this, cx| {
                this.gcm = gcm;
                match result {
                    Ok((entries, warning)) => {
                        this.update_entries(entries, cx);
                        this.error = warning;
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.set_busy(None, cx);
            })
            .ok();
        })
        .detach();
        panel.set_busy(Some(Busy::Loading), cx);
        panel
    }
    fn update_entries(&mut self, entries: Vec<AuthProfile>, cx: &mut Context<Self>) {
        self.all_profiles = entries;
        self.refilter(cx);
    }
    pub fn set_remote_urls(&mut self, remote_urls: Vec<String>, cx: &mut Context<Self>) {
        if self.remote_urls != remote_urls {
            self.remote_urls = remote_urls;
            self.refilter(cx);
            cx.notify();
        }
    }
    fn refilter(&mut self, cx: &mut Context<Self>) {
        let entries = profiles::manageable_for_remote_urls(&self.all_profiles, &self.remote_urls);
        self.delete_focus
            .retain(|id, _| entries.iter().any(|entry| &entry.id == id));
        self.row_focus
            .retain(|id, _| entries.iter().any(|entry| &entry.id == id));
        for entry in &entries {
            self.row_focus
                .entry(entry.id.clone())
                .or_insert_with(|| cx.focus_handle());
            self.delete_focus
                .entry(entry.id.clone())
                .or_insert_with(|| cx.focus_handle());
        }
        self.profiles = entries;
    }
    fn set_busy(&mut self, busy: Option<Busy>, cx: &mut Context<Self>) {
        self.busy = busy;
        for editor in [&self.url, &self.username, &self.password] {
            editor.update(cx, |editor, cx| {
                editor.enabled = busy.is_none();
                cx.notify();
            });
        }
        cx.notify();
    }
    pub fn popup_open(&self) -> bool {
        self.popup
    }
    pub fn can_close(&self) -> bool {
        self.busy != Some(Busy::Saving)
    }
    pub fn is_composing(&self, cx: &App) -> bool {
        self.popup
            && [&self.url, &self.username, &self.password]
                .iter()
                .any(|editor| editor.read(cx).is_composing())
    }

    fn change_method(&mut self, method: AuthMethod, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        self.method = method;
        self.error = None;
        self.password
            .update(cx, |editor, cx| editor.set_text("", cx));
        cx.notify();
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() || self.popup {
            return;
        }
        self.cancel = Arc::new(AtomicBool::new(false));
        self.popup = true;
        self.error = None;
        self.notice = None;
        self.password
            .update(cx, |editor, cx| editor.set_text("", cx));
        self.url.read(cx).focus_handle.focus(window);
        cx.notify();
    }
    pub fn close_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.popup {
            return false;
        }
        if self.busy == Some(Busy::Saving) || self.is_composing(cx) {
            return true;
        }
        self.cancel.store(true, Ordering::Relaxed);
        self.popup = false;
        self.error = None;
        self.password
            .update(cx, |editor, cx| editor.set_text("", cx));
        self.set_busy(None, cx);
        self.add_focus.focus(window);
        true
    }
    pub fn submit(&mut self, cx: &mut Context<Self>) {
        if !self.popup || self.busy.is_some() || self.is_composing(cx) {
            return;
        }
        self.error = None;
        let url = match profiles::check_new(self.url.read(cx).message().trim()) {
            Ok(url) => url,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let username = if self.method == AuthMethod::Password {
            self.username.read(cx).message().trim().to_owned()
        } else {
            String::new()
        };
        let method = self.method;
        if method == AuthMethod::Password {
            if username.is_empty() || self.password.read(cx).message().is_empty() {
                self.error = Some("ID와 비밀번호 또는 토큰을 입력하세요.".into());
                cx.notify();
                return;
            }
            let secret = self
                .password
                .update(cx, |editor, cx| editor.take_secret(cx));
            self.persist_profile(url, username, method, Some(secret), cx);
            return;
        }
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let Some(gcm) = self.gcm.clone() else {
            self.error = Some(
                "GCM이 설치되어 있지 않습니다. GCM을 설치하거나 ID와 비밀번호 방식을 선택하세요."
                    .into(),
            );
            cx.notify();
            return;
        };
        let target = match LoginTarget::parse(&url) {
            Ok(target) => target,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        self.set_busy(Some(Busy::Waiting), cx);
        let token_cancel = cancel.clone();
        let task = cx.background_executor().spawn(async move {
            credentials::authenticate(&gcm, &target, true, &token_cancel)
                .map(|account| (account, Secret::new(String::new())))
        });
        self.await_auth(task, url, method, cancel, cx);
    }
    fn await_auth(
        &mut self,
        task: Task<anyhow::Result<(String, Secret)>>,
        url: String,
        method: AuthMethod,
        cancel: Arc<AtomicBool>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                match result {
                    Ok((username, _)) if !method.uses_vault() && username == "연결된 계정" => {
                        this.error = Some("GCM에서 계정 ID를 확인하지 못했습니다. GCM 로그인 상태를 확인하고 다시 시도하세요.".into());
                        this.set_busy(None, cx);
                    }
                    Ok((username, secret)) => this.persist_profile(
                        url,
                        username,
                        method,
                        method.uses_vault().then_some(secret),
                        cx,
                    ),
                    Err(error) => {
                        this.error = Some(error.to_string());
                        this.set_busy(None, cx);
                    }
                }
            })
            .ok();
        })
        .detach();
    }
    fn persist_profile(
        &mut self,
        url: String,
        username: String,
        method: AuthMethod,
        secret: Option<Secret>,
        cx: &mut Context<Self>,
    ) {
        self.set_busy(Some(Busy::Saving), cx);
        let task = cx
            .background_executor()
            .spawn(async move { profiles::register(&url, &username, method, secret.as_ref()) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(entries) => {
                        this.update_entries(entries, cx);
                        this.popup = false;
                        this.notice = Some(
                            if method == AuthMethod::Password {
                                "인증 정보를 저장했습니다. 서버 로그인은 전송 시 확인합니다."
                            } else {
                                "인증을 등록했습니다."
                            }
                            .into(),
                        );
                        this.restore_focus = true;
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.set_busy(None, cx);
            })
            .ok();
        })
        .detach();
    }
    fn remove(&mut self, id: String, cx: &mut Context<Self>) {
        if self.busy.is_some() || self.popup {
            return;
        }
        self.error = None;
        self.notice = None;
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let gcm = self.gcm.clone();
        let removed_scope = self
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .map(|profile| (profile.url.clone(), profile.username.clone()));
        self.set_busy(Some(Busy::Saving), cx);
        let task = cx.background_executor().spawn(async move {
            let mut shared_logged_out = false;
            let entries = profiles::remove_after_logout(&id, |profile| {
                credentials::logout_profile(
                    gcm.as_ref(),
                    profile,
                    &profiles::SystemExistingCredentials,
                    &cancel,
                )?;
                shared_logged_out = true;
                Ok(())
            })?;
            if shared_logged_out {
                return Ok::<_, anyhow::Error>(profiles::load_with_existing().unwrap_or_else(
                    |_| {
                        (
                            entries,
                            Some(
                                "로그아웃은 완료했지만 기존 인증 목록을 다시 읽지 못했습니다."
                                    .into(),
                            ),
                        )
                    },
                ));
            }
            Ok((entries, None))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok((entries, warning)) => {
                        this.update_entries(entries, cx);
                        this.error = warning;
                        this.restore_focus = true;
                        let other_scope = removed_scope.as_ref().is_some_and(|(url, username)| {
                            this.profiles.iter().any(|profile| &profile.username == username && &profile.url != url)
                        });
                        this.notice = Some(if other_scope {
                            "선택한 주소의 인증을 로그아웃했습니다. 같은 계정명의 다른 주소 인증은 목록에 남아 있습니다."
                        } else { "로그아웃하고 인증 등록을 삭제했습니다." }.into());
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.set_busy(None, cx);
            })
            .ok();
        })
        .detach();
    }
    pub fn popup(&self, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let enabled = self.busy.is_none();
        let mut choices = div().flex().flex_wrap().gap_2();
        for (index, method) in [AuthMethod::BrowserGcm, AuthMethod::Password]
            .into_iter()
            .enumerate()
        {
            let selected = self.method == method;
            choices = choices.child(
                button(
                    ["auth-browser", "auth-password"][index],
                    ["브라우저 로그인", "ID와 비밀번호"][index],
                    enabled,
                )
                .track_focus(&self.methods[index])
                .when(selected, |button| {
                    button
                        .bg(rgb(theme::SELECT))
                        .border_color(rgb(theme::ACCENT))
                })
                .when(self.methods[index].is_focused(window), |button| {
                    button.border_1().border_color(rgb(theme::ACCENT))
                })
                .on_click(cx.listener(move |this, event, window, cx| {
                    if enabled && crate::form_controls::checkbox_click(event) {
                        this.methods[index].focus(window);
                        this.change_method(method, cx);
                    }
                })),
            );
        }
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .pb_1()
            .child(div().child("인증 방식"))
            .child(choices);
        body = body.child(crate::form_controls::labeled_input(
            "HTTP/HTTPS 저장소 또는 서비스 URL",
            self.url.clone(),
        ));
        if self.method == AuthMethod::Password {
            body = body.child(crate::form_controls::labeled_input(
                "ID",
                self.username.clone(),
            ));
        }
        if self.method == AuthMethod::Password {
            body = body.child(crate::form_controls::labeled_input(
                "비밀번호 또는 토큰",
                self.password.clone(),
            ));
        }

        body = body
            .when_some(self.error.clone(), |body, error| {
                body.child(div().text_color(rgb(theme::ERROR)).child(error))
            })
            .when_some(self.busy, |body, busy| {
                body.child(if busy == Busy::Saving {
                    "인증 정보를 저장하고 있습니다…"
                } else {
                    "인증을 기다리고 있습니다…"
                })
            });
        let footer = button("auth-submit", "", enabled)
            .child("확인")
            .on_click(cx.listener(move |this, event, _, cx| {
                if enabled && crate::form_controls::checkbox_click(event) {
                    this.submit(cx);
                }
            }));
        crate::dialog::Dialog::new("settings-auth-overlay", "인증 추가")
            .width(px(600.))
            .body(body)
            .footer(div().flex().gap_2().child(footer).child(
                button("auth-cancel", "취소", self.can_close()).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.close_popup(window, cx);
                    },
                )),
            ))
            .build()
    }
}
impl Render for AuthPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.restore_focus {
            self.restore_focus = false;
            self.add_focus.focus(window);
        }
        let enabled = self.busy.is_none() && !self.popup;
        let mut body = div().flex().flex_col().gap_2().flex_shrink_0().child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(theme::ACCENT))
                .child("인증"),
        );
        for (index, entry) in self.profiles.iter().enumerate() {
            let id = entry.id.clone();
            let focus = self.delete_focus[&id].clone();
            body = body.child(
                crate::form_controls::navigable_list_row(
                    ("auth-profile", index),
                    index,
                    self.profiles
                        .iter()
                        .map(|entry| self.row_focus[&entry.id].clone())
                        .collect(),
                    window,
                    enabled,
                    self.list_scroll.clone(),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(crate::form_controls::list_title(format!(
                            "{} · {}{}",
                            entry.username,
                            entry.method.label(),
                            if entry.imported {
                                " · 기존 인증"
                            } else {
                                ""
                            }
                        )))
                        .child(crate::form_controls::list_detail(
                            crate::settings_git::display_url(&entry.url),
                        )),
                )
                .child(
                    crate::form_controls::list_delete("auth-delete", enabled)
                        .track_focus(&focus)
                        .on_click(cx.listener(move |this, event, window, cx| {
                            if enabled && crate::form_controls::checkbox_click(event) {
                                focus.focus(window);
                                this.remove(id.clone(), cx);
                            }
                        })),
                ),
            );
        }
        if self.busy == Some(Busy::Loading) {
            body = body.child("인증 목록을 읽고 있습니다…");
        } else if self.profiles.is_empty() {
            body = body.child(if self.remote_urls.is_empty() {
                "원격 저장소를 등록하면 해당 주소에 해당하는 인증을 표시합니다."
            } else {
                "현재 원격 주소 또는 상위 주소에 해당하는 인증이 없습니다."
            });
        }
        if self
            .profiles
            .iter()
            .any(|profile| !profile.method.uses_vault())
        {
            body = body.child(div().text_xs().text_color(rgb(theme::MUTED)).child("GCM 인증 삭제 시 공유된 저장 인증도 제거되어 다른 Git 도구에서도 로그아웃될 수 있습니다."));
        }
        body.child(
            div().flex().child(
                button("auth-add", "추가", enabled)
                    .track_focus(&self.add_focus)
                    .when(self.add_focus.is_focused(window), |button| {
                        button
                            .bg(rgb(theme::SELECT))
                            .border_1()
                            .border_color(rgb(theme::ACCENT))
                    })
                    .on_click(cx.listener(move |this, event, window, cx| {
                        if enabled && crate::form_controls::checkbox_click(event) {
                            this.open(window, cx);
                        }
                    })),
            ),
        )
        .when(!self.popup, |body| {
            body.when_some(self.error.clone(), |body, error| {
                body.child(div().text_color(rgb(theme::ERROR)).child(error))
            })
            .when_some(self.notice.clone(), |body, notice| {
                body.child(div().text_color(rgb(theme::ACCENT)).child(notice))
            })
        })
    }
}
fn button(id: &'static str, label: &str, enabled: bool) -> Stateful<Div> {
    crate::dialog::button(id, label.to_owned(), id == "auth-submit", enabled)
}
