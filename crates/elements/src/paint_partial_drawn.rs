//! The drawing steps, the extents index, the hits and the kept batches
//! after paint (`paint_partial.rs`): made afresh after a walk, else
//! following the splice's edits (`kept_order_labelled.rs`) and patched
//! for the slots painted again, shifted or freed.

use crate::drawn_steps::DrawnSteps;
use crate::frame_extents::Extents;
use crate::kept_hits::HitsChanged;
use crate::kept_order::KeptOrder;
use crate::paint_labels::Label;
use crate::paint_order::Step;
use crate::painted::Painted;

/// `painted`'s drawing steps made afresh, its kept batches unsorted and
/// its hits dropped, when `order` was walked again, its splice
/// relabelled every label, or it patched too many slots (`many`); else
/// the three following its splice, if any, and patched for the
/// `patched` slots, the hits for the first `with_hits` only, noting the
/// hits that changed and whether a splice was patched.
pub(crate) fn drawn_updated(
    painted: &mut Painted,
    order: &KeptOrder,
    (patched, with_hits): (&[usize], usize),
) {
    let spliced = order.walks.splices > 0;
    if walked(order) || (spliced && many(patched, painted)) {
        drawn_afresh(painted, order);
        return;
    }
    if spliced {
        spliced_into(painted, &order.splice_edits);
        painted.splice_patched = true;
    }
    painted
        .drawn
        .patch(patched.iter().copied(), &painted.kept_boxes);
    extents_patched(painted, patched);
    painted.hits_changed = hits_patched(painted, &patched[..with_hits], spliced);
    batches_patched(painted, patched);
    let drawn = painted.drawn.take_counts();
    painted.index_counts = drawn.and(painted.extents.take_counts());
}

/// `painted`'s drawing steps and extents index made afresh from
/// `order`, its kept batches unsorted and its hits dropped.
fn drawn_afresh(painted: &mut Painted, order: &KeptOrder) {
    painted.drawn = DrawnSteps::of((&order.steps, &order.labels), &painted.kept_boxes);
    let slots = painted.kept_boxes.len();
    painted.extents = Extents::of(&painted.drawn, slots, &painted.kept_boxes);
    batches_unsorted(painted);
    painted.hits = std::cell::OnceCell::new();
    painted.hits_changed = HitsChanged::Unknown;
}

/// `painted`'s kept hits, if gathered, patched for the `slots`; the hits
/// that changed, every one when a splice took hits out with its rows.
fn hits_patched(painted: &mut Painted, slots: &[usize], spliced: bool) -> HitsChanged {
    let Some(hits) = painted.hits.get_mut() else {
        // Only a release build reaches this arm: the debug check gathers
        // the hits after every paint (`kept_paint_check.rs`). Unknown is
        // the safe answer either way; a gather is one hit test's cost.
        return HitsChanged::Unknown;
    };
    let changed = hits.patch(slots.iter().copied(), &painted.drawn, &painted.kept_boxes);
    if spliced {
        return HitsChanged::Unknown;
    }
    HitsChanged::on_page(changed, &painted.frames)
}

/// Whether a splice that patched the `patched` slots makes the drawing
/// steps, the extents index and the hits afresh: when they are more
/// than `REBUILD` (64, the extents index's floor, so a small page
/// patches) and more than a quarter of `painted`'s slots. Measured on
/// `vapors-perf 8 2 --scaling Insert` (an unframed lane, half its boxes
/// patched per splice) with each path forced (2026-11-15): making the
/// three afresh costs 518 instructions a box, patching 1,932 a patched
/// slot, so they cost the same at about 27% of the slots patched. The
/// switches and spans that measured it: `measure/4d1-threshold-switches`
/// (456e8a44), its commit message giving the commands.
fn many(patched: &[usize], painted: &Painted) -> bool {
    let share = painted.kept_boxes.len() / 4;
    patched.len() > crate::frame_extents::REBUILD.max(share)
}

/// Whether `order` was walked, or its splice relabelled every label:
/// the drawing steps, the extents index and the hits are made afresh.
pub(crate) fn walked(order: &KeptOrder) -> bool {
    order.walks.order > 0 || order.splice_edits.all
}

/// The drawing steps, the extents index and the hits of `painted`
/// following a splice's `edits` (`kept_order_labelled.rs`), its kept
/// batches unsorted: the steps it put in are patched in with their slots.
fn spliced_into(painted: &mut Painted, edits: &crate::kept_order_splice::SpliceEdits) {
    let slots = painted.kept_boxes.len();
    painted.drawn.splice(edits, slots);
    painted.extents.splice(edits, slots);
    if let Some(hits) = painted.hits.get_mut() {
        hits.splice(edits);
    }
    batches_spliced(painted, edits);
}

/// `painted`'s kept batches, if any, following the splice's `edits`
/// (`KeptBatches::spliced`).
fn batches_spliced(painted: &mut Painted, edits: &crate::kept_order_splice::SpliceEdits) {
    let Some(mut kept) = painted.kept_batches.take() else {
        return;
    };
    kept.spliced(edits);
    *painted.kept_batches.get_mut() = Some(kept);
}

/// `painted`'s kept batches, if any, searched again in the framed lanes
/// whose rows moved, when no other frame did (`KeptBatches::lanes_moved`).
pub(crate) fn batches_lanes_moved(painted: &mut Painted, freed: &[(usize, crate::layout::Key)]) {
    let Some(mut kept) = painted.kept_batches.take() else {
        return;
    };
    kept.lanes_moved(painted.batch_sources(), freed);
    *painted.kept_batches.get_mut() = Some(kept);
}

/// `painted`'s extents index patched for the `patched` slots' steps.
fn extents_patched(painted: &mut Painted, patched: &[usize]) {
    let labels_of = |&slot: &usize| painted.drawn.labels_of(slot);
    let steps: Vec<(Label, Step)> = patched.iter().flat_map(labels_of).collect();
    let drawn = (&painted.drawn, painted.kept_boxes.len());
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
        .flat_map(|&slot| painted.drawn.labels_of(slot));
    kept.patch(steps, painted.batch_sources());
    *painted.kept_batches.get_mut() = Some(kept);
}

/// `painted`'s kept batches, if any, to be sorted again at the next
/// draw, keeping their keys for the runs made again.
pub(crate) fn batches_unsorted(painted: &mut Painted) {
    if let Some(kept) = painted.kept_batches.get_mut() {
        kept.unsort();
    }
}
