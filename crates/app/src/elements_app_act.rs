//! What each pointer action does to the document and the gesture.

use vapors_elements::Document;

use super::ElementsApp;
use crate::pointer::Action;

impl ElementsApp {
    pub(super) fn act(&mut self, action: Action) {
        let document = &self.document;
        match action {
            Action::Move(point) => {
                self.pointer = point;
                self.touch.mouse();
                Document::pointer_moved(document, point);
            }
            Action::Press(point) => Document::pointer_pressed(document, point),
            Action::Release(point) => Document::pointer_released(document, point),
            Action::Leave => Document::pointer_left(document),
            Action::Tap(point) => Document::tapped(document, point),
            Action::Wheel(point, delta) => {
                self.touch.mouse();
                let now = vapors_platform::Instant::now();
                self.wheel
                    .scroll(&mut document.borrow_mut(), point, delta, now);
            }
            Action::TouchDown(id, point) => self.touch.down(id, point),
            Action::Pan(delta) => _ = self.touch.pan(&mut document.borrow_mut(), delta),
            Action::TouchUp(id) => self.touch.up(id),
        }
        document
            .borrow_mut()
            .set_thumbs_shown(self.touch.thumbs_shown());
    }
}
