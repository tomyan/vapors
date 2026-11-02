//! The window losing and regaining focus, as headed Chromium 155 orders
//! it (`tests/focus_window.rs`): on the loss a number or text field
//! commits, then the focused element gets `blur` and `focusout` and
//! stays the active element; on the return it gets `focus` and
//! `focusin`. A repeated notice fires nothing. While the window is away
//! a focus move fires nothing (`Document::focus`), so the return focuses
//! whatever has focus then, once, as Chromium fires focus events only
//! when the page regains focus. The loss cancels every pointer held
//! (`events_cancel.rs`): its release would happen outside. On the web
//! the window is the canvas: focusing the page's own HTML (touché's Ask
//! field) is a loss too, and blurs the card.

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
    /// Whether the window is away: lost focus and not regained it.
    pub(crate) fn window_away(&self) -> bool {
        self.window == Window::Blurred
    }

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
        if focused {
            Self::window_regained(document);
        } else {
            Self::window_lost(document);
        }
    }

    fn window_lost(document: &Rc<RefCell<Document>>) {
        Self::cancel_pointers(document);
        let Some(node) = document.borrow().active_element() else {
            return;
        };
        crate::number_input::commit(document, node);
        crate::text_input::commit(document, node);
        crate::focus::fire(document, node, &["blur", "focusout"]);
    }

    fn window_regained(document: &Rc<RefCell<Document>>) {
        if let Some(node) = document.borrow().active_element() {
            crate::focus::fire(document, node, &["focus", "focusin"]);
        }
    }
}
