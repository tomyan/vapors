//! What each pointer action does to the document and the gesture.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;
use vapors_platform::Instant;

use super::DocumentInput;
use crate::pointer::Action;

impl DocumentInput {
    /// The window lost or regained focus: on a loss the releases of what
    /// is held happen outside and never come, so the routes held and the
    /// gesture are forgotten, and the document cancels its pointers.
    pub(super) fn window_focus(
        &mut self,
        document: &Rc<RefCell<Document>>,
        focused: bool,
        now: Instant,
    ) {
        if !focused {
            self.held = super::route::Held::default();
            self.touch.forget(now);
        }
        Document::window_focus(document, focused);
    }

    /// `action`, handled at `now`: the platform's time for the event
    /// (`Platform::now`), never the clock read here (#440).
    pub(super) fn act(&mut self, document: &Rc<RefCell<Document>>, action: Action, now: Instant) {
        match action {
            Action::Move(point) => {
                self.pointer = Some(point);
                self.touch.mouse();
                Document::pointer_moved(document, point);
            }
            Action::Press(point, clicks, modifiers) => {
                self.touch.stop_fling();
                Document::pointer_pressed_with(document, point, clicks, modifiers);
            }
            Action::Release(point, modifiers) => {
                Document::pointer_released_with(document, point, modifiers);
            }
            Action::Leave => Document::pointer_left(document),
            Action::Tap(point, clicks) => {
                Document::touch_tapped(document, point);
                if clicks.get() == 2 {
                    Document::touch_double_tapped(document, point);
                }
            }
            // the tap's count sends dblclick; the photo's pairing is not
            // the DOM's (review-v100 R2)
            Action::DoubleTap(_) => {}
            Action::Wheel(point, delta) => {
                self.touch.mouse();
                self.touch.stop_fling();
                self.wheel
                    .scroll(&mut document.borrow_mut(), point, delta, now);
            }
            Action::TouchDown(id, point) => {
                self.touch.down(id, point, now);
                Document::touch_pressed(document, id, point);
            }
            Action::TouchMove(id, point) => Document::touch_moved(document, id, point),
            Action::Pan(delta) => self.pan(document, delta, now),
            Action::Fling(velocity, at) => self.released(&document.borrow(), velocity, at),
            Action::TouchUp(id, point) => {
                self.touch.up(id, now);
                Document::touch_released(document, id, point);
            }
            Action::TouchCancel(id, point) => {
                self.touch.up(id, now);
                Document::touch_cancelled(document, id, point);
            }
        }
        self.touch.draw_thumbs(&mut document.borrow_mut(), now);
    }
}

impl DocumentInput {
    /// A pan scrolls when the document lets it (`Document::touch_panned`,
    /// which holds the RELAXED ownership rule and cancels the touches).
    fn pan(&mut self, document: &Rc<RefCell<Document>>, delta: [f32; 2], now: Instant) {
        if Document::touch_panned(document) {
            self.touch.pan(&mut document.borrow_mut(), delta, now);
        }
    }
}

#[cfg(test)]
#[path = "document_input_act_tests.rs"]
mod tests;
