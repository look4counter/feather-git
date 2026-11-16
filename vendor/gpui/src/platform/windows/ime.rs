use std::ops::Range;

pub(crate) fn is_ime_key(virtual_key: u16) -> bool {
    // VK_KANA/VK_HANGUL, IME_ON, KANJI/HANJA, IME_OFF,
    // CONVERT, NONCONVERT, ACCEPT, MODECHANGE, and PROCESSKEY.
    matches!(virtual_key, 0x15 | 0x16 | 0x19 | 0x1a | 0x1c..=0x1f | 0xe5)
}

pub(crate) fn dispatch_ime_key(virtual_key: u16, translate: impl FnOnce()) -> bool {
    if is_ime_key(virtual_key) {
        translate();
        true
    } else {
        false
    }
}

pub(crate) fn dispatch_character(
    has_native_composition: bool,
    complete: impl FnOnce(),
    insert: impl FnOnce(),
) {
    if has_native_composition {
        complete();
    }
    insert();
}

#[derive(Default)]
pub(crate) struct ImeState {
    ended: bool,
}

pub(crate) enum ImeEvent<'a> {
    End,
    Boundary,
    Character(&'a str),
    Cancel,
    Composition {
        result: Option<&'a str>,
        composing: Option<(&'a str, Option<Range<usize>>)>,
    },
}

#[derive(Debug, PartialEq)]
pub(crate) enum ImeAction<'a> {
    Unmark,
    Replace(&'a str),
    Mark(&'a str, Option<Range<usize>>),
}

impl ImeState {
    pub(crate) fn actions<'a>(&mut self, event: ImeEvent<'a>) -> Vec<ImeAction<'a>> {
        let mut actions = Vec::new();
        match event {
            // Korean IMEs can send the result after END. Keep the marked range
            // until that result or the next independent input, avoiding duplicates.
            ImeEvent::End => self.ended = true,
            ImeEvent::Boundary => self.finish(&mut actions),
            ImeEvent::Character(text) => {
                self.finish(&mut actions);
                actions.push(ImeAction::Replace(text));
            }
            ImeEvent::Cancel => {
                self.ended = false;
                actions.push(ImeAction::Replace(""));
            }
            ImeEvent::Composition { result, composing } => {
                if let Some(text) = result {
                    self.ended = false;
                    actions.push(ImeAction::Replace(text));
                }
                if let Some((text, selection)) = composing {
                    self.finish(&mut actions);
                    actions.push(ImeAction::Mark(text, selection));
                }
            }
        }
        actions
    }

    fn finish(&mut self, actions: &mut Vec<ImeAction<'_>>) {
        if std::mem::take(&mut self.ended) {
            actions.push(ImeAction::Unmark);
        }
    }
}
