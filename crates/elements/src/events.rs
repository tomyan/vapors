//! Pointer input into the tree: hit testing against the last painted
//! update, `:hover` and `:active`, and DOM-style dispatch of
//! `pointerdown`, `pointerup`, `pointermove` and `click` from the target
//! up through its ancestors until a listener stops propagation. The
//! document is not borrowed while a listener runs, so a listener may
//! write signals whose effects change the tree.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Event, NodeId};

use crate::document::Document;

/// A point in CSS pixels from the viewport's top-left.
pub type Point = [f32; 2];

/// Where the pointer is pressed and what it hovers.
#[derive(Default)]
pub(crate) struct Pointer {
    pub(crate) pressed: Option<NodeId>,
    pub(crate) hovered: Option<NodeId>,
    /// Where the pointer last moved, until it leaves the window.
    pub(crate) position: Option<Point>,
}

impl Document {
    /// The pointer moved to `point`: `:hover` follows it, and its target
    /// gets `pointermove`.
    pub fn pointer_moved(document: &Rc<RefCell<Document>>, point: Point) {
        let target = document.borrow_mut().hover(point);
        if let Some(target) = target {
            dispatch(document, target, Event::new("pointermove"));
        }
    }

    /// The pointer left the window: nothing is hovered, and a press in
    /// progress is abandoned (`:active` clears, no `click` follows),
    /// since without pointer capture its release is never seen.
    pub fn pointer_left(document: &Rc<RefCell<Document>>) {
        let mut doc = document.borrow_mut();
        doc.set_flag_chain(States::HOVER, None);
        doc.set_flag_chain(States::ACTIVE, None);
        doc.pointer = Pointer::default();
    }

    /// The primary button went down at `point`: the target and its
    /// ancestors become `:active`, and the target gets `pointerdown`.
    pub fn pointer_pressed(document: &Rc<RefCell<Document>>, point: Point) {
        let target = document.borrow_mut().press(point);
        if let Some(target) = target {
            dispatch(document, target, Event::new("pointerdown"));
        }
    }

    /// The primary button came up at `point`: the target gets
    /// `pointerup`, `:active` clears, and `click` goes to the nearest
    /// element holding both the pressed and the released target, as in
    /// the DOM, unless a disabled button holds it.
    pub fn pointer_released(document: &Rc<RefCell<Document>>, point: Point) {
        let (target, click) = document.borrow_mut().release(point);
        if let Some(target) = target {
            dispatch(document, target, Event::new("pointerup"));
        }
        if let Some(click) = click {
            dispatch(document, click, Event::new("click"));
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
    /// before the first, or after a refused one.
    #[must_use]
    pub fn hit(&self, point: Point) -> Option<NodeId> {
        self.painted.as_ref()?.hit(point)
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

    fn press(&mut self, point: Point) -> Option<NodeId> {
        let target = self.hit(point);
        self.set_flag_chain(States::ACTIVE, target);
        self.pointer.pressed = target;
        target
    }

    fn release(&mut self, point: Point) -> (Option<NodeId>, Option<NodeId>) {
        let target = self.hit(point);
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
        (target, click.filter(|node| !self.in_disabled_button(*node)))
    }
}

#[path = "events_tree.rs"]
mod tree;

use tree::dispatch;
