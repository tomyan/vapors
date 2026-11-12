//! The window lost focus while a pointer was held (the lead, on
//! review-v100 of #112): its release happens outside and never comes,
//! so each pointer held is cancelled as the platform taking it: the
//! captured (or pressed) element gets `pointercancel`, a range's drag or
//! a number's scrub ends with no `change`, and the capture and `:active`
//! end.

use std::cell::RefCell;
use std::rc::Rc;

use super::capture::{MOUSE, dispatch_pointer, process_pending, release_capture};
use crate::document::Document;

impl Document {
    /// Cancel every pointer held: each finger down, and the mouse while
    /// its button is down or it is captured.
    pub fn cancel_pointers(document: &Rc<RefCell<Document>>) {
        let fingers = document.borrow().pointer.touches.clone();
        for (finger, point) in fingers {
            Self::touch_cancelled(document, finger, point);
        }
        Self::mouse_cancelled(document);
    }

    fn mouse_cancelled(document: &Rc<RefCell<Document>>) {
        let (target, point) = {
            let doc = document.borrow();
            let captured = doc.captured(MOUSE).capture.filter(|n| doc.contains(*n));
            let pressed = doc.pointer.pressed.filter(|n| doc.contains(*n));
            let dragging = doc.pointer.drags.contains_key(&MOUSE);
            if captured.is_none() && pressed.is_none() && !dragging {
                return;
            }
            (
                captured.or(pressed),
                doc.pointer.position.unwrap_or_default(),
            )
        };
        if let Some(target) = target {
            let event = document
                .borrow()
                .pointer_event("pointercancel", MOUSE, target, point);
            dispatch_pointer(document, MOUSE, target, event);
        }
        crate::range_drag::dropped(document, MOUSE);
        crate::number_scrub::dropped(document, MOUSE);
        document.borrow_mut().pointer.text_presses.remove(&MOUSE);
        release_capture(document, MOUSE);
        process_pending(document, MOUSE, point);
        document.borrow_mut().release(None);
    }
}
