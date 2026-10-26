//! What each pointer action does to the document and the gesture.

use vapors_elements::Document;

use super::ElementsApp;
use crate::pointer::Action;

impl ElementsApp {
    pub(super) fn act(&mut self, action: Action) {
        let document = &std::rc::Rc::clone(&self.document);
        match action {
            Action::Move(point) => {
                self.pointer = point;
                self.touch.mouse();
                Document::pointer_moved(document, point);
            }
            Action::Press(point) => Document::pointer_pressed(document, point),
            Action::Release(point) => Document::pointer_released(document, point),
            Action::Leave => Document::pointer_left(document),
            Action::Tap(point) => Document::touch_tapped(document, point),
            Action::Wheel(point, delta) => {
                self.touch.mouse();
                let now = vapors_platform::Instant::now();
                self.wheel
                    .scroll(&mut document.borrow_mut(), point, delta, now);
            }
            Action::TouchDown(id, point) => {
                self.touch.down(id, point);
                Document::touch_pressed(document, id, point);
            }
            Action::TouchMove(id, point) => Document::touch_moved(document, id, point),
            Action::Pan(delta) => self.pan(delta),
            Action::TouchUp(id, point) => {
                self.touch.up(id);
                Document::touch_released(document, id, point);
            }
            Action::TouchCancel(id, point) => {
                self.touch.up(id);
                Document::touch_cancelled(document, id, point);
            }
        }
        document
            .borrow_mut()
            .set_thumbs_shown(self.touch.thumbs_shown());
    }
}

impl ElementsApp {
    /// A pan scrolls unless a listener owns a touch, and the first pan
    /// cancels the touches no listener owns, whether or not anything
    /// scrolls, as Chromium does with `touch-action: auto`
    /// (`fixtures/capture.events`).
    ///
    /// RELAXED (3-E1b, the lead's decision, 2026-10-26): a touch whose
    /// `pointerdown` listener called `set_pointer_capture` owns its pans,
    /// standing in for `touch-action: none`, which vapors does not have.
    /// Chromium cancels that touch too (the thumb in capture.events,
    /// left out of the oracle's comparison for this reason). 3-E4's
    /// `touch-action` retires this rule.
    fn pan(&mut self, delta: [f32; 2]) {
        let document = &self.document;
        if document.borrow().any_touch_owned() {
            return;
        }
        Document::cancel_unowned_touches(document);
        self.touch.pan(&mut document.borrow_mut(), delta);
    }
}
