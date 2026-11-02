//! What each pointer action does to the document and the gesture.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;

use super::DocumentInput;
use crate::pointer::Action;

impl DocumentInput {
    /// The window lost or regained focus: on a loss the releases of what
    /// is held happen outside and never come, so the routes held and the
    /// gesture are forgotten, and the document cancels its pointers.
    pub(super) fn window_focus(&mut self, document: &Rc<RefCell<Document>>, focused: bool) {
        if !focused {
            self.held = super::route::Held::default();
            self.touch.forget();
        }
        Document::window_focus(document, focused);
    }

    pub(super) fn act(&mut self, document: &Rc<RefCell<Document>>, action: Action) {
        match action {
            Action::Move(point) => {
                self.pointer = point;
                self.touch.mouse();
                Document::pointer_moved(document, point);
            }
            Action::Press(point, clicks) => {
                Document::pointer_pressed_clicks(document, point, clicks);
            }
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
            Action::Pan(delta) => self.pan(document, delta),
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

impl DocumentInput {
    /// A pan scrolls when the document lets it (`Document::touch_panned`,
    /// which holds the RELAXED ownership rule and cancels the touches).
    fn pan(&mut self, document: &Rc<RefCell<Document>>, delta: [f32; 2]) {
        if Document::touch_panned(document) {
            self.touch.pan(&mut document.borrow_mut(), delta);
        }
    }
}
