#[path = "../vendor/gpui/src/platform/windows/ime.rs"]
mod windows_ime;
use crate::text_input::apply_edit;
use gpui::SharedString;
use std::{cell::RefCell, ops::Range};
use windows_ime::{ImeAction, ImeEvent, ImeState, dispatch_character, dispatch_ime_key};

#[derive(Default)]
struct Input {
    text: SharedString,
    selection: Range<usize>,
    marked: Option<Range<usize>>,
    state: ImeState,
}
impl Input {
    fn event(&mut self, event: ImeEvent<'_>) {
        for action in self.state.actions(event) {
            match action {
                ImeAction::Unmark => self.marked = None,
                ImeAction::Replace(text) => apply_edit(
                    &mut self.text,
                    &mut self.selection,
                    &mut self.marked,
                    None,
                    text,
                    None,
                    false,
                ),
                ImeAction::Mark(text, selected) => apply_edit(
                    &mut self.text,
                    &mut self.selection,
                    &mut self.marked,
                    None,
                    text,
                    selected,
                    true,
                ),
            }
        }
        assert!(self.text.is_char_boundary(self.selection.start));
        assert!(self.text.is_char_boundary(self.selection.end));
    }
    fn composing(&mut self, text: &str) {
        self.event(ImeEvent::Composition {
            result: None,
            composing: Some((text, None)),
        });
    }
}

#[test]
fn native_composition_is_completed_before_latin_character() {
    for prefix in ["", "한글😀"] {
        for (korean, latin) in [("러", "l"), ("쿼리", "ID")] {
            let input = RefCell::new(Input::default());
            input.borrow_mut().event(ImeEvent::Character(prefix));
            input.borrow_mut().composing(korean);
            dispatch_character(
                true,
                || {
                    let mut input = input.borrow_mut();
                    input.event(ImeEvent::Composition {
                        result: Some(korean),
                        composing: None,
                    });
                    input.event(ImeEvent::End);
                },
                || input.borrow_mut().event(ImeEvent::Character(latin)),
            );
            let mut input = input.borrow_mut();
            assert_eq!(input.text.as_ref(), format!("{prefix}{korean}{latin}"));
            assert!(input.marked.is_none());
            let end = input.text.len();
            input.selection = end - 1..end;
            input.event(ImeEvent::Character(""));
            assert_eq!(
                input.text.as_ref(),
                format!("{prefix}{korean}{}", &latin[..latin.len() - 1])
            );
        }
    }
}

#[test]
fn result_and_end_orders_do_not_lose_or_duplicate_korean() {
    for end_first in [false, true] {
        let mut input = Input::default();
        input.composing("러");
        if end_first {
            input.event(ImeEvent::End);
        }
        input.event(ImeEvent::Composition {
            result: Some("러"),
            composing: None,
        });
        if !end_first {
            input.event(ImeEvent::End);
        }
        input.event(ImeEvent::Boundary);
        input.event(ImeEvent::Character("l"));
        assert_eq!(input.text.as_ref(), "러l");
    }
    let mut input = Input::default();
    input.composing("러");
    input.event(ImeEvent::End);
    input.event(ImeEvent::Character("l"));
    assert_eq!(input.text.as_ref(), "러l");
}

#[test]
fn previous_result_precedes_new_composition_and_cancel_keeps_committed_text() {
    let mut input = Input::default();
    input.composing("러");
    input.event(ImeEvent::Composition {
        result: Some("러"),
        composing: Some(("ㅎ", Some(1..1))),
    });
    assert_eq!(input.text.as_ref(), "러ㅎ");
    input.composing("한");
    assert_eq!(input.text.as_ref(), "러한");
    input.event(ImeEvent::Cancel);
    input.event(ImeEvent::End);
    input.event(ImeEvent::Character("l"));
    assert_eq!(input.text.as_ref(), "러l");
}

#[test]
fn ended_composition_boundary_keeps_selection_and_next_korean() {
    let mut input = Input::default();
    input.composing("한");
    input.event(ImeEvent::End);
    input.composing("글");
    assert_eq!(input.text.as_ref(), "한글");
    input.event(ImeEvent::End);
    input.event(ImeEvent::Boundary);
    input.selection = 0..input.text.len();
    input.composing("러");
    assert_eq!(input.text.as_ref(), "러");
}

#[test]
fn ime_mode_keys_translate_but_alt_and_ordinary_keys_do_not() {
    for key in [0x15, 0xe5, 0x1c] {
        let mut calls = 0;
        assert!(dispatch_ime_key(key, || calls += 1));
        assert_eq!(calls, 1);
    }
    for key in [0x12, 0xa5, 0x41, 0x25, 0xe7] {
        assert!(!windows_ime::is_ime_key(key));
        assert!(!dispatch_ime_key(key, || panic!(
            "ordinary keys keep app route"
        )));
    }
}

#[test]
fn dead_key_composition_is_not_committed_before_combined_character() {
    let input = RefCell::new(Input::default());
    input.borrow_mut().composing("´");
    dispatch_character(
        false,
        || panic!("no native composition"),
        || input.borrow_mut().event(ImeEvent::Character("é")),
    );
    assert_eq!(input.borrow().text.as_ref(), "é");
    assert!(input.borrow().marked.is_none());
}
