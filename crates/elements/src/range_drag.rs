//! A range input's drag (3-E3a): a press anywhere on it sets the value
//! from the point and starts a drag that the input captures, each move
//! sets the value, and the release ends it, as Chromium's slider thumb
//! drags. `input` fires on every change and `change` when the drag
//! ends with another value than it began with.
//!
//! A finger decides at its first move whether it slides or scrolls, as
//! Chromium's `SliderContainerElement` does: a move more along the
//! range than across it makes the range own the touch (no pan takes
//! it); one more across leaves the touch to the pan, and the range
//! takes no more of its moves. Each pointer drags on its own.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::events::{Asked, Point};
use crate::range_input::fire;

/// A drag of a range by one pointer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Drag {
    pub(crate) node: NodeId,
    /// The value when it began, for `change` at its end.
    before: f64,
    direction: Direction,
}

/// Which way a drag goes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    /// Along the range: every move sets the value (the mouse always).
    Along,
    /// A finger that has not moved yet, from where it went down.
    Undecided(Point),
    /// A finger that moved across the range: a pan's, no longer the
    /// range's.
    Across,
}

impl Document {
    /// Makes pointer `id`'s drag the range's own: a finger so owned is
    /// not taken by a pan (`touch_panned`).
    fn own_pointer(&mut self, id: u32) {
        self.pointer.captures.entry(id).or_default().explicit = true;
    }
}

/// Pointer `id` pressed `node` at `point`: a live range takes the
/// value there, captures the pointer and begins a drag; the mouse's
/// goes along the range from the start, a finger's is decided at its
/// first move.
pub(crate) fn pressed(document: &Rc<RefCell<Document>>, id: u32, node: NodeId, point: Point) {
    if !document.borrow().live_range(node) {
        return;
    }
    let before = document.borrow().value_as_number(node).unwrap_or(0.0);
    let direction = if id == crate::events::capture::MOUSE {
        Direction::Along
    } else {
        Direction::Undecided(point)
    };
    let drag = Drag {
        node,
        before,
        direction,
    };
    document.borrow_mut().pointer.drags.insert(id, drag);
    crate::events::capture::ask_unless_asked(document, id, Asked::CaptureTo(node));
    set_from_point(document, node, point);
}

/// Pointer `id` moved to `point`: the range it drags takes the value
/// there, unless the drag went across.
pub(crate) fn moved(document: &Rc<RefCell<Document>>, id: u32, point: Point) {
    let Some(drag) = document.borrow().pointer.drags.get(&id).copied() else {
        return;
    };
    let direction = decided(drag.direction, point);
    if direction != drag.direction {
        let mut doc = document.borrow_mut();
        if direction == Direction::Along {
            doc.own_pointer(id);
        }
        if let Some(drag) = doc.pointer.drags.get_mut(&id) {
            drag.direction = direction;
        }
    }
    if direction != Direction::Across {
        set_from_point(document, drag.node, point);
    }
}

/// A finger's direction once it has moved to `point`: along the range
/// when it moved further across the page than down it, across when
/// further down, undecided while equal or unmoved, as Chromium's
/// `GetDirection` has it.
fn decided(direction: Direction, point: Point) -> Direction {
    let Direction::Undecided(from) = direction else {
        return direction;
    };
    let (dx, dy) = ((point[0] - from[0]).abs(), (point[1] - from[1]).abs());
    if dx > dy {
        Direction::Along
    } else if dy > dx {
        Direction::Across
    } else {
        direction
    }
}

/// Pointer `id` came up: its drag ends, with `change` when the value
/// is not the one it began with; for the mouse before the capture's
/// `lostpointercapture`, for a finger after it, as Chromium orders
/// them.
pub(crate) fn released(document: &Rc<RefCell<Document>>, id: u32) {
    let Some(drag) = document.borrow_mut().pointer.drags.remove(&id) else {
        return;
    };
    let now = document.borrow().value_as_number(drag.node);
    if document.borrow().contains(drag.node) && now != Some(drag.before) {
        fire(document, drag.node, "change");
    }
}

fn set_from_point(document: &Rc<RefCell<Document>>, node: NodeId, point: Point) {
    let Some(value) = document.borrow().range_value_at(node, point) else {
        return;
    };
    if document.borrow_mut().set_range_value(node, value) {
        fire(document, node, "input");
    }
}
