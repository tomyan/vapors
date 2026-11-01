//! The system IME's composition over the focused text field (E6c), as
//! the edits the web's text input sends (`Event::TextEdited`): composing
//! text is in the field's value while it composes, as Chromium's own
//! `<input>` holds it (doc 03), replacing the selection when it starts
//! and its own range after; a commit replaces the composing range and
//! ends it. Kept against the field the app last set, which the app's
//! next set confirms.

use winit::event::Ime;

use crate::{Event, TextField};

/// The field the app last set, and the range an IME is composing in it.
#[derive(Default)]
pub(crate) struct Composition {
    field: Option<TextField>,
    composing: Option<(u32, u32)>,
}

impl Composition {
    /// The app set the focused field, or none: a composition ends with
    /// the field's focus, or when the field holds another text than the
    /// composition made (the app refused an edit, or changed the text).
    pub(crate) fn set_field(&mut self, field: Option<TextField>) {
        let same = matches!((&self.field, &field), (Some(was), Some(now)) if was.text == now.text);
        if !same {
            self.composing = None;
        }
        self.field = field;
    }

    /// The edit `ime` makes to the field, if it changes anything.
    pub(crate) fn edit(&mut self, ime: &Ime) -> Option<Event> {
        match ime {
            Ime::Preedit(text, cursor) => self.replace(text, cursor.map(|(_, end)| end), true),
            Ime::Commit(text) => self.replace(text, None, false),
            Ime::Disabled => self.replace("", None, false),
            Ime::Enabled => None,
        }
    }

    /// Replaces the composing range (else the selection) with `text`;
    /// the caret goes `cursor` bytes into it, else after it; `composes`
    /// keeps the new text as the composing range.
    fn replace(&mut self, text: &str, cursor: Option<usize>, composes: bool) -> Option<Event> {
        let field = self.field.as_mut()?;
        let (start, end) = self.composing.unwrap_or(ordered(field.selection));
        if text.is_empty() && self.composing.is_none() {
            return None;
        }
        let before = field.text.clone();
        let after = replaced(&before, (start, end), text)?;
        let length = units(text);
        let caret = start + cursor.map_or(length, |at| units(text.get(..at).unwrap_or(text)));
        self.composing = (composes && length > 0).then_some((start, start + length));
        field.text.clone_from(&after);
        field.selection = (caret, caret);
        Some(Event::TextEdited {
            before,
            text: after,
            selection: (caret, caret),
            composing: self.composing,
        })
    }
}

/// `(start, end)` with start first.
fn ordered((a, b): (u32, u32)) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// `text`'s length in UTF-16 units.
fn units(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

/// `text` with the UTF-16 range `(start, end)` replaced by `with`;
/// `None` for a range past the text or inside a character.
fn replaced(text: &str, (start, end): (u32, u32), with: &str) -> Option<String> {
    let byte = |at: u32| {
        let mut counted = 0;
        for (index, c) in text.char_indices() {
            if counted == at {
                return Some(index);
            }
            counted += u32::try_from(c.len_utf16()).ok()?;
        }
        (counted == at).then_some(text.len())
    };
    let (start, end) = (byte(start)?, byte(end)?);
    Some(format!("{}{with}{}", &text[..start], &text[end..]))
}

#[cfg(test)]
#[path = "ime_tests.rs"]
mod tests;
