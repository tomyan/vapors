//! Touches as pointers (3-E1b): finger `n` is pointer `TOUCHES + n`,
//! captured by its `pointerdown` target without asking (Pointer Events
//! 3's implicit touch capture), so its moves and `pointerup` go there;
//! `pointercancel` ends a touch the platform or a scroll took. A touch
//! whose `pointerdown` listener called `set_pointer_capture` is owned by
//! that element, which the app reads before letting a pan scroll. The
//! click comes from the platform's tap, after the touch's own down and
//! up, so a tap is one click.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;

use super::capture::{
    TOUCHES, ask_unless_asked, dispatch_pointer, process_pending, release_capture,
};
use super::tree::dispatch;
use super::{Asked, Point};
use crate::document::Document;

/// Finger `finger`'s pointer id.
fn touch_id(finger: u64) -> u32 {
    TOUCHES + u32::try_from(finger).unwrap_or(u32::MAX - TOUCHES)
}

impl Document {
    /// Finger `finger` went down at `point`: its target becomes
    /// `:active`, gets `pointerdown`, and captures the touch.
    pub fn touch_pressed(document: &Rc<RefCell<Document>>, finger: u64, point: Point) {
        let id = touch_id(finger);
        let target = document.borrow().hit(point);
        let mut doc = document.borrow_mut();
        // `:active` follows the first finger down only (review-v78, M1)
        if doc.pointer.touches.is_empty() {
            doc.set_flag_chain(States::ACTIVE, target);
        }
        doc.pointer.touches.push((finger, point));
        drop(doc);
        let Some(target) = target else { return };
        let event = document
            .borrow()
            .pointer_event("pointerdown", id, target, point);
        dispatch_pointer(document, id, target, event);
        ask_unless_asked(document, id, Asked::CaptureTo(target));
    }

    /// Finger `finger` moved to `point`: its capturing element gets
    /// `pointermove`.
    pub fn touch_moved(document: &Rc<RefCell<Document>>, finger: u64, point: Point) {
        Self::touch_event(document, finger, point, "pointermove");
    }

    /// Finger `finger` lifted at `point`: `pointerup`, and the capture
    /// ends.
    pub fn touch_released(document: &Rc<RefCell<Document>>, finger: u64, point: Point) {
        Self::touch_ended(document, finger, point, "pointerup");
    }

    /// Finger `finger` was taken by the platform or a scroll:
    /// `pointercancel`, and the capture ends.
    pub fn touch_cancelled(document: &Rc<RefCell<Document>>, finger: u64, point: Point) {
        Self::touch_ended(document, finger, point, "pointercancel");
    }

    /// The platform's tap at `point`, after the touch's own down and up:
    /// `click` on the element there, unless a disabled button holds it.
    pub fn touch_tapped(document: &Rc<RefCell<Document>>, point: Point) {
        let target = document
            .borrow()
            .hit(point)
            .filter(|node| !document.borrow().in_disabled_button(*node));
        if let Some(target) = target {
            let event = document
                .borrow()
                .pointer_event("click", touch_id(0), target, point);
            dispatch(document, target, event);
        }
    }

    /// Whether finger `finger` is down and a listener captured it.
    #[must_use]
    pub fn touch_owned(&self, finger: u64) -> bool {
        let captured = self.captured(touch_id(finger));
        captured.explicit && (captured.capture.is_some() || captured.pending.is_some())
    }

    /// A pan took the touches: each finger down that no listener
    /// captured gets `pointercancel` where it last was. The app calls
    /// this at a gesture's first pan, as Chromium cancels then.
    pub fn cancel_unowned_touches(document: &Rc<RefCell<Document>>) {
        let touches = document.borrow().pointer.touches.clone();
        for (finger, point) in touches {
            if !document.borrow().touch_owned(finger) {
                Self::touch_cancelled(document, finger, point);
            }
        }
    }

    /// A pan of the fingers down: whether it may scroll. While a finger
    /// is owned by a listener's capture it may not; otherwise the pan
    /// takes the touches, each getting `pointercancel`, whether or not
    /// anything scrolls, as Chromium does (`fixtures/capture.events`).
    ///
    /// RELAXED (3-E1b, the lead's decision, 2026-10-26): ownership by
    /// `set_pointer_capture` stands in for `touch-action: none`, which
    /// vapors does not have; Chromium cancels an owned touch too. 3-E4's
    /// `touch-action` retires it.
    pub fn touch_panned(document: &Rc<RefCell<Document>>) -> bool {
        if document.borrow().any_touch_owned() {
            return false;
        }
        Self::cancel_unowned_touches(document);
        true
    }

    /// Whether any finger down now is owned by a listener's capture.
    #[must_use]
    pub fn any_touch_owned(&self) -> bool {
        self.pointer
            .touches
            .iter()
            .any(|(finger, _)| self.touch_owned(*finger))
    }

    fn touch_event(
        document: &Rc<RefCell<Document>>,
        finger: u64,
        point: Point,
        name: &str,
    ) -> Option<()> {
        let id = touch_id(finger);
        {
            let mut doc = document.borrow_mut();
            let down = doc.pointer.touches.iter_mut().find(|(f, _)| *f == finger)?;
            down.1 = point;
        }
        process_pending(document, id, point);
        let target = document.borrow().pointer_target(id, point)?;
        let event = document.borrow().pointer_event(name, id, target, point);
        dispatch_pointer(document, id, target, event);
        Some(())
    }

    fn touch_ended(document: &Rc<RefCell<Document>>, finger: u64, point: Point, name: &str) {
        let id = touch_id(finger);
        if Self::touch_event(document, finger, point, name).is_some() {
            release_capture(document, id);
            process_pending(document, id, point);
        }
        Self::forget_touch(document, finger, id);
    }

    /// Forgets finger `finger` and its capture, whether or not its last
    /// event found a target (review-v78, M2); the first finger takes
    /// `:active` with it.
    fn forget_touch(document: &Rc<RefCell<Document>>, finger: u64, id: u32) {
        let mut doc = document.borrow_mut();
        let primary = doc
            .pointer
            .touches
            .first()
            .is_some_and(|(first, _)| *first == finger);
        doc.pointer.touches.retain(|(down, _)| *down != finger);
        doc.pointer.captures.remove(&id);
        if primary && doc.pointer.pressed.is_none() {
            doc.set_flag_chain(States::ACTIVE, None);
        }
    }
}
