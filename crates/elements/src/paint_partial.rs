//! Paint that keeps the last paint in place (IU3 3c-3d2,
//! `docs/plans/iu3.md` "The scene in stable slots"): the last update's
//! `Painted` is taken, the changed slots' parts are dropped and painted
//! again, and it is returned. Which slots: those built again, and those
//! moved or whose clip was redone whose part was made from other values
//! (`BoxFrom`) and cannot be shifted to them (`kept_shift.rs`); no
//! other kept part is read. A dropped part frees its slab ids and
//! removes its map entries before any part is painted, so a part
//! painted after takes them. The thumbs painted are those of the scroll
//! containers that can have changed (`thumbs_again.rs`). The scene is
//! the parts of the steps that draw walked in paint order
//! (`scene_view.rs`, `drawn_steps.rs`); it equals a full paint's, which
//! a debug build checks (`kept_paint.rs`).

use crate::blocks::IndexCounts;
use crate::changed_slots::ChangedSlots;
use crate::kept_box::{BoxFrom, KeptBox};
use crate::kept_hits::HitsChanged;
use crate::kept_order::KeptOrder;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, started};
use crate::paint_partial_drawn::{batches_unsorted, drawn_updated, walked};
use crate::paint_partial_read::to_paint;
use crate::painted::Painted;
use crate::repaint::Repaint;
use crate::thumbs_again;

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
    let label_of = &|step| painted.drawn.label_of(step);
    let order = KeptOrder::updated(kept_order, (boxes, rects), input, changed, label_of);
    let scale = input.scale;
    let frames_moved_now =
        crate::paint_frames::updated(&mut painted, &order, (boxes, input), (changed, kept));
    let lanes_moved = !changed.started.is_empty();
    let (again, shifts) = to_paint(&order, (boxes, rects), (changed, kept), &mut painted, scale);
    dropped(&again, changed, boxes.len(), &mut painted);
    let shifted: Vec<usize> = shifts.iter().map(|shift| shift.slot).collect();
    crate::kept_shift::apply(&mut painted, shifts, scale);
    crate::kept_maps::fill(&mut painted, (boxes, rects, &order), kept, changed);
    let shown_last = kept.then_some(painted.from.thumb_alpha);
    let thumbs = thumbs_again::of_update(&order, (&again, &shifted), changed, (shown_last, input));
    let (patched, with_hits) = patched_slots((&again, &shifted, &thumbs), changed);
    let again_slots: Vec<usize> = again.iter().map(|(slot, _)| *slot).collect();
    let mut repaint = Repaint::new(boxes, rects, &order);
    repaint.boxes(again, input, &mut painted);
    let long = crate::clip_chain_frames::long_chains(&order, &mut painted, &again_slots);
    input.long_chains.extend(long);
    repaint.thumbs(&thumbs, input, &mut painted);
    drawn_updated(&mut painted, &order, (&patched, with_hits));
    crate::paint_lane_ink::note_painted(&mut painted, input, (walked(&order), &patched));
    if frames_moved_now {
        frames_moved(&mut painted);
    } else if lanes_moved {
        lane_rows_moved(&mut painted, &changed.freed);
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

/// After only framed lanes' rows moved (started elsewhere): the kept
/// batches searched again in those lanes, at the view's edges, and the
/// hover tested again.
fn lane_rows_moved(painted: &mut Painted, freed: &[(usize, crate::layout::Key)]) {
    crate::paint_partial_drawn::batches_lanes_moved(painted, freed);
    painted.hits_changed = HitsChanged::Unknown;
}

/// The slots whose parts this update paints again, shifts or drops:
/// those painted `again`, the `shifted` and the freed, then the scroll
/// containers whose `thumbs` it paints, which add no hits; and how many
/// come before those.
fn patched_slots(
    (again, shifted, thumbs): (&[(usize, BoxFrom)], &[usize], &[usize]),
    changed: &ChangedSlots,
) -> (Vec<usize>, usize) {
    let again = again.iter().map(|(slot, _)| *slot);
    let freed = changed.freed.iter().map(|(slot, _)| *slot);
    let mut slots: Vec<usize> = again.chain(shifted.iter().copied()).chain(freed).collect();
    let with_hits = slots.len();
    slots.extend(thumbs.iter().copied());
    (slots, with_hits)
}

/// `last` for this update, with `started`'s canvas and scale and its
/// counts cleared; and its paint order, taken.
fn restarted(mut last: Painted, started: &Painted) -> (Painted, Option<KeptOrder>) {
    let order = std::mem::take(&mut last.order);
    (last.canvas, last.canvas_from) = (started.canvas, started.canvas_from);
    last.scale = started.scale;
    (last.boxes_drawn, last.items_built, last.parts_visited_kept) = (0, 0, 0);
    (last.boxes_shifted, last.index_counts) = (0, IndexCounts::default());
    last.splice_patched = false;
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
    let past: Vec<KeptBox> = painted.kept_boxes.drain_from(past).flatten().collect();
    for gone in past {
        gone.drop_from(painted);
    }
    painted.kept_boxes.resize_with(len, || None);
}

#[cfg(test)]
#[path = "paint_partial_tests.rs"]
mod tests;
