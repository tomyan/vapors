//! The focus fixup (HTML's "focus fixup rule"): the focused element
//! loses focus once it is no longer focusable by the rule
//! [`Document::focus`] uses (`focus_style.rs`): `display: none` on it
//! or an ancestor, `visibility` other than `visible`, or disabled, as
//! Chromium blurs it at the task after its next style update.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;
use crate::focus_style::Focus;

impl Document {
    /// Moves focus to nothing when the focused element is no longer
    /// focusable, with its commit, `blur` and `focusout`
    /// ([`Document::focus`]). A host calls this after each successful
    /// update; `ElementsApp` does. After a refused update focus stays.
    /// A focus requested before styles could be computed
    /// ([`Focus::Pending`]) is applied here, or dropped with no event
    /// when the element is not focusable now.
    pub fn settle_focus(document: &Rc<RefCell<Document>>) {
        let pending = std::mem::take(&mut document.borrow_mut().settle).pending;
        if document.borrow().lost_focusability() {
            let _ = Document::focus(document, None);
        }
        if pending.is_some() {
            let _ = Document::focus(document, pending);
        }
    }

    /// Whether an element has focus that, after a successful update,
    /// cannot have it; `false` without one.
    pub(crate) fn lost_focusability(&self) -> bool {
        let Some(focused) = self.active_element() else {
            return false;
        };
        self.painted.is_some() && self.focusability(focused) == Focus::NotFocusable
    }

    /// Whether a successful update has run since the last
    /// [`Document::settle_focus`] and the focused element is not
    /// focusable: a host that drives its own frames forgot the settle.
    #[must_use]
    pub fn unsettled_focus(&self) -> bool {
        self.settle.due && self.lost_focusability()
    }
}

/// What the next [`Document::settle_focus`] owes.
#[derive(Default)]
pub(crate) struct Settle {
    /// A successful update ran since the last settle.
    pub(crate) due: bool,
    /// The element a [`Focus::Pending`] focus asked for.
    pub(crate) pending: Option<vapors_dom::NodeId>,
}
