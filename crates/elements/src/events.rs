//! Pointer input into the tree: hit testing against the last painted
//! update, `:hover` and `:active`, and DOM-style dispatch of
//! `pointerdown`, `pointerup`, `pointermove` and `click` from the target
//! up through its ancestors until a listener stops propagation. The
//! document is not borrowed while a listener runs, so a listener may
//! write signals whose effects change the tree.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::NodeId;

use crate::document::Document;

/// A point in CSS pixels from the viewport's top-left.
pub type Point = [f32; 2];

/// A change of capture a listener, or `pointerup`, asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Asked {
    /// To this element.
    CaptureTo(NodeId),
    /// To no element.
    Release,
}

/// Where the pointer is pressed and what it hovers.
#[derive(Default)]
pub(crate) struct Pointer {
    pub(crate) pressed: Option<NodeId>,
    pub(crate) hovered: Option<NodeId>,
    /// The fingers down now, with where each last was (3-E1b).
    pub(crate) touches: Vec<(u64, Point)>,
    /// Each pointer's capture, by pointer id (the mouse's is `MOUSE`).
    pub(crate) captures: std::collections::HashMap<u32, capture::Captured>,
    /// Where the pointer last moved, until it leaves the window.
    pub(crate) position: Option<Point>,
    /// Each pointer's drag of a range, by pointer id (3-E3a).
    pub(crate) drags: std::collections::HashMap<u32, crate::range_drag::Drag>,
}

impl Document {
    /// The pointer moved to `point`: `:hover` follows it, and its target
    /// gets `pointermove`; while captured, the capturing element does.
    pub fn pointer_moved(document: &Rc<RefCell<Document>>, point: Point) {
        process_pending(document, MOUSE, point);
        document.borrow_mut().pointer.position = Some(point);
        let target = match hover_capturing(document) {
            Some(capturing) => Some(capturing),
            None => document.borrow_mut().hover(point),
        };
        if let Some(target) = target {
            let event = document
                .borrow()
                .pointer_event("pointermove", MOUSE, target, point);
            dispatch_pointer(document, MOUSE, target, event);
        }
        crate::range_drag::moved(document, MOUSE, point);
    }

    /// The pointer left the window: nothing is hovered, and a press in
    /// progress is abandoned (`:active` clears, no `click` follows),
    /// unless the pointer is captured, when its release is still seen.
    pub fn pointer_left(document: &Rc<RefCell<Document>>) {
        let mut doc = document.borrow_mut();
        doc.set_flag_chain(States::HOVER, None);
        doc.pointer.hovered = None;
        doc.pointer.position = None;
        if doc.captured(MOUSE).capture.is_none() {
            doc.set_flag_chain(States::ACTIVE, None);
            doc.pointer.pressed = None;
        }
    }

    /// The primary button went down at `point`: the target and its
    /// ancestors become `:active`, and the target gets `pointerdown`.
    pub fn pointer_pressed(document: &Rc<RefCell<Document>>, point: Point) {
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
        // a range takes the value at the press before focus moves, as
        // Chromium orders `input` before `focusin` (3-E3a)
        if let (false, Some(target)) = (prevented, target) {
            crate::range_drag::pressed(document, MOUSE, target, point);
        }
        // a press focuses what it is on, or blurs, unless prevented (3-E2a)
        if !prevented {
            let focusable = target.and_then(|target| document.borrow().focusable_holding(target));
            Self::focus(document, focusable);
        }
    }

    /// The primary button came up at `point`: the target (the capturing
    /// element, while captured) gets `pointerup`, a dragged range's
    /// `change` fires, the capture ends, `:active` clears, and `click`
    /// goes to the nearest element holding both the pressed and the
    /// released target, as in the DOM, unless a disabled control holds
    /// it.
    pub fn pointer_released(document: &Rc<RefCell<Document>>, point: Point) {
        process_pending(document, MOUSE, point);
        let target = document.borrow().pointer_target(MOUSE, point);
        if let Some(target) = target {
            let event = document
                .borrow()
                .pointer_event("pointerup", MOUSE, target, point);
            dispatch_pointer(document, MOUSE, target, event);
        }
        crate::range_drag::released(document, MOUSE);
        release_capture(document, MOUSE);
        process_pending(document, MOUSE, point);
        let click = document.borrow_mut().release(target);
        if let Some(click) = click {
            let event = document
                .borrow()
                .pointer_event("click", MOUSE, click, point);
            crate::events::tree::dispatch_click(document, click, event);
        }
    }

    /// A tap at `point`: a press and a release there, which is a click.
    pub fn tapped(document: &Rc<RefCell<Document>>, point: Point) {
        Self::pointer_pressed(document, point);
        Self::pointer_released(document, point);
    }

    /// The cursor over `point` in the last painted update: the hit
    /// element's, or the default arrow over nothing or before the first.
    #[must_use]
    pub fn cursor(&self, point: Point) -> crate::Cursor {
        self.painted
            .as_ref()
            .map_or(crate::Cursor::Default, |painted| painted.cursor(point))
    }

    /// The element `point` hits in the last painted update, or `None`
    /// before the first, after a refused one, or where the element hit
    /// has been removed since (review-v77, H1).
    #[must_use]
    pub fn hit(&self, point: Point) -> Option<NodeId> {
        self.painted
            .as_ref()?
            .hit(point)
            .filter(|node| self.contains(*node))
    }

    /// After an update: the element under the pointer's last position may
    /// have changed with the layout, so `:hover` follows it (the next
    /// update shows it) without an event.
    pub(crate) fn rehover_in(&mut self, painted: &crate::painted::Painted) {
        let Some(point) = self.pointer.position else {
            return;
        };
        let target = painted.hit(point);
        if target != self.pointer.hovered {
            self.set_flag_chain(States::HOVER, target);
            self.pointer.hovered = target;
        }
    }

    fn hover(&mut self, point: Point) -> Option<NodeId> {
        self.pointer.position = Some(point);
        let target = self.hit(point);
        if target != self.pointer.hovered {
            self.set_flag_chain(States::HOVER, target);
            self.pointer.hovered = target;
        }
        target
    }

    /// `target`, the capturing element while captured (review-v77, M3),
    /// becomes the pressed element and `:active` with its ancestors.
    fn press(&mut self, target: Option<NodeId>) -> Option<NodeId> {
        self.set_flag_chain(States::ACTIVE, target);
        self.pointer.pressed = target;
        target
    }

    fn release(&mut self, target: Option<NodeId>) -> Option<NodeId> {
        self.set_flag_chain(States::ACTIVE, None);
        let pressed = self
            .pointer
            .pressed
            .take()
            .filter(|node| self.contains(*node));
        let click = match (pressed, target) {
            (Some(pressed), Some(target)) => self.common_ancestor(pressed, target),
            _ => None,
        };
        click.filter(|node| !self.in_disabled_button(*node))
    }
}

#[path = "events_capture.rs"]
pub(crate) mod capture;
#[path = "events_touch.rs"]
mod touch;
#[path = "events_tree.rs"]
pub(crate) mod tree;

use capture::{MOUSE, dispatch_pointer, hover_capturing, process_pending, release_capture};
