//! Typed text (the platform's `Character`), to the focused field that
//! takes it: a text field (E6a) or a number field (3-E3c).

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;

impl Document {
    /// `text` was typed (the platform's `Character`): it goes to the
    /// focused text or number field, control characters left out.
    pub fn character_typed(document: &Rc<RefCell<Document>>, text: &str) {
        let printable: String = text.chars().filter(|c| !c.is_control()).collect();
        let Some(node) = document.borrow().active_element() else {
            return;
        };
        if printable.is_empty() {
            return;
        }
        if document.borrow().is_text_input(node) {
            crate::text_input::typed(document, node, &printable);
        } else {
            crate::number_input::typed(document, node, &printable);
        }
    }
}
