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
    /// A pan scrolls unless a listener owns a touch, and a pan that
    /// scrolls cancels the touches no listener owns.
    ///
    /// RELAXED (3-E1b, the lead's decision, 2026-10-26): who owns a touch
    /// is decided by whether a `pointerdown` listener called
    /// `set_pointer_capture`, because vapors has no `touch-action` yet.
    /// 3-E4's `touch-action` retires this rule.
    fn pan(&mut self, delta: [f32; 2]) {
        let document = &self.document;
        if document.borrow().any_touch_owned() {
            return;
        }
        if self.touch.pan(&mut document.borrow_mut(), delta) {
            Document::cancel_unowned_touches(document);
        }
    }
}
