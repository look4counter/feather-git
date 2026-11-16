use crate::{dialog::Dialog, path_display::display_path, session::RepositoryEntry, theme::*};
use gpui::{prelude::*, *};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

const HOLD_DURATION: Duration = Duration::from_millis(1500);

#[derive(Clone, Default)]
pub struct HoldToDelete(Rc<RefCell<HoldState>>);

#[derive(Default)]
struct HoldState {
    started: Option<Instant>,
    generation: u64,
}

impl HoldToDelete {
    fn begin(&self, now: Instant) -> u64 {
        self.reset();
        let mut state = self.0.borrow_mut();
        state.started = Some(now);
        state.generation
    }
    pub fn reset(&self) {
        let mut state = self.0.borrow_mut();
        state.started = None;
        state.generation = state.generation.wrapping_add(1);
    }
    fn progress(&self, now: Instant) -> Option<(u64, f32)> {
        let state = self.0.borrow();
        let elapsed = now.saturating_duration_since(state.started?);
        Some((
            state.generation,
            if elapsed >= HOLD_DURATION {
                1.
            } else {
                (elapsed.as_secs_f32() / HOLD_DURATION.as_secs_f32()).min(1. - f32::EPSILON)
            },
        ))
    }
    fn complete(&self, generation: u64, now: Instant) -> bool {
        let state = self.0.borrow();
        let ready = state.generation == generation
            && state
                .started
                .is_some_and(|start| now.saturating_duration_since(start) >= HOLD_DURATION);
        drop(state);
        if ready {
            self.reset();
        }
        ready
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RemovalChoice {
    EntryOnly,
    Files,
    Cancel,
}

fn is_cancel_key(key: &str) -> bool {
    key == "escape"
}

pub fn render(
    entry: &RepositoryEntry,
    focus: &FocusHandle,
    hold: &HoldToDelete,
    window: &mut Window,
    on_choice: impl Fn(RemovalChoice, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let callback = Rc::new(on_choice);
    let keyboard = callback.clone();
    if !window.is_window_active() || !focus.is_focused(window) {
        hold.reset();
    }
    let progress = hold.progress(Instant::now());
    if let Some((generation, fraction)) = progress {
        if fraction >= 1. {
            // 먼저 가득 찬 프레임을 그린 뒤 현재 누름만 한 번 소비합니다.
            let hold = hold.clone();
            let callback = callback.clone();
            let focus = focus.clone();
            window.on_next_frame(move |window, cx| {
                if !window.is_window_active() || !focus.is_focused(window) {
                    hold.reset();
                    return;
                }
                if hold.complete(generation, Instant::now()) {
                    callback(RemovalChoice::Files, window, cx);
                }
            });
        } else {
            window.request_animation_frame();
        }
    }
    let keyboard_hold = hold.clone();
    let buttons = div().flex().gap_2().children(
        [
            (
                "remove-recent-entry",
                "항목만 삭제",
                RemovalChoice::EntryOnly,
            ),
            (
                "remove-repository-files",
                "저장소 파일 삭제",
                RemovalChoice::Files,
            ),
            ("cancel-recent-removal", "취소", RemovalChoice::Cancel),
        ]
        .into_iter()
        .map(|(id, label, choice)| {
            let callback = callback.clone();
            crate::dialog::button(id, label, choice == RemovalChoice::Files, true)
                .relative()
                .overflow_hidden()
                .when(choice == RemovalChoice::Files, |button| {
                    let press = hold.clone();
                    let release = hold.clone();
                    let release_out = hold.clone();
                    let movement = hold.clone();
                    button
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .h_full()
                                .w(relative(progress.map_or(0., |(_, fraction)| fraction)))
                                .bg(rgba((ACCENT << 8) | 0x55)),
                        )
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            press.begin(Instant::now());
                            window.refresh();
                        })
                        .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            release.reset();
                            window.refresh();
                        })
                        .on_mouse_up_out(MouseButton::Left, move |_, window, _| {
                            release_out.reset();
                            window.refresh();
                        })
                        .child(
                            canvas(
                                |bounds, _, _| bounds,
                                move |_, bounds, window, _| {
                                    // hover는 누르는 중에도 해제되므로 실제 영역과 좌측 버튼 상태를 확인합니다.
                                    let movement = movement.clone();
                                    let exit = movement.clone();
                                    window.on_mouse_event(
                                        move |event: &MouseMoveEvent, phase, window, _| {
                                            if phase == DispatchPhase::Capture
                                                && (!bounds.contains(&event.position)
                                                    || !event.dragging())
                                                && movement.progress(Instant::now()).is_some()
                                            {
                                                movement.reset();
                                                window.refresh();
                                            }
                                        },
                                    );
                                    window.on_mouse_event(
                                        move |_: &MouseExitEvent, phase, window, _| {
                                            if phase == DispatchPhase::Capture
                                                && exit.progress(Instant::now()).is_some()
                                            {
                                                exit.reset();
                                                window.refresh();
                                            }
                                        },
                                    );
                                },
                            )
                            .absolute()
                            .size_full(),
                        )
                })
                .child(div().relative().child(label))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    if choice != RemovalChoice::Files {
                        callback(choice, window, cx);
                    }
                })
        }),
    );
    Dialog::new("recent-removal-overlay", "최근 저장소 삭제")
        .width(px(540.)).track_focus(focus)
        .on_key_down(move |event, window, cx| {
            cx.stop_propagation();
            if is_cancel_key(&event.keystroke.key) { keyboard_hold.reset(); keyboard(RemovalChoice::Cancel, window, cx); }
        })
        .body(div().flex().flex_col().gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(entry.name.clone()))
            .child(div().text_size(px(12.)).child(display_path(&entry.path)))
            .child(div().text_size(px(13.)).text_color(rgb(MUTED))
                .child("항목만 삭제하면 디스크의 파일은 유지됩니다. 저장소 파일 삭제를 선택하면 작업 파일과 .git을 포함한 폴더 전체가 영구 삭제되며 복구할 수 없습니다. 삭제하려면 저장소 파일 삭제 버튼을 1.5초 동안 누르세요.")))
        .footer(buttons).build()
}

#[cfg(test)]
mod tests {
    use super::{HOLD_DURATION, HoldToDelete};
    use std::time::{Duration, Instant};

    #[test]
    fn hold_requires_full_duration_and_completes_only_once() {
        let hold = HoldToDelete::default();
        let start = Instant::now();
        let generation = hold.begin(start);
        assert_eq!(hold.progress(start), Some((generation, 0.)));
        assert_eq!(
            hold.progress(start + HOLD_DURATION / 2),
            Some((generation, 0.5))
        );
        assert!(!hold.complete(generation, start + HOLD_DURATION - Duration::from_millis(1)));
        assert!(
            hold.progress(start + HOLD_DURATION - Duration::from_nanos(1))
                .unwrap()
                .1
                < 1.
        );
        assert_eq!(hold.progress(start + HOLD_DURATION), Some((generation, 1.)));
        assert!(hold.complete(generation, start + HOLD_DURATION));
        assert!(!hold.complete(generation, start + HOLD_DURATION));
        assert!(hold.progress(start + HOLD_DURATION).is_none());
    }

    #[test]
    fn cancelled_and_previous_holds_cannot_delete_after_restart() {
        let hold = HoldToDelete::default();
        let start = Instant::now();
        let previous = hold.begin(start);
        hold.reset();
        assert!(!hold.complete(previous, start + HOLD_DURATION));
        assert!(hold.progress(start + HOLD_DURATION).is_none());
        let next = hold.begin(start + HOLD_DURATION);
        assert!(!hold.complete(previous, start + HOLD_DURATION * 2));
        assert_eq!(hold.progress(start + HOLD_DURATION), Some((next, 0.)));
        assert!(!hold.complete(next, start + HOLD_DURATION));
        assert!(hold.complete(next, start + HOLD_DURATION * 2));
    }

    #[test]
    fn keyboard_only_cancels_and_never_confirms_permanent_delete() {
        assert!(super::is_cancel_key("escape"));
        for key in ["enter", "space", "delete", "backspace"] {
            assert!(!super::is_cancel_key(key));
        }
    }
}
