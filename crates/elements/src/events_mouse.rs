//! The mouse's button with its click count and modifiers: the press and
//! the release, their events carrying the modifiers held, and
//! `dblclick` after the `click` of the second press in a row, as the
//! platform counts presses, to the element that took the click, as
//! Chromium sends it.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_dom::NodeId;

use super::Point;
use super::capture::{MOUSE, dispatch_pointer, process_pending, release_capture};
use super::tree::dispatch_bubbling;
use crate::document::Document;

impl Document {
    /// The primary button went down at `point`, the `clicks`th press in a
    /// row, with `modifiers` held: as [`Document::pointer_pressed_clicks`],
    /// its events carrying the modifiers.
    pub fn pointer_pressed_with(
        document: &Rc<RefCell<Document>>,
        point: Point,
        clicks: NonZeroU32,
        modifiers: crate::Modifiers,
    ) {
        document.borrow_mut().pointer.mouse_press = (Some(clicks), modifiers);
        process_pending(document, MOUSE, point);
        document.borrow_mut().keyboard = false;
        let target = document.borrow().pointer_target(MOUSE, point);
        let target = document.borrow_mut().press(target);
        let prevented = target.is_some_and(|target| {
            let event = document
                .borrow()
                .pointer_event("pointerdown", MOUSE, target, point);
            dispatch_pointer(document, MOUSE, target, event)
        });
        if prevented {
            return;
        }
        // a listener may have removed the target: a removed one holds
        // nothing focusable, so the press blurs, as Chromium's does
        let target = target.and_then(|target| document.borrow().still_in(target));
        // a range takes the value at the press before focus moves, as
        // Chromium orders `input` before `focusin` (3-E3a)
        if let Some(target) = target {
            crate::range_drag::pressed(document, MOUSE, target, point);
            crate::number_scrub::pressed(document, MOUSE, target, point);
        }
        // a press focuses what it is on, or blurs, unless prevented (3-E2a)
        let focusable = target.and_then(|target| document.borrow().focusable_holding(target));
        Self::focus(document, focusable);
        // a text field's caret lands where the press did, after focus
        // so keyboard focus's selection does not override it (E6a)
        if let Some(target) = target.and_then(|target| document.borrow().still_in(target)) {
            crate::text_pointer::pressed(document, MOUSE, target, point, clicks);
        }
    }

    /// The primary button came up at `point` with `modifiers` held: as
    /// [`Document::pointer_released`], its events carrying the modifiers,
    /// and `dblclick` after the click of the second press in a row.
    pub fn pointer_released_with(
        document: &Rc<RefCell<Document>>,
        point: Point,
        modifiers: crate::Modifiers,
    ) {
        document.borrow_mut().pointer.mouse_press.1 = modifiers;
        process_pending(document, MOUSE, point);
        let target = document.borrow().pointer_target(MOUSE, point);
        if let Some(target) = target {
            let event = document
                .borrow()
                .pointer_event("pointerup", MOUSE, target, point);
            dispatch_pointer(document, MOUSE, target, event);
        }
        crate::range_drag::released(document, MOUSE);
        crate::number_scrub::released(document, MOUSE);
        release_capture(document, MOUSE);
        process_pending(document, MOUSE, point);
        let click = document.borrow_mut().release(target);
        if let Some(click) = click {
            let event = document
                .borrow()
                .pointer_event("click", MOUSE, click, point);
            crate::events::tree::dispatch_click(document, click, event);
            double_click(document, click, point);
        }
        document.borrow_mut().pointer.mouse_press.0 = None;
        // a press held inside a text field's selection moves the caret
        // after `click`, as Chromium does (review-v91, S1)
        crate::text_pointer::released(document, MOUSE);
    }
}

/// `dblclick` at `clicked`, when the press now is the second in a row.
pub(crate) fn double_click(document: &Rc<RefCell<Document>>, clicked: NodeId, point: Point) {
    let second = document
        .borrow()
        .pointer
        .mouse_press
        .0
        .is_some_and(|clicks| clicks.get() == 2);
    if !second {
        return;
    }
    let event = document
        .borrow()
        .pointer_event("dblclick", MOUSE, clicked, point);
    dispatch_bubbling(document, clicked, event);
}
