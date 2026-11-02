//! The focused text field as the platform's text input sees it (E6, the
//! web page's hidden textarea): the field's text, selection and caret
//! area for the platform to mirror, and the textarea's edits coming
//! back. An edit is the textarea's whole state after it, with the text
//! it was made against; composing text is in the value, underlined, as
//! Chromium's own `<input>` holds it. Selections count UTF-16 units, as
//! the DOM's do.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::text_history::EditKind;

/// The focused text field, for the platform to mirror.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInput {
    /// The field's node.
    pub node: NodeId,
    /// Its text.
    pub text: String,
    /// Its selection, start then end, in UTF-16 units.
    pub selection: (usize, usize),
    /// The caret's rect `[x, y, width, height]` in CSS px from the
    /// viewport's top left (the selection's last rect when it is not
    /// collapsed), for the IME's candidate window.
    pub area: [f32; 4],
}

/// The platform's text input after an edit.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEdit {
    /// The text the edit was made against: the last one mirrored.
    pub before: String,
    /// The text after it.
    pub text: String,
    /// The selection after it, in UTF-16 units of `text`.
    pub selection: (usize, usize),
    /// The range an IME is composing, in UTF-16 units of `text`.
    pub composing: Option<(usize, usize)>,
}

impl Document {
    /// The focused live text field as the platform's text input
    /// mirrors it; `None` when none has focus or it is not painted yet.
    pub fn text_input(&mut self) -> Option<TextInput> {
        let node = self.active_element().filter(|node| self.live_text(*node))?;
        let origin = *self.painted.as_ref()?.text_origins.get(&node)?;
        let selection = self.selection(node)?;
        let text = self.editor(node).plain.raw_text().to_owned();
        let (_, marks) = self.focused_text_marks()?;
        let [x, y, width, height] = marks.caret.or(marks.selection.last().copied())?;
        Some(TextInput {
            node,
            text,
            selection,
            area: [origin[0] + x, origin[1] + y, width.max(1.0), height],
        })
    }

    /// The platform's text input was edited: the focused field takes
    /// its text, selection and composing range, with `input`.
    ///
    /// # Errors
    /// No live text field has focus, the edit was made against a text
    /// the field no longer holds, or a range falls past the text or
    /// inside a character; the field is left as it was.
    pub fn text_edited(document: &Rc<RefCell<Document>>, edit: &TextEdit) -> Result<(), String> {
        let node = document.borrow_mut().edited_field(&edit.before)?;
        let selection = bytes(&edit.text, edit.selection, "selection")?;
        let composing = match edit.composing {
            Some(range) => Some(bytes(&edit.text, range, "composing range")?),
            None => None,
        };
        document.borrow_mut().take_edit(node, edit, selection);
        document.borrow_mut().editor(node).composing = composing;
        crate::text_input::edited(document, node);
        Ok(())
    }

    /// The focused live text field an edit against `before` is for.
    fn edited_field(&mut self, before: &str) -> Result<NodeId, String> {
        let node = self
            .active_element()
            .filter(|node| self.live_text(*node))
            .ok_or("the text input was edited with no text field focused")?;
        let holds = self.editor(node).plain.raw_text();
        if holds != before {
            return Err(format!(
                "the text input's edit was made against {before:?}, but the field holds {holds:?}; the platform mirrors the field again"
            ));
        }
        Ok(node)
    }

    /// Replaces what changed between the field's text and `edit.text`,
    /// recorded for undo, then selects `selection` (bytes).
    fn take_edit(&mut self, node: NodeId, edit: &TextEdit, selection: Range<usize>) {
        let changed = changed(&edit.before, &edit.text);
        let inserted = &edit.text
            [changed.start..changed.start + changed.len() + edit.text.len() - edit.before.len()];
        let kind = if inserted.is_empty() {
            EditKind::DeleteBackward
        } else {
            EditKind::Insert
        };
        if edit.before != edit.text {
            self.edit(node, kind, |d| {
                d.select_byte_range(changed.start, changed.end);
                d.insert_or_replace_selection(inserted);
            });
        }
        self.drive_editor(node, |d| {
            d.select_byte_range(selection.start, selection.end);
        });
    }
}

/// The byte range of `before` that differs from `after`: what is left
/// between their common start and common end, on character boundaries
/// (equal characters are equal bytes, so a boundary in one is one in
/// the other).
fn changed(before: &str, after: &str) -> Range<usize> {
    let start = before
        .char_indices()
        .zip(after.chars())
        .find(|((_, a), b)| a != b)
        .map_or(before.len().min(after.len()), |((i, _), _)| i);
    let start = (0..=start)
        .rev()
        .find(|i| before.is_char_boundary(*i) && after.is_char_boundary(*i))
        .unwrap_or(0);
    let room = before.len().min(after.len()) - start;
    let common_end = before[start..]
        .chars()
        .rev()
        .zip(after[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>()
        .min(room);
    start..before.len() - common_end
}

/// `range` in UTF-16 units of `text` as bytes, refused past the text,
/// inside a character or backwards.
fn bytes(text: &str, (start, end): (usize, usize), what: &str) -> Result<Range<usize>, String> {
    let byte = |units: usize| {
        let mut counted = 0;
        for (index, c) in text.char_indices() {
            if counted == units {
                return Some(index);
            }
            counted += c.len_utf16();
        }
        (counted == units).then_some(text.len())
    };
    match (byte(start), byte(end)) {
        (Some(start), Some(end)) if start <= end => Ok(start..end),
        _ => Err(format!(
            "the text input's {what} {start}..{end} is not a range of {text:?} in UTF-16 units"
        )),
    }
}

#[cfg(test)]
#[path = "text_bridge_tests.rs"]
mod tests;
