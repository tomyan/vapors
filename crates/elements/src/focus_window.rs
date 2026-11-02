//! The window losing and regaining focus, as headed Chromium 155 orders
//! it (`tests/focus_window.rs`): on the loss a number or text field
//! commits, then the focused element gets `blur` and `focusout` and
//! stays the active element; on the return it gets `focus` and
//! `focusin`. A repeated notice fires nothing.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;

/// Whether the window has focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Window {
    Focused,
    Blurred,
}

impl Document {
    /// The window lost focus (`focused` false) or regained it.
    pub fn window_focus(document: &Rc<RefCell<Document>>, focused: bool) {
        let now = if focused {
            Window::Focused
        } else {
            Window::Blurred
        };
        if std::mem::replace(&mut document.borrow_mut().window, now) == now {
            return;
        }
        let Some(node) = document.borrow().active_element() else {
            return;
        };
        if now == Window::Blurred {
            crate::number_input::commit(document, node);
            crate::text_input::commit(document, node);
            crate::focus::fire(document, node, &["blur", "focusout"]);
        } else {
            crate::focus::fire(document, node, &["focus", "focusin"]);
        }
    }
}
