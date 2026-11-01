//! Pasted text into the focused text field (E6b), as Chromium pastes:
//! `paste` fires on the field first, bubbling, and a listener that
//! prevents its default keeps the text out; else the text replaces the
//! selection with `input`, as an undo step of its own, each line break
//! a space, as Chromium pastes into a one-line field. The platform
//! sends a paste whatever has focus; the app hands it here first, and
//! has it itself when no live text field is focused.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::Event;

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;
use crate::text_history::EditKind;

impl Document {
    /// Whether a text field the user can edit has focus: a paste would
    /// go to it.
    #[must_use]
    pub fn text_field_focused(&self) -> bool {
        self.active_element()
            .is_some_and(|node| self.live_text(node))
    }

    /// `text` was pasted: whether the focused text field took it (also
    /// when a `paste` listener kept it out); `false` when no live text
    /// field has focus, so the paste is the app's.
    pub fn text_pasted(document: &Rc<RefCell<Document>>, text: &str) -> bool {
        let focused = document.borrow().active_element();
        let Some(node) = focused.filter(|node| document.borrow().live_text(*node)) else {
            return false;
        };
        if dispatch_bubbling(document, node, Event::new("paste")) {
            return true;
        }
        let line = one_line(text);
        document.borrow_mut().edit(node, EditKind::Paste, |d| {
            d.insert_or_replace_selection(&line);
        });
        crate::text_input::edited(document, node);
        true
    }
}

/// `text` on one line: each line break (CR LF, CR or LF) a space.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}
