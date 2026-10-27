//! A range input's value and keys (3-E3a): `Document::value` and
//! `value_as_number`; arrow keys step the value, Shift with them by
//! ten steps (vapors' own: Chromium steps by one), Page keys by a tenth
//! of the range, Home and End to the ends, each with `input` and
//! `change`. The pointer's drag is `range_drag`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId, PropertyValue};

use crate::document::Document;
use crate::events::Point;
use crate::events::tree::dispatch_bubbling;
use crate::keys::Modifiers;
use crate::range::{Range, written};

/// Chromium's steps for Shift with an arrow key, vapors' own default
/// action (`2026-10-27-direct-tools-first`, 3).
const COARSE_STEPS: f64 = 10.0;

impl Document {
    /// A control's value as the DOM's `value` string: a range's number
    /// written, a number field's text when it is a number (else empty);
    /// `None` for an element that is not a control with a value.
    #[must_use]
    pub fn value(&self, node: NodeId) -> Option<String> {
        self.range(node)
            .map(|range| written(range.value))
            .or_else(|| self.number_field(node).map(|field| field.value()))
    }

    /// A range's value as a number, or a number field's (NaN when its
    /// text is no number, as the DOM's `valueAsNumber`); `None` for
    /// another element.
    #[must_use]
    pub fn value_as_number(&self, node: NodeId) -> Option<f64> {
        self.range(node).map(|range| range.value).or_else(|| {
            self.number_field(node)
                .map(|field| field.number().unwrap_or(f64::NAN))
        })
    }

    /// The range `node` is, when it is an `input type=range`.
    pub(crate) fn range(&self, node: NodeId) -> Option<Range> {
        crate::range::of_element(&self.arena, node)
    }

    /// Sets the value, snapped; whether it changed.
    pub(crate) fn set_range_value(&mut self, node: NodeId, value: f64) -> bool {
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
    pub(crate) fn range_value_at(&self, node: NodeId, point: Point) -> Option<f64> {
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
    pub(crate) fn live_range(&self, node: NodeId) -> bool {
        self.range(node).is_some() && !self.is_disabled_control(node)
    }
}

/// `key` went down on `node`: a live range steps its value, with
/// `input` and `change` when it moved.
pub(crate) fn key(document: &Rc<RefCell<Document>>, node: NodeId, key: &str, modifiers: Modifiers) {
    let doc = document.borrow();
    let (Some(range), true) = (doc.range(node), doc.live_range(node)) else {
        return;
    };
    drop(doc);
    let steps = if modifiers.shift { COARSE_STEPS } else { 1.0 };
    let Some(value) = stepped(&range, key, steps) else {
        return;
    };
    if document.borrow_mut().set_range_value(node, value) {
        fire(document, node, "input");
        fire(document, node, "change");
    }
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
pub(crate) fn fire(document: &Rc<RefCell<Document>>, node: NodeId, name: &str) {
    let Some(value) = document.borrow().value_as_number(node) else {
        return;
    };
    let event = Event::new(name)
        .with_value(&written(value))
        .with_value_number(value);
    dispatch_bubbling(document, node, event);
}
