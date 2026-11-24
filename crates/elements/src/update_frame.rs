//! A frame's Update phase (2-S1), the steps a host runs around its
//! `Document::update`: the `selectionchange`s, `scroll`s and `resize`s
//! owed from before the frame, the update and the focus fixup, the
//! `resize`s that update owes, then, when anything changed since it, a
//! second update and fixup. A frame whose `resize`s fired runs a third
//! when the second left the document changed (the hover moved by rows
//! built under a resting pointer, or the fixup's blur);
//! any other runs at most two. What the last leaves owed fires at the
//! next frame's start.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;

/// The last update of a frame left the document changed again.
fn unsettled(updates: &str) -> String {
    format!(
        "the document changed after each of {updates} updates in one frame: a blur listener of \
         the focus fixup changes the document, a hover style moves what is under the pointer, or \
         a `resize` handler's change moves it again; fix: make the listener's change settle, or \
         keep the hover style from moving boxes"
    )
}

impl Document {
    /// Runs a frame's Update phase on `document`, `update` being the
    /// host's `Document::update` with its own checks.
    ///
    /// # Errors
    /// `update`'s error, or when the document changed after each of the
    /// frame's updates.
    pub fn update_frame(
        document: &Rc<RefCell<Document>>,
        mut update: impl FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        // the selectionchange each text field owes, once per frame, as
        // Chromium fires one per task (E6a)
        Document::settle_selection(document);
        // the scroll each container owes, once per frame before style
        // and layout, as Chromium's "run the scroll steps" (virtual list
        // V1); an offset the update clamps fires at the next frame's
        Document::settle_scroll(document);
        update_and_settle(document, &mut update)?;
        // the resize the update owes, carrying the laid-out (clamped)
        // offset, after layout as a ResizeObserver's callbacks run;
        // what its handlers change the second update lays out
        let resized = Document::settle_resize(document);
        // the fixup's blur and the rehover under a resting pointer are
        // changes the update did not see; one more update shows them,
        // so the a11y take and the draw hold this frame (review-v242 H1)
        if !document.borrow().changed_since_update() {
            return Ok(());
        }
        update_and_settle(document, &mut update)?;
        if !resized {
            return settled(document, "two");
        }
        // what the second update and fixup changed: the hover the
        // resize handlers' rows moved under a resting pointer, or a blur;
        // one more update shows it (virtual list V4)
        if document.borrow().changed_since_update() {
            update_and_settle(document, &mut update)?;
        }
        settled(document, "three")
    }
}

/// Refuses a frame whose last update left the document changed.
fn settled(document: &Rc<RefCell<Document>>, updates: &str) -> Result<(), String> {
    if document.borrow().changed_since_update() {
        return Err(unsettled(updates));
    }
    Ok(())
}

/// `update`, then the focus fixup: an element the update left without
/// a box, or disabled, loses focus before the next event can reach it.
fn update_and_settle(
    document: &Rc<RefCell<Document>>,
    update: &mut impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    update()?;
    Document::settle_focus(document);
    Ok(())
}
