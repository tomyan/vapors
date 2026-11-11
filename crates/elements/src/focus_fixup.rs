//! The focus fixup (HTML's "focus fixup rule"): the focused element
//! loses focus once the last update shows it is no longer focusable,
//! having no box, `visibility: hidden` or being disabled, as Chromium
//! blurs it at the task after its next style update.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;

impl Document {
    /// Moves focus to nothing when the focused element is no longer
    /// focusable in the last update, with its commit, `blur` and
    /// `focusout` ([`Document::focus`]). The app calls this after each
    /// update. After a refused update nothing is known about boxes, so
    /// focus stays.
    pub fn settle_focus(document: &Rc<RefCell<Document>>) {
        if document.borrow().lost_focusability() {
            Document::focus(document, None);
        }
    }

    /// Whether an element has focus that the last successful update
    /// shows cannot have it; `false` without one.
    fn lost_focusability(&self) -> bool {
        let Some(focused) = self.active_element() else {
            return false;
        };
        self.painted.is_some() && !self.focusable(focused)
    }
}
