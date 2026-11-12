//! Paint that keeps the last paint in place (IU3 3c-3d2,
//! `docs/plans/iu3.md` "The scene in stable slots"): the last update's
//! `Painted` is taken, the changed slots' parts are dropped and painted
//! again, and it is returned. Which slots: those built again, and those
//! moved or whose clip was redone whose part was made from other values
//! (`BoxFrom`) and cannot be shifted to them (`kept_shift.rs`); no
//! other kept part is read. A dropped part frees its
//! slab ids and removes its map entries before any part is painted, so
//! a part painted after takes them. Scroll thumbs are painted on every
//! update that paints. The scene is the parts of the steps that draw
//! walked in paint order (`scene_view.rs`, `drawn_steps.rs`); it
//! equals a full paint's, which a debug build checks (`kept_paint.rs`).

use crate::changed_slots::ChangedSlots;
use crate::drawn_steps::DrawnSteps;
use crate::kept_box::{BoxFrom, KeptBox};
use crate::kept_hits::HitsChanged;
use crate::kept_order::KeptOrder;
use crate::kept_shift::Shift;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, started};
use crate::paint_order::Step;
use crate::painted::Painted;
use crate::repaint::Repaint;

/// Paints `boxes` at `rects`, keeping `last`'s parts in place where
/// they show what painting again would, and patching its maps for the
/// `changed` slots; a new paint of every box when `last` is `None` or
/// every box changed.
pub(crate) fn paint_partial(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    last: Option<Painted>,
    changed: &ChangedSlots,
) -> Painted {
    let started = started(input);
    let (mut painted, kept_order) = match last {
        Some(last) if !changed.every => restarted(last, &started),
        _ => (started, None),
    };
    let kept = kept_order.is_some();
    let order = KeptOrder::updated(kept_order, (boxes, rects), input, changed);
    let scale = input.scale;
    let (again, shifts) = to_paint(&order, (boxes, rects), (changed, kept), &mut painted, scale);
    dropped(&again, changed, boxes.len(), &mut painted);
    let shifted: Vec<usize> = shifts.iter().map(|shift| shift.slot).collect();
    crate::kept_shift::apply(&mut painted, shifts, scale);
    crate::kept_maps::fill(&mut painted, (boxes, rects), kept, changed);
    let (patched, with_hits) = patched_slots((&again, &shifted), changed, &order);
    let mut repaint = Repaint::new(boxes, rects, &order);
    repaint.boxes(again, input, &mut painted);
    repaint.thumbs(input, &mut painted);
    drawn_updated(&mut painted, &order, (&patched, with_hits));
    painted.order = order;
    painted
}

/// The slots whose parts this update paints again, shifts or drops:
/// those painted `again`, the `shifted` and the freed, then the scroll
/// containers, whose thumbs add no hits; and how many come before the
/// scroll containers.
fn patched_slots(
    (again, shifted): (&[(usize, BoxFrom)], &[usize]),
    changed: &ChangedSlots,
    order: &KeptOrder,
) -> (Vec<usize>, usize) {
    let again = again.iter().map(|(slot, _)| *slot);
    let freed = changed.freed.iter().map(|(slot, _)| *slot);
    let mut slots: Vec<usize> = again.chain(shifted.iter().copied()).chain(freed).collect();
    let with_hits = slots.len();
    slots.extend(order.thumbs.iter().copied());
    (slots, with_hits)
}

/// `painted`'s drawing steps made afresh, and its kept batches and hits
/// dropped, when `order` was walked again; else all three patched for
/// the `patched` slots, the hits for the first `with_hits` only, noting
/// the hits that changed.
fn drawn_updated(
    painted: &mut Painted,
    order: &KeptOrder,
    (patched, with_hits): (&[usize], usize),
) {
    if order.walks.order > 0 {
        painted.drawn = DrawnSteps::of(&order.steps, &painted.kept_boxes);
        painted.kept_batches = std::cell::RefCell::default();
        painted.hits = std::cell::OnceCell::new();
        painted.hits_changed = HitsChanged::Unknown;
        return;
    }
    painted
        .drawn
        .patch(patched.iter().copied(), &painted.kept_boxes);
    painted.hits_changed = match painted.hits.get_mut() {
        Some(hits) => {
            let slots = patched[..with_hits].iter().copied();
            HitsChanged::Known(hits.patch(slots, &painted.drawn, &painted.kept_boxes))
        }
        // Only a release build reaches this arm: the debug check gathers
        // the hits after every paint (`kept_paint_check.rs`). Unknown is
        // the safe answer either way; a gather is one hit test's cost.
        None => HitsChanged::Unknown,
    };
    batches_patched(painted, patched);
}

/// `painted`'s kept batches, if any, patched for the `patched` slots'
/// steps.
fn batches_patched(painted: &mut Painted, patched: &[usize]) {
    if let Some(kept) = painted.kept_batches.get_mut() {
        let steps = patched
            .iter()
            .flat_map(|&slot| painted.drawn.places_of(slot));
        kept.patch(steps, &painted.kept_boxes);
    }
}

/// `last` for this update, with `started`'s canvas and scale and its
/// counts cleared; and its paint order, taken.
fn restarted(mut last: Painted, started: &Painted) -> (Painted, Option<KeptOrder>) {
    let order = std::mem::take(&mut last.order);
    (last.canvas, last.canvas_from) = (started.canvas, started.canvas_from);
    last.scale = started.scale;
    (last.boxes_drawn, last.items_built, last.parts_visited_kept) = (0, 0, 0);
    last.boxes_shifted = 0;
    (last, Some(order))
}

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
fn to_paint(
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
    // A checked checkbox's mark is placed from the content box by a
    // fraction of its size, which `snaps_by` does not check.
    let checked = boxes[slot].checked == Some(true);
    let edges = |at| [at, crate::clip::content_box(&boxes[slot], at)];
    let shift = Shift::of(slot, (part, &boxes[slot].content), from, (scale, edges));
    match shift.filter(|_| !checked) {
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
    let changed = changed.rebuilt.iter().chain(&changed.moved);
    let mut slots: Vec<usize> = changed.chain(&order.redone).copied().collect();
    slots.sort_unstable();
    slots.dedup();
    slots
}

/// Drops the parts of the freed slots, of the slots painted `again`,
/// and of any slot past the `len` boxes, freeing their ids and removing
/// their entries.
fn dropped(again: &[(usize, BoxFrom)], changed: &ChangedSlots, len: usize, painted: &mut Painted) {
    let freed = changed.freed.iter().map(|(slot, _)| *slot);
    for slot in freed.chain(again.iter().map(|(slot, _)| *slot)) {
        if let Some(gone) = painted.kept_boxes.get_mut(slot).and_then(Option::take) {
            gone.drop_from(painted);
        }
    }
    let past = len.min(painted.kept_boxes.len());
    let past: Vec<KeptBox> = painted.kept_boxes.drain(past..).flatten().collect();
    for gone in past {
        gone.drop_from(painted);
    }
    painted.kept_boxes.resize_with(len, || None);
}

#[cfg(test)]
#[path = "paint_partial_tests.rs"]
mod tests;
