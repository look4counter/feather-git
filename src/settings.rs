use crate::{
    commit_editor::CommitEditor,
    credentials::{self, Gcm, LoginTarget},
    session,
    settings_git::{GitContext, Identity, SettingsSnapshot, display_url},
};
use gpui::{prelude::*, *};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    User,
    Remote,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Work {
    Loading,
    Saving,
    Auth,
}

#[derive(Clone, Copy)]
enum AuthAction {
    Status,
    Login,
    Logout,
    Configure,
}

pub struct SettingsPanel {
    context: GitContext,
    tab: SettingsTab,
    snapshot: Option<SettingsSnapshot>,
    name: Entity<CommitEditor>,
    email: Entity<CommitEditor>,
    login_url: Entity<CommitEditor>,
    use_global: bool,
    gcm: Option<Gcm>,
    helper_configured: bool,
    account: Option<(String, String)>,
    work: Option<Work>,
    error: Option<String>,
    notice: Option<String>,
    cancel: Arc<AtomicBool>,
}

impl Drop for SettingsPanel {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl SettingsPanel {
    pub fn new(repository: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            context: GitContext::new(repository),
            tab: SettingsTab::User,
            snapshot: None,
            name: cx.new(|cx| CommitEditor::single_line("", cx)),
            email: cx.new(|cx| CommitEditor::single_line("", cx)),
            login_url: cx.new(|cx| CommitEditor::single_line("", cx)),
            use_global: true,
            gcm: None,
            helper_configured: false,
            account: None,
            work: None,
            error: None,
            notice: None,
            cancel: Arc::new(AtomicBool::new(false)),
        };
        panel.load(cx);
        panel
    }

    pub fn can_close(&self) -> bool {
        self.work != Some(Work::Saving)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_composing(&self, cx: &App) -> bool {
        [&self.name, &self.email, &self.login_url]
            .iter()
            .any(|editor| editor.read(cx).is_composing())
    }

    pub fn focus_next(&self, backwards: bool, window: &mut Window, cx: &App) {
        if self.tab != SettingsTab::User || self.work.is_some() {
            return;
        }
        let inputs = [&self.name, &self.email, &self.login_url];
        let index = inputs
            .iter()
            .position(|input| input.read(cx).focus.is_focused(window));
        let next = match index {
            Some(index) if backwards => (index + 2) % 3,
            Some(index) => (index + 1) % 3,
            None => 0,
        };
        inputs[next].read(cx).focus.focus(window);
    }

    fn set_work(&mut self, work: Option<Work>, cx: &mut Context<Self>) {
        self.work = work;
        for editor in [&self.name, &self.email, &self.login_url] {
            editor.update(cx, |editor, cx| {
                editor.enabled = work.is_none() && self.snapshot.is_some();
                cx.notify();
            });
        }
        cx.notify();
    }

    fn apply(&mut self, snapshot: SettingsSnapshot, cx: &mut Context<Self>) {
        self.use_global = snapshot.use_global;
        self.name.update(cx, |editor, cx| {
            editor.set_text(&snapshot.effective.name, cx)
        });
        self.email.update(cx, |editor, cx| {
            editor.set_text(&snapshot.effective.email, cx)
        });
        if self.login_url.read(cx).message().is_empty() {
            let target = snapshot
                .info
                .remotes
                .iter()
                .flat_map(|remote| &remote.fetch_urls)
                .map(|url| display_url(url))
                .find(|url| LoginTarget::parse(url).is_ok())
                .unwrap_or_default();
            self.login_url
                .update(cx, |editor, cx| editor.set_text(&target, cx));
        }
        self.snapshot = Some(snapshot);
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        if self.work.is_some() {
            return;
        }
        self.error = None;
        self.set_work(Some(Work::Loading), cx);
        let context = self.context.clone();
        let task = cx.background_executor().spawn(async move {
            let snapshot = context.load();
            let gcm = Gcm::detect(&context);
            (snapshot, gcm, context.gcm_configured())
        });
        cx.spawn(async move |this, cx| {
            let (snapshot, gcm, configured) = task.await;
            this.update(cx, |this, cx| {
                this.gcm = gcm;
                this.helper_configured = configured;
                match snapshot {
                    Ok(snapshot) => this.apply(snapshot, cx),
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.set_work(None, cx);
            })
            .ok();
        })
        .detach();
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        if self.work.is_some()
            || self.snapshot.is_none()
            || self.tab != SettingsTab::User
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
            context.save_identity(&identity, global)?;
            context.load()
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(snapshot) => {
                        this.apply(snapshot, cx);
                        this.notice = Some("사용자 설정을 저장했습니다.".into());
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                this.set_work(None, cx);
            })
            .ok();
        })
        .detach();
    }

    fn auth(&mut self, action: AuthAction, cx: &mut Context<Self>) {
        if self.work.is_some() {
            return;
        }
        let Some(gcm) = self.gcm.clone() else {
            self.error = Some("Git Credential Manager를 설치한 뒤 설정을 다시 열어 주세요.".into());
            cx.notify();
            return;
        };
        let url = self.login_url.read(cx).message().trim().to_owned();
        let target = if matches!(action, AuthAction::Configure) {
            None
        } else {
            match LoginTarget::parse(&url) {
                Ok(target) => Some(target),
                Err(error) => {
                    self.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            }
        };
        self.error = None;
        self.notice = Some(
            if matches!(action, AuthAction::Login) {
                "브라우저에서 로그인을 완료하세요. 기존 인증이 있으면 재사용합니다."
            } else {
                "인증 정보를 처리하고 있습니다…"
            }
            .into(),
        );
        self.set_work(
            Some(if matches!(action, AuthAction::Configure) {
                Work::Saving
            } else {
                Work::Auth
            }),
            cx,
        );
        let cancel = self.cancel.clone();
        let context = self.context.clone();
        let task = cx.background_executor().spawn(async move {
            let account = match action {
                AuthAction::Configure => {
                    gcm.configure(&cancel)?;
                    None
                }
                AuthAction::Logout => {
                    credentials::logout(&gcm, target.as_ref().unwrap(), &cancel)?;
                    None
                }
                AuthAction::Status | AuthAction::Login => Some(credentials::authenticate(
                    &gcm,
                    target.as_ref().unwrap(),
                    matches!(action, AuthAction::Login),
                    &cancel,
                )?),
            };
            Ok::<_, anyhow::Error>((account, context.gcm_configured()))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.cancel.load(Ordering::Relaxed) { return; }
                match result {
                    Ok((account, configured)) => {
                        this.helper_configured = configured;
                        if !matches!(action, AuthAction::Configure) { this.account = account.map(|account| (url, account)); }
                        this.notice = Some(match action {
                            AuthAction::Configure => "Git에 GCM을 연결했습니다.",
                            AuthAction::Logout => "저장된 로그인을 삭제했습니다. 브라우저 로그인 세션은 유지됩니다.",
                            _ => "GCM 로그인 정보를 확인했습니다.",
                        }.into());
                    }
                    Err(error) => { this.account = None; this.notice = None; this.error = Some(error.to_string()); }
                }
                this.set_work(None, cx);
            }).ok();
        }).detach();
    }

    fn user_tab(&self, cx: &Context<Self>) -> Div {
        let mut body = div().flex().flex_col().gap_3();
        body = body
            .child(field("이름", self.name.clone()))
            .child(field("이메일", self.email.clone()))
            .child(
                action(
                    "settings-global",
                    if self.use_global {
                        "☑ 전역 설정 사용"
                    } else {
                        "☐ 현재 저장소 설정 사용"
                    },
                    self.work.is_none() && self.context.repository.is_some(),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if this.work.is_some() || this.context.repository.is_none() {
                        return;
                    }
                    this.use_global = !this.use_global;
                    if let Some(snapshot) = &this.snapshot {
                        let identity = if this.use_global {
                            &snapshot.global
                        } else {
                            &snapshot.effective
                        };
                        this.name
                            .update(cx, |editor, cx| editor.set_text(&identity.name, cx));
                        this.email
                            .update(cx, |editor, cx| editor.set_text(&identity.email, cx));
                    }
                    cx.notify();
                })),
            )
            .child(div().text_xs().text_color(rgb(0x8290a6)).child(
                if self.context.repository.is_none() {
                    "새 탭에서는 전역 설정만 편집할 수 있습니다."
                } else if self.use_global {
                    "저장하면 현재 저장소의 개별 이름·이메일을 제거하고 전역 값을 사용합니다."
                } else {
                    "현재 저장소의 이름·이메일만 저장합니다."
                },
            ))
            .child(
                action(
                    "settings-save",
                    "사용자 설정 저장 · ctrl + s",
                    self.work.is_none() && self.snapshot.is_some(),
                )
                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
            )
            .child(div().mt_3().text_color(rgb(0x5edac1)).child("원격 로그인"));
        if let Some(gcm) = &self.gcm {
            body = body.child(div().text_xs().child(format!(
                "GCM {} · {}",
                gcm.version,
                if self.helper_configured {
                    "Git에 연결됨"
                } else {
                    "Git 연결 필요"
                }
            )));
        } else {
            body = body.child(
                div()
                    .text_xs()
                    .child("Git Credential Manager가 없습니다. 설치 후 설정을 다시 열어 주세요."),
            );
        }
        body = body.child(field(
            "로그인할 HTTPS 저장소 또는 서비스 URL",
            self.login_url.clone(),
        ));
        let url = self.login_url.read(cx).message().trim();
        body = body.child(
            div().text_xs().child(
                self.account
                    .as_ref()
                    .filter(|(target, _)| target == url)
                    .map(|(_, account)| format!("연결된 계정: {account}"))
                    .unwrap_or_else(|| {
                        "이 주소의 로그인 상태를 확인하세요. SSH는 SSH 키로 인증합니다.".into()
                    }),
            ),
        );
        let enabled = self.work.is_none() && self.gcm.is_some();
        body = body.child(div().flex().flex_wrap().gap_2()
            .child(action("settings-auth-status", "상태 확인", enabled).on_click(cx.listener(|this, _, _, cx| this.auth(AuthAction::Status, cx))))
            .child(action("settings-auth-login", "브라우저 로그인", enabled).on_click(cx.listener(|this, _, _, cx| this.auth(AuthAction::Login, cx))))
            .child(action("settings-auth-logout", "로그아웃", enabled).on_click(cx.listener(|this, _, _, cx| this.auth(AuthAction::Logout, cx))))
            .child(action("settings-auth-configure", "Git에 GCM 연결", enabled).on_click(cx.listener(|this, _, _, cx| this.auth(AuthAction::Configure, cx)))))
            .child(div().text_xs().text_color(rgb(0x8290a6)).child("인증 정보는 GCM이 관리합니다. 브라우저 로그인 지원은 서비스와 GCM 버전에 따릅니다."));
        body
    }

    fn remote_tab(&self) -> Div {
        let mut body = div().flex().flex_col().gap_3();
        let Some(path) = &self.context.repository else {
            return body.child("저장소를 열면 원격 정보를 볼 수 있습니다.");
        };
        body = body.child(div().text_sm().child(session::display_path(path)));
        let Some(snapshot) = &self.snapshot else {
            return body;
        };
        body = body
            .child(format!(
                "현재 브랜치: {}",
                snapshot.info.branch.as_deref().unwrap_or("Detached HEAD")
            ))
            .child(format!(
                "추적 브랜치: {}",
                snapshot.info.upstream.as_deref().unwrap_or("없음")
            ));
        if snapshot.info.remotes.is_empty() {
            body = body.child("등록된 원격 저장소가 없습니다.");
        }
        for remote in &snapshot.info.remotes {
            let mut entry = div()
                .p_3()
                .border_1()
                .border_color(rgb(0x293343))
                .rounded_md()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_color(rgb(0x5edac1)).child(remote.name.clone()));
            for url in &remote.fetch_urls {
                entry = entry.child(format!("가져오기: {}", display_url(url)));
            }
            for url in &remote.push_urls {
                entry = entry.child(format!("푸시: {}", display_url(url)));
            }
            body = body.child(entry);
        }
        body.child(
            div()
                .text_xs()
                .text_color(rgb(0x8290a6))
                .child("원격 정보는 읽기 전용입니다."),
        )
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.tab == SettingsTab::User {
            self.user_tab(cx)
        } else {
            self.remote_tab()
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .h(px(570.))
            .max_h(window.viewport_size().height - px(180.))
            .min_h_0()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        action("settings-user-tab", "사용자", true)
                            .when(self.tab == SettingsTab::User, |tab| {
                                tab.bg(rgb(0x203d42)).text_color(rgb(0x5edac1))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.tab = SettingsTab::User;
                                cx.notify();
                            })),
                    )
                    .child(
                        action("settings-remote-tab", "원격", true)
                            .when(self.tab == SettingsTab::Remote, |tab| {
                                tab.bg(rgb(0x203d42)).text_color(rgb(0x5edac1))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.tab = SettingsTab::Remote;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("settings-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pr_2()
                    .child(body),
            )
            .when_some(self.work, |panel, work| {
                panel.child(match work {
                    Work::Loading => "설정을 읽고 있습니다…",
                    Work::Saving => "저장하고 있습니다…",
                    Work::Auth => "인증을 처리하고 있습니다… (닫으면 취소)",
                })
            })
            .when_some(self.error.clone(), |panel, error| {
                panel.child(div().text_sm().text_color(rgb(0xf4a0a8)).child(error))
            })
            .when_some(self.notice.clone(), |panel, notice| {
                panel.child(div().text_sm().text_color(rgb(0x5edac1)).child(notice))
            })
            .when(self.snapshot.is_none() && self.work.is_none(), |panel| {
                panel.child(
                    action("settings-retry", "다시 읽기", true)
                        .on_click(cx.listener(|this, _, _, cx| this.load(cx))),
                )
            })
    }
}

fn action(id: &'static str, label: &str, enabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .text_sm()
        .bg(rgb(0x202b39))
        .text_color(rgb(0xd9e2ed))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|button| button.bg(rgb(0x2c4052)))
        })
        .when(!enabled, |button| button.opacity(0.45))
        .child(label.to_owned())
}

fn field(label: &str, editor: Entity<CommitEditor>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_sm().child(label.to_owned()))
        .child(
            div()
                .h(px(36.))
                .flex_shrink_0()
                .border_1()
                .border_color(rgb(0x293343))
                .rounded_sm()
                .p_1()
                .child(editor),
        )
}
