//! Keys and typed text into the document: a key going down or up to its
//! focused element, and whether a key going down was used.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;
use vapors_platform::Event;

/// A key going down or up, to the document's focused element (3-E2a);
/// the text a key produced, to a focused field (3-E3c). For a key going
/// down, whether the document used it.
pub(super) fn key(document: &Rc<RefCell<Document>>, event: &Event) -> Option<bool> {
    if let Event::Character(text) = event {
        Document::character_typed(document, text);
        return None;
    }
    let Event::Key {
        key,
        pressed,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    let modifiers = vapors_elements::Modifiers {
        shift: modifiers.shift,
        control: modifiers.control,
        alt: modifiers.alt,
        meta: modifiers.meta,
    };
    if *pressed {
        Some(Document::key_pressed(document, key, modifiers))
    } else {
        Document::key_released(document, key, modifiers);
        None
    }
}
