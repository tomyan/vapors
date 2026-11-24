//! A frame's Update phase (2-S1), the steps a host runs around its
//! `Document::update`: the `selectionchange`s, `scroll`s and `resize`s
//! owed from before the frame, the update and the focus fixup, the
//! `scroll`s and `resize`s that update owes (a clamped offset, a new
//! size), then, when anything changed since it, a second update and
//! fixup. A frame runs at most two updates; what the second leaves owed
//! fires at the next frame's start.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;

/// The second update of a frame left the document changed again.
const UNSETTLED: &str = "the document changed after each of two updates in one frame: a blur \
    listener of the focus fixup changes the document, or a hover style moves what is under the \
    pointer; fix: make the listener's change settle, or keep the hover style from moving boxes";

impl Document {
    /// Runs a frame's Update phase on `document`, `update` being the
    /// host's `Document::update` with its own checks.
    ///
    /// # Errors
    /// `update`'s error, or when the document changed after each of the
    /// two updates.
    pub fn update_frame(
        document: &Rc<RefCell<Document>>,
        mut update: impl FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        // the selectionchange each text field owes, once per frame, as
        // Chromium fires one per task (E6a)
        Document::settle_selection(document);
        // the scroll each container owes, once per frame before style
        // and layout, as Chromium's "run the scroll steps" (virtual list V1)
        Document::settle_scroll(document);
        update_and_settle(document, &mut update)?;
        // the scroll a clamp to the new layout owes, then the resize
        // that layout owes, as a ResizeObserver's callbacks run after
        // layout and before paint; what their handlers change the second
        // update lays out, so this frame draws it (virtual list V4)
        Document::settle_scroll(document);
        // the fixup's blur and the rehover under a resting pointer are
        // changes the update did not see; one more update shows them,
        // so the a11y take and the draw hold this frame (review-v242 H1)
        if !document.borrow().changed_since_update() {
            return Ok(());
        }
        update_and_settle(document, &mut update)?;
        if document.borrow().changed_since_update() {
            return Err(UNSETTLED.to_owned());
        }
        Ok(())
    }
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
