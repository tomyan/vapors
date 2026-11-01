//! A text field's own undo and redo (E6b), separate from the app's
//! document history, grouped as Chromium groups a field's edits: a run
//! of typing is one step; a run of Backspace is one step, and a Delete
//! joins it; a run of Delete is one step, and a Backspace does not join
//! it; a word deletion (Control+Backspace, Control+Delete) always starts
//! a step, which cluster deletions in its direction and typing then
//! join; typing after a deletion joins it, a deletion after typing does
//! not; a paste is a step alone; anything that moves the selection
//! other than an edit ends the run. A step keeps the text and selection
//! before and after it. At most [`LIMIT`] steps are kept.

use std::ops::Range;

/// What an edit did, for grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditKind {
    /// Typed text.
    Insert,
    /// Backspace, by cluster (or a selection).
    DeleteBackward,
    /// Delete, by cluster (or a selection).
    DeleteForward,
    /// Control+Backspace with a collapsed selection.
    DeleteWordBackward,
    /// Control+Delete with a collapsed selection.
    DeleteWordForward,
    /// A paste.
    Paste,
}

/// The most undo steps a field keeps; the oldest goes first. Chromium's
/// own: `kMaximumUndoStackDepth` in Blink's
/// `core/editing/commands/undo_stack.cc`.
pub(crate) const LIMIT: usize = 1000;

/// A field's text and selection (bytes), at one moment.
pub(crate) type Snapshot = (String, Range<usize>);

struct Step {
    before: Snapshot,
    after: Snapshot,
}

/// One field's undo and redo stacks.
#[derive(Default)]
pub(crate) struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// The kind of the step still open to a following edit.
    open: Option<EditKind>,
}

impl History {
    /// Records an edit from `before` to `after`: it joins the open step
    /// when Chromium would group them, else starts one; nothing is left
    /// to redo.
    pub(crate) fn edited(&mut self, kind: EditKind, before: Snapshot, after: Snapshot) {
        self.redo.clear();
        let joins = self.open.is_some_and(|open| joins(open, kind));
        match self.undo.last_mut() {
            Some(step) if joins => step.after = after,
            _ => self.push(Step { before, after }),
        }
        self.open = opens(kind);
    }

    fn push(&mut self, step: Step) {
        if self.undo.len() == LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(step);
    }

    /// The selection moved other than by an edit: the open step closes.
    pub(crate) fn close(&mut self) {
        self.open = None;
    }

    /// The text and selection to go back to, if any step is left.
    pub(crate) fn undo(&mut self) -> Option<Snapshot> {
        self.open = None;
        let step = self.undo.pop()?;
        let before = step.before.clone();
        self.redo.push(step);
        Some(before)
    }

    /// The text and selection to go forward to, if an undone step is left.
    pub(crate) fn redo(&mut self) -> Option<Snapshot> {
        self.open = None;
        let step = self.redo.pop()?;
        let after = step.after.clone();
        self.undo.push(step);
        Some(after)
    }
}

/// Whether an edit of `kind` joins an open step of kind `open`.
fn joins(open: EditKind, kind: EditKind) -> bool {
    use EditKind::{DeleteBackward, DeleteForward, Insert};
    match kind {
        Insert => true,
        DeleteBackward => open == DeleteBackward,
        DeleteForward => matches!(open, DeleteForward | DeleteBackward),
        _ => false,
    }
}

/// The kind a step is open as after an edit of `kind`: a word
/// deletion's as its direction's; none after a paste.
fn opens(kind: EditKind) -> Option<EditKind> {
    match kind {
        EditKind::DeleteWordBackward => Some(EditKind::DeleteBackward),
        EditKind::DeleteWordForward => Some(EditKind::DeleteForward),
        EditKind::Paste => None,
        other => Some(other),
    }
}

#[cfg(test)]
#[path = "text_history_tests.rs"]
mod tests;
