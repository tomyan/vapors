//! A range input's default actions (3-E3a): a press anywhere on it
//! sets the value from the point and starts a drag that the input
//! captures, each move sets the value, and the release ends it, as
//! Chromium's slider thumb drags; arrow keys step the value, Shift
//! with them by ten steps (vapors' own: Chromium steps by one), Page
//! keys by a tenth of the range, Home and End to the ends. `input`
//! fires on every change and `change` when a drag ends with another
//! value than it began with, or with each key.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId, PropertyValue};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;
use crate::events::{Asked, Point};
use crate::keys::Modifiers;
use crate::range::{Range, written};

/// Chromium's steps for Shift with an arrow key, vapors' own default
/// action (`2026-10-27-direct-tools-first`, 3).
const COARSE_STEPS: f64 = 10.0;

/// A drag of a range in progress.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scrub {
    pub(crate) node: NodeId,
    /// The pointer dragging it.
    pub(crate) id: u32,
    /// The value when it began, for `change` at its end.
    before: f64,
}

impl Document {
    /// A control's value as the DOM's `value` string: a range's number
    /// written; `None` for an element that is not a control with a
    /// value.
    #[must_use]
    pub fn value(&self, node: NodeId) -> Option<String> {
        self.range(node).map(|range| written(range.value))
    }

    /// A range's value as a number; `None` for another element.
    #[must_use]
    pub fn value_as_number(&self, node: NodeId) -> Option<f64> {
        self.range(node).map(|range| range.value)
    }

    /// The range `node` is, when it is an `input type=range`.
    pub(crate) fn range(&self, node: NodeId) -> Option<Range> {
        crate::range::of_element(&self.arena, node)
    }

    /// Sets the value, snapped; whether it changed.
    fn set_range_value(&mut self, node: NodeId, value: f64) -> bool {
        let Some(range) = self.range(node) else {
            return false;
        };
        let snapped = range.snapped(value);
        // both are snapped values, so equal means the same step
        #[allow(clippy::float_cmp)]
        if snapped == range.value {
            return false;
        }
        self.changed = true;
        if let Some(element) = self.arena.get_mut(node, "range").element_mut() {
            element
                .properties
                .insert("value".to_owned(), PropertyValue::Number(snapped));
        }
        true
    }

    /// The value at `point` on `node`'s track: the thumb's centre runs
    /// from half a thumb in at either end, as Chromium maps a press.
    fn range_value_at(&self, node: NodeId, point: Point) -> Option<f64> {
        let track = self.painted.as_ref()?.ranges.get(&node)?;
        let span = (track.width - track.thumb).max(0.0);
        let fraction = if span > 0.0 {
            (point[0] - track.x - track.thumb / 2.0) / span
        } else {
            0.0
        };
        Some(self.range(node)?.at_fraction(f64::from(fraction)))
    }

    /// A range the user can change: not disabled.
    fn live_range(&self, node: NodeId) -> bool {
        self.range(node).is_some() && !self.is_disabled_control(node)
    }
}

/// Pointer `id` pressed `node` at `point`: a live range takes the
/// value there, captures the pointer and begins a drag. A finger is
/// owned by the range while it drags, so no pan scrolls instead
/// (`Document::touch_panned`'s rule, as a listener's capture owns one).
pub(crate) fn pressed(document: &Rc<RefCell<Document>>, id: u32, node: NodeId, point: Point) {
    if !document.borrow().live_range(node) {
        return;
    }
    let before = document.borrow().value_as_number(node).unwrap_or(0.0);
    document.borrow_mut().pointer.scrub = Some(Scrub { node, id, before });
    crate::events::capture::ask_unless_asked(document, id, Asked::CaptureTo(node));
    document
        .borrow_mut()
        .pointer
        .captures
        .entry(id)
        .or_default()
        .explicit = true;
    set_from_point(document, node, point);
}

/// Pointer `id` moved to `point`: the range it drags takes the value
/// there.
pub(crate) fn moved(document: &Rc<RefCell<Document>>, id: u32, point: Point) {
    let scrub = document.borrow().pointer.scrub;
    if let Some(scrub) = scrub.filter(|scrub| scrub.id == id) {
        set_from_point(document, scrub.node, point);
    }
}

/// Pointer `id` came up or was cancelled: its drag ends, with `change`
/// when the value is not the one it began with, before the capture's
/// `lostpointercapture`, as Chromium orders them.
pub(crate) fn released(document: &Rc<RefCell<Document>>, id: u32) {
    let scrub = document.borrow().pointer.scrub;
    let Some(scrub) = scrub.filter(|scrub| scrub.id == id) else {
        return;
    };
    document.borrow_mut().pointer.scrub = None;
    let now = document.borrow().value_as_number(scrub.node);
    if document.borrow().contains(scrub.node) && now != Some(scrub.before) {
        fire(document, scrub.node, "change");
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

/// `key` went down on `node`, a live range: whether it stepped the
/// value (`input` and `change` then fire).
pub(crate) fn key(
    document: &Rc<RefCell<Document>>,
    node: NodeId,
    key: &str,
    modifiers: Modifiers,
) -> bool {
    let doc = document.borrow();
    let (Some(range), true) = (doc.range(node), doc.live_range(node)) else {
        return false;
    };
    drop(doc);
    let steps = if modifiers.shift { COARSE_STEPS } else { 1.0 };
    let Some(value) = stepped(&range, key, steps) else {
        return false;
    };
    if document.borrow_mut().set_range_value(node, value) {
        fire(document, node, "input");
        fire(document, node, "change");
    }
    true
}

/// The value `key` moves `range` to, or `None` for a key that does not.
fn stepped(range: &Range, key: &str, steps: f64) -> Option<f64> {
    Some(match key {
        "ArrowRight" | "ArrowUp" => range.stepped(steps),
        "ArrowLeft" | "ArrowDown" => range.stepped(-steps),
        "PageUp" => range.paged(true),
        "PageDown" => range.paged(false),
        "Home" => range.min,
        "End" => range.max,
        _ => return None,
    })
}

/// `input` or `change` on `node`, carrying its value as text and as a
/// number.
fn fire(document: &Rc<RefCell<Document>>, node: NodeId, name: &str) {
    let Some(value) = document.borrow().value_as_number(node) else {
        return;
    };
    let event = Event::new(name)
        .with_value(&written(value))
        .with_value_number(value);
    dispatch_bubbling(document, node, event);
}
