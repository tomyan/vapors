//! Which changed slots a partial paint paints again, shifts or keeps
//! (`paint_partial.rs`): each rebuilt, moved or reclipped slot's kept
//! part read against what its box reads now.

use crate::changed_slots::ChangedSlots;
use crate::kept_box::BoxFrom;
use crate::kept_order::KeptOrder;
use crate::kept_shift::Shift;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint_order::Step;
use crate::painted::Painted;

/// What reading a changed slot's kept part found.
enum Read {
    /// Made from what the box reads now: kept in place.
    Kept,
    /// Made from a rect the box moved from: shifted to the new one.
    Shifted(Shift),
    /// Made from other values, or none kept: painted again from these.
    Again(BoxFrom),
}

/// The slots to paint again, each with what it reads now, and the
/// shifts of those to shift: every box step's when nothing is kept,
/// else each changed or reclipped slot whose box was built again or
/// whose part was made from other values. Notes how many kept parts
/// that read and kept.
pub(crate) fn to_paint(
    order: &KeptOrder,
    (boxes, rects): (&[LayoutBox], &[BoxRect]),
    (changed, kept): (&ChangedSlots, bool),
    painted: &mut Painted,
    scale: f32,
) -> (Vec<(usize, BoxFrom)>, Vec<Shift>) {
    let slots = candidates(order, changed, !kept);
    let shown = slots
        .into_iter()
        .filter(|&slot| !matches!(boxes[slot].content, Content::Vacant));
    let (mut again, mut shifts) = (Vec::new(), Vec::new());
    for slot in shown {
        let from = BoxFrom::of(boxes, rects, order.clips.own(slot), slot);
        match read(slot, from, boxes, painted, scale) {
            Read::Kept => painted.parts_visited_kept += 1,
            Read::Shifted(shift) => shifts.push(shift),
            Read::Again(from) => again.push((slot, from)),
        }
    }
    (again, shifts)
}

/// Whether slot `slot`'s kept part, if its box was kept, shows `from`,
/// shifts to it, or is painted again.
fn read(slot: usize, from: BoxFrom, boxes: &[LayoutBox], painted: &Painted, scale: f32) -> Read {
    let part = painted.kept_boxes.get(slot).and_then(Option::as_ref);
    let Some(part) = part.filter(|_| boxes[slot].kept) else {
        return Read::Again(from);
    };
    if part.from == from {
        return Read::Kept;
    }
    match Shift::of_box(slot, part, &boxes[slot], from, scale) {
        Some(shift) => Read::Shifted(shift),
        None => Read::Again(from),
    }
}

/// Every box step's slot when `fresh`, else the rebuilt, moved and
/// reclipped slots, each once.
fn candidates(order: &KeptOrder, changed: &ChangedSlots, fresh: bool) -> Vec<usize> {
    if fresh {
        let own = |step: &Step| matches!(step, Step::Box(_)).then(|| step.index());
        return order.steps.iter().filter_map(own).collect();
    }
    let moved = changed.moved.iter().chain(&changed.scrolled);
    let changed = changed.rebuilt.iter().chain(moved);
    let mut slots: Vec<usize> = changed.chain(&order.redone).copied().collect();
    slots.sort_unstable();
    slots.dedup();
    slots
}
