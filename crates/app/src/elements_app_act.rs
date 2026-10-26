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
    /// A pan scrolls when the document lets it (`Document::touch_panned`,
    /// which holds the RELAXED ownership rule and cancels the touches).
    fn pan(&mut self, delta: [f32; 2]) {
        let document = &self.document;
        if Document::touch_panned(document) {
            self.touch.pan(&mut document.borrow_mut(), delta);
        }
    }
}

impl ElementsApp {
    /// A key going down or up, to the document's focused element (3-E2a).
    pub(super) fn key(&mut self, event: &vapors_platform::Event) {
        let vapors_platform::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } = event
        else {
            return;
        };
        let modifiers = vapors_elements::Modifiers {
            shift: modifiers.shift,
            control: modifiers.control,
            alt: modifiers.alt,
            meta: modifiers.meta,
        };
        if *pressed {
            Document::key_pressed(&self.document, key, modifiers);
        } else {
            Document::key_released(&self.document, key, modifiers);
        }
    }
}
