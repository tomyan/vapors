//! A touch tap (the platform's `Tap`, after the touch's own down and up):
//! focus, `click`, and a text field's caret, in Chromium's order; the
//! second tap of a run (the platform's `Tap` with a count of 2) also
//! sends `dblclick`, as desktop headless Chromium 155 does for CDP touch
//! taps (the corpus's `clicks.events`). Chrome on Android, on a page
//! without double-tap zoom, is assumed to do the same; not yet measured
//! on the Pixel.

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

    /// The platform's double tap at `point`, after that tap's own
    /// [`Document::touch_tapped`]: `dblclick` on the element the tap
    /// clicked, none when a disabled control holds it.
    pub fn touch_double_tapped(document: &Rc<RefCell<Document>>, point: Point) {
        let Some(hit) = document.borrow().hit(point) else {
            return;
        };
        if document.borrow().in_disabled_button(hit) {
            return;
        }
        let event = document
            .borrow()
            .pointer_event("dblclick", touch_id(0), hit, point);
        crate::events::tree::dispatch_bubbling(document, hit, event);
    }
}
