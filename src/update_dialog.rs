use crate::{dialog, theme::*, updater};
use gpui::{prelude::*, *};

pub struct Dismiss;
impl EventEmitter<Dismiss> for UpdateDialog {}

enum State {
    Checking,
    Current,
    Available(updater::Release),
    Downloading,
    Failed(String),
}

pub struct UpdateDialog {
    focus: FocusHandle,
    state: State,
}

impl UpdateDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            state: State::Checking,
        }
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window);
        self.check(window, cx);
    }

    fn check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state = State::Checking;
        let task = cx.background_executor().spawn(async { updater::check() });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.state = match result {
                    Ok(Some(release)) => State::Available(release),
                    Ok(None) => State::Current,
                    Err(error) => State::Failed(format!("{error:#}")),
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let State::Available(release) = &self.state else {
            return;
        };
        let release = release.clone();
        self.state = State::Downloading;
        let task = cx
            .background_executor()
            .spawn(async move { updater::prepare(release) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                match result.and_then(updater::PreparedUpdate::launch) {
                    Ok(()) => cx.quit(),
                    Err(error) => {
                        this.state = State::Failed(format!("{error:#}"));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn dismiss(&self, cx: &mut Context<Self>) {
        if !matches!(self.state, State::Downloading) {
            cx.emit(Dismiss);
        }
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let message = match &self.state {
            State::Checking => "최신 버전을 확인하는 중입니다…".into(),
            State::Current => "최신 버전을 사용 중입니다.".into(),
            State::Available(release) => {
                format!("새 버전 {}을 설치할 수 있습니다.", release.version)
            }
            State::Downloading => {
                "업데이트를 다운로드하는 중입니다… 완료되면 앱이 다시 시작됩니다.".into()
            }
            State::Failed(error) => error.clone(),
        };
        let footer = div()
            .flex()
            .gap_2()
            .child(
                dialog::button(
                    "update-close",
                    "닫기",
                    false,
                    !matches!(self.state, State::Downloading),
                )
                .on_click(cx.listener(|this, _, _, cx| this.dismiss(cx))),
            )
            .when(matches!(self.state, State::Available(_)), |el| {
                el.child(
                    dialog::button("update-apply", "업데이트 및 재시작", true, true)
                        .on_click(cx.listener(|this, _, window, cx| this.update(window, cx))),
                )
            })
            .when(matches!(self.state, State::Failed(_)), |el| {
                el.child(
                    dialog::button("update-retry", "다시 확인", true, true).on_click(cx.listener(
                        |this, _, window, cx| {
                            if matches!(this.state, State::Failed(_)) {
                                this.check(window, cx);
                            }
                        },
                    )),
                )
            });
        dialog::Dialog::new("update-dialog", "업데이트 확인")
            .track_focus(&self.focus)
            .body(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_size(px(13.))
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .child(format!("현재 버전 {}", updater::CURRENT_VERSION)),
                    )
                    .child(message),
            )
            .footer(footer)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.dismiss(cx);
                }
                cx.stop_propagation();
            }))
            .build()
    }
}
