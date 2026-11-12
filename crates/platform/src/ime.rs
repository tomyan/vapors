//! The system IME's composition over the focused text field (E6c), as
//! the edits the web's text input sends (`Event::TextEdited`): composing
//! text is in the field's value while it composes, as Chromium's own
//! `<input>` holds it (doc 03), replacing the selection when it starts
//! and its own range after; a commit replaces the composing range and
//! ends it, in one edit, so one undo goes back to the text before the
//! composition, as on the web. winit sends an empty preedit right
//! before each commit; that clear is held until the next event, which
//! takes it in (a commit) or applies it (anything else). Kept against
//! the field the app last set, which the app's next set confirms.

use winit::event::Ime;

use crate::{Event, TextField};

/// The field the app last set, and the range an IME is composing in it.
#[derive(Default)]
pub(crate) struct Composition {
    field: Option<TextField>,
    composing: Option<(u32, u32)>,
    /// An empty preedit while composing, not yet applied.
    clear_held: bool,
}

impl Composition {
    /// The app set the focused field, or none. A composition ends, its
    /// text kept, with the field's focus, with another field, when the
    /// field holds another text than the composition made (the app
    /// refused an edit, or changed the text), or when the selection
    /// moves out of the composing range (a click elsewhere in the
    /// field), as Chromium's `<input>` commits it. Answers whether a
    /// composition ended so, for the IME to be told.
    pub(crate) fn set_field(&mut self, field: Option<TextField>) -> bool {
        let kept = self
            .composing
            .is_some_and(|range| match (&self.field, &field) {
                (Some(was), Some(now)) => {
                    was.id == now.id && was.text == now.text && inside(now.selection, range)
                }
                _ => false,
            });
        let ended = self.composing.is_some() && !kept;
        if ended {
            self.composing = None;
            self.clear_held = false;
        }
        self.field = field;
        ended
    }

    /// The field the app last set.
    pub(crate) fn field(&self) -> Option<&TextField> {
        self.field.as_ref()
    }

    /// The edit `ime` makes to the field, if it changes anything.
    pub(crate) fn edit(&mut self, ime: &Ime) -> Option<Event> {
        match ime {
            // winit's clear before a commit: held for the commit
            Ime::Preedit(text, _) if text.is_empty() && self.composing.is_some() => {
                self.clear_held = true;
                None
            }
            Ime::Preedit(text, cursor) => self.replace(text, cursor.map(|(_, end)| end), true),
            Ime::Commit(text) => self.replace(text, None, false),
            Ime::Disabled => self.replace("", None, false),
            Ime::Enabled => None,
        }
    }

    /// A held clear that no commit followed, applied before the next
    /// event that is not the IME's (the IME cancelled its preedit).
    pub(crate) fn flush(&mut self) -> Option<Event> {
        if !self.clear_held {
            return None;
        }
        self.replace("", None, false)
    }

    /// Replaces the composing range (else the selection) with `text`;
    /// the caret goes `cursor` bytes into it, else after it; `composes`
    /// keeps the new text as the composing range.
    fn replace(&mut self, text: &str, cursor: Option<usize>, composes: bool) -> Option<Event> {
        self.clear_held = false;
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

/// Whether the selection `(a, b)` lies within `range`, its ends included.
fn inside((a, b): (u32, u32), (start, end): (u32, u32)) -> bool {
    let (low, high) = ordered((a, b));
    start <= low && high <= end
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

#[cfg(test)]
#[path = "ime_end_tests.rs"]
mod end_tests;
