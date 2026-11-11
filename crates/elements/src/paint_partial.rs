//! Paint that keeps the last paint in place (IU3 3c-3d2,
//! `docs/plans/iu3.md` "The scene in stable slots"): the last update's
//! `Painted` is taken, the changed slots' parts are dropped and painted
//! again, and it is returned. Which slots: those built again, and those
//! moved or whose clip was redone whose part was made from other values
//! (`BoxFrom`); no other kept part is read. A dropped part frees its
//! slab ids and removes its map entries before any part is painted, so
//! a part painted after takes them. Scroll thumbs are painted on every
//! update that paints. The scene is the parts walked in paint order
//! (`scene_view.rs`); it equals a full paint's, which a debug build
//! checks (`kept_paint.rs`).

use crate::changed_slots::ChangedSlots;
use crate::kept_box::{BoxFrom, KeptBox};
use crate::kept_order::KeptOrder;
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
    let again = to_paint(&order, (boxes, rects), (changed, kept), &mut painted);
    dropped(&again, changed, boxes.len(), &mut painted);
    crate::kept_maps::fill(&mut painted, (boxes, rects), kept, changed);
    let mut repaint = Repaint::new(boxes, rects, &order);
    repaint.boxes(again, input, &mut painted);
    repaint.thumbs(input, &mut painted);
    painted.order = order;
    painted
}

/// `last` for this update, with `started`'s canvas and scale, its counts and
/// gathered hits cleared; and its paint order, taken.
fn restarted(mut last: Painted, started: &Painted) -> (Painted, Option<KeptOrder>) {
    let order = std::mem::take(&mut last.order);
    (last.canvas, last.canvas_from) = (started.canvas, started.canvas_from);
    last.scale = started.scale;
    (last.boxes_drawn, last.items_built, last.parts_visited_kept) = (0, 0, 0);
    last.hits = std::cell::OnceCell::new();
    (last, Some(order))
}

/// The slots to paint again, each with what it reads now: every box
/// step's when nothing is kept, else each changed or reclipped slot
/// whose box was built again or whose part was made from other values.
/// Notes how many kept parts that read and kept.
fn to_paint(
    order: &KeptOrder,
    (boxes, rects): (&[LayoutBox], &[BoxRect]),
    (changed, kept): (&ChangedSlots, bool),
    painted: &mut Painted,
) -> Vec<(usize, BoxFrom)> {
    let slots = candidates(order, changed, !kept);
    let shown = slots
        .into_iter()
        .filter(|&slot| !matches!(boxes[slot].content, Content::Vacant));
    let read = shown.map(|slot| (slot, BoxFrom::of(boxes, rects, order.clips.own(slot), slot)));
    let kept = |(slot, from): &(usize, BoxFrom)| {
        let part = painted.kept_boxes.get(*slot).and_then(Option::as_ref);
        boxes[*slot].kept && part.is_some_and(|part| part.from == *from)
    };
    let (kept, again): (Vec<_>, Vec<_>) = read.partition(kept);
    painted.parts_visited_kept = kept.len();
    again
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
