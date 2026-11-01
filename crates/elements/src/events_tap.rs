//! A touch tap (the platform's `Tap`, after the touch's own down and up):
//! focus, `click`, and a text field's caret, in Chromium's order.

use std::cell::RefCell;
use std::rc::Rc;

use crate::document::Document;
use crate::events::Point;
use crate::events::touch::touch_id;

impl Document {
    /// The platform's tap at `point`, after the touch's own down and up:
    /// focus moves to what the tap is on (or leaves), then `click` on
    /// the element there unless a disabled control holds it, then a
    /// text field takes the caret at the tap, as Chromium orders a tap.
    pub fn touch_tapped(document: &Rc<RefCell<Document>>, point: Point) {
        let hit = document.borrow().hit(point);
        // a tap whose pointerdown a listener cancelled still clicks, but
        // moves neither focus nor a caret, as in Chromium
        let prevented = std::mem::take(&mut document.borrow_mut().pointer.touch_down_prevented);
        if !prevented {
            let focusable = hit.and_then(|node| document.borrow().focusable_holding(node));
            Self::focus(document, focusable);
        }
        let Some(hit) = hit else { return };
        if !document.borrow().in_disabled_button(hit) {
            let event = document
                .borrow()
                .pointer_event("click", touch_id(0), hit, point);
            crate::events::tree::dispatch_click(document, hit, event);
        }
        if !prevented {
            crate::text_pointer::tapped(document, hit, point);
        }
    }
}
