//! A number field's scrub (3-E3c): with `data-scrub`, a drag along the
//! field steps its number, one `step` per [`DEFAULT_PX_PER_STEP`] CSS
//! px of movement, or per the px `data-scrub="<px>"` names. The press
//! still focuses the field; the first step clears its selection and
//! fires `input`, and the release fires `change` when the number moved.
//! A finger decides at its first move whether it scrubs or scrolls, as
//! a range's does (`range_drag::Direction`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::events::{Asked, Point};
use crate::range::written;
use crate::range_drag::Direction;

/// How far the pointer moves for one step, CSS px, without a number
/// in `data-scrub`.
pub(crate) const DEFAULT_PX_PER_STEP: f32 = 2.0;

/// One pointer's scrub of a number field.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scrub {
    pub(crate) node: NodeId,
    /// Where the pointer pressed.
    from: Point,
    /// The number then, which the steps count from.
    base: f64,
    px_per_step: f32,
    direction: Direction,
    /// Whether any step was taken, so the release fires `change`.
    moved: bool,
}

impl Document {
    /// The px per step `node`'s `data-scrub` asks for, when it is a
    /// live number field with the attribute.
    fn scrub_px(&self, node: NodeId) -> Option<f32> {
        self.number_field(node)?;
        if self.is_disabled_control(node) {
            return None;
        }
        let value = self.attribute(node, "data-scrub")?;
        if value.is_empty() {
            return Some(DEFAULT_PX_PER_STEP);
        }
        value.parse::<f32>().ok().filter(|px| *px > 0.0)
    }
}

/// Pointer `id` pressed `node` at `point`: a scrubbable field begins a
/// scrub, capturing the pointer.
pub(crate) fn pressed(document: &Rc<RefCell<Document>>, id: u32, node: NodeId, point: Point) {
    let (Some(px_per_step), Some(field)) = (
        document.borrow().scrub_px(node),
        document.borrow().number_field(node),
    ) else {
        return;
    };
    let base = field.number().unwrap_or_else(|| field.clamped(0.0));
    let direction = if id == crate::events::capture::MOUSE {
        Direction::Along
    } else {
        Direction::Undecided(point)
    };
    let scrub = Scrub {
        node,
        from: point,
        base,
        px_per_step,
        direction,
        moved: false,
    };
    document.borrow_mut().pointer.scrubs.insert(id, scrub);
    crate::events::capture::ask_unless_asked(document, id, Asked::CaptureTo(node));
}

/// Pointer `id` moved to `point`: its scrub steps the number by the
/// distance along the field from the press.
pub(crate) fn moved(document: &Rc<RefCell<Document>>, id: u32, point: Point) {
    let Some(scrub) = document.borrow().pointer.scrubs.get(&id).copied() else {
        return;
    };
    let direction = crate::range_drag::decided(scrub.direction, point);
    {
        let mut doc = document.borrow_mut();
        if direction == Direction::Along && scrub.direction != direction {
            doc.pointer.captures.entry(id).or_default().explicit = true;
        }
        if let Some(scrub) = doc.pointer.scrubs.get_mut(&id) {
            scrub.direction = direction;
        }
    }
    if direction == Direction::Across {
        return;
    }
    let steps = f64::from(((point[0] - scrub.from[0]) / scrub.px_per_step).trunc());
    step_to(document, id, scrub, steps);
}

fn step_to(document: &Rc<RefCell<Document>>, id: u32, scrub: Scrub, steps: f64) {
    let Some(field) = document.borrow().number_field(scrub.node) else {
        return;
    };
    let value = field.clamped(scrub.base + steps * field.step.unwrap_or(1.0));
    let changed = document
        .borrow_mut()
        .set_number_text(scrub.node, written(value));
    if changed {
        if let Some(scrub) = document.borrow_mut().pointer.scrubs.get_mut(&id) {
            scrub.moved = true;
        }
        crate::number_input::fire_input(document, scrub.node);
    }
}

/// Pointer `id` was taken by the platform: its scrub ends with
/// nothing, as a range's drag does.
pub(crate) fn dropped(document: &Rc<RefCell<Document>>, id: u32) {
    document.borrow_mut().pointer.scrubs.remove(&id);
}

/// Pointer `id` came up: its scrub ends, committing the number with
/// `change` when a step was taken.
pub(crate) fn released(document: &Rc<RefCell<Document>>, id: u32) {
    let Some(scrub) = document.borrow_mut().pointer.scrubs.remove(&id) else {
        return;
    };
    if scrub.moved && document.borrow().contains(scrub.node) {
        crate::number_input::commit(document, scrub.node);
    }
}
