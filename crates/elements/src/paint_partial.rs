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
use crate::frame_extents::Extents;
use crate::frames::Frames;
use crate::kept_box::{BoxFrom, KeptBox};
use crate::kept_hits::HitsChanged;
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, started};
use crate::paint_order::Step;
use crate::paint_partial_read::to_paint;
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
    let place = &|step| painted.drawn.place_of(step);
    let order = KeptOrder::updated(kept_order, (boxes, rects), input, changed, place);
    let scale = input.scale;
    let frames_were = std::mem::replace(&mut painted.frames, frames(&order, boxes, input));
    let (again, shifts) = to_paint(&order, (boxes, rects), (changed, kept), &mut painted, scale);
    dropped(&again, changed, boxes.len(), &mut painted);
    let shifted: Vec<usize> = shifts.iter().map(|shift| shift.slot).collect();
    crate::kept_shift::apply(&mut painted, shifts, scale);
    crate::kept_maps::fill(&mut painted, (boxes, rects, &order), kept, changed);
    let (patched, with_hits) = patched_slots((&again, &shifted), changed, &order);
    let mut repaint = Repaint::new(boxes, rects, &order);
    repaint.boxes(again, input, &mut painted);
    repaint.thumbs(input, &mut painted);
    drawn_updated(&mut painted, &order, (&patched, with_hits));
    if painted.frames != frames_were {
        frames_moved(&mut painted);
    }
    painted.order = order;
    painted
}

/// After a frame's shift or clip changed: every item and hit in it is
/// elsewhere on the page, so the kept batches' steps are sorted again
/// at the next draw and the hover is tested again.
fn frames_moved(painted: &mut Painted) {
    batches_unsorted(painted);
    painted.hits_changed = HitsChanged::Unknown;
}

/// `painted`'s kept batches, if any, to be sorted again at the next
/// draw, keeping their keys for the runs made again.
fn batches_unsorted(painted: &mut Painted) {
    if let Some(kept) = painted.kept_batches.get_mut() {
        kept.unsort();
    }
}

/// The scroll frames of `order`'s scroll containers, each shifted by
/// its offset in `input` as `scroll::frame_shift` gives it.
fn frames(order: &KeptOrder, boxes: &[LayoutBox], input: &PaintInput<'_>) -> Frames {
    let scrolls = |slot| input.stackings.scrolls(slot);
    crate::frames_check::assert_scrollers_agree(&order.thumbs, &order.clips, &scrolls);
    let depth = |slot| order.tree.depth(slot);
    let shift =
        |slot| crate::scroll::frame_shift(scroll_offset(boxes, slot, input.scroll), input.scale);
    Frames::of(&order.thumbs, (&order.clips, &depth), &shift, input.scale)
}

/// The scroll offset of the scroll container at `slot`, or a panic
/// naming it: placement settles every scroll container's offset
/// before paint.
fn scroll_offset(boxes: &[LayoutBox], slot: usize, scroll: &crate::scroll::Offsets) -> [f32; 2] {
    let node = boxes[slot].element_in(slot);
    node.and_then(|node| scroll.get(node)).unwrap_or_else(|| {
        panic!(
            "scroll container slot {slot} ({node:?}) has no settled offset: placement \
             settles every scroll container's (placement_walk.rs `settled_offset`); \
             reproduce with `cargo xtask test`"
        )
    })
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

/// `painted`'s drawing steps made afresh, its kept batches unsorted and
/// its hits dropped, when `order` was walked again; else all three patched for
/// the `patched` slots, the hits for the first `with_hits` only, noting
/// the hits that changed.
fn drawn_updated(
    painted: &mut Painted,
    order: &KeptOrder,
    (patched, with_hits): (&[usize], usize),
) {
    // After a splice the drawing steps, hits and batches are made afresh
    // as after a walk, until a follow-up shifts them by the splice's edits.
    if order.walks.order > 0 || order.walks.splices > 0 {
        painted.drawn = DrawnSteps::of(&order.steps, &painted.kept_boxes);
        let places = order.steps.len();
        painted.extents = Extents::of(&painted.drawn, places, &painted.kept_boxes);
        batches_unsorted(painted);
        painted.hits = std::cell::OnceCell::new();
        painted.hits_changed = HitsChanged::Unknown;
        return;
    }
    painted
        .drawn
        .patch(patched.iter().copied(), &painted.kept_boxes);
    extents_patched(painted, order, patched);
    painted.hits_changed = match painted.hits.get_mut() {
        Some(hits) => {
            let slots = patched[..with_hits].iter().copied();
            let changed = hits.patch(slots, &painted.drawn, &painted.kept_boxes);
            HitsChanged::on_page(changed, &painted.frames)
        }
        // Only a release build reaches this arm: the debug check gathers
        // the hits after every paint (`kept_paint_check.rs`). Unknown is
        // the safe answer either way; a gather is one hit test's cost.
        None => HitsChanged::Unknown,
    };
    batches_patched(painted, patched);
}

/// `painted`'s extents index patched for the `patched` slots' steps.
fn extents_patched(painted: &mut Painted, order: &KeptOrder, patched: &[usize]) {
    let places_of = |&slot: &usize| painted.drawn.places_of(slot);
    let steps: Vec<(usize, Step)> = patched.iter().flat_map(places_of).collect();
    let drawn = (&painted.drawn, order.steps.len());
    painted.extents.patch(&steps, drawn, &painted.kept_boxes);
}

/// `painted`'s kept batches, if any, patched for the `patched` slots'
/// steps.
fn batches_patched(painted: &mut Painted, patched: &[usize]) {
    let Some(mut kept) = painted.kept_batches.take() else {
        return;
    };
    let steps = patched
        .iter()
        .flat_map(|&slot| painted.drawn.places_of(slot));
    kept.patch(steps, painted.batch_sources());
    *painted.kept_batches.get_mut() = Some(kept);
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
