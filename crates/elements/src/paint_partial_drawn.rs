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
/// its hits dropped, when `order` was walked again or its splice
/// relabelled every label; else the three following its splice, if any,
/// and patched for the `patched` slots, the hits for the first
/// `with_hits` only, noting the hits that changed.
pub(crate) fn drawn_updated(
    painted: &mut Painted,
    order: &KeptOrder,
    (patched, with_hits): (&[usize], usize),
) {
    if walked(order) || (order.walks.splices > 0 && many(patched, painted)) {
        painted.drawn = DrawnSteps::of((&order.steps, &order.labels), &painted.kept_boxes);
        let slots = painted.kept_boxes.len();
        painted.extents = Extents::of(&painted.drawn, slots, &painted.kept_boxes);
        batches_unsorted(painted);
        painted.hits = std::cell::OnceCell::new();
        painted.hits_changed = HitsChanged::Unknown;
        return;
    }
    let spliced = order.walks.splices > 0;
    if spliced {
        spliced_into(painted, &order.splice_edits);
    }
    painted
        .drawn
        .patch(patched.iter().copied(), &painted.kept_boxes);
    extents_patched(painted, patched);
    painted.hits_changed = match painted.hits.get_mut() {
        // a splice took hits out with its rows: any may differ
        Some(hits) if spliced => {
            hits.patch(
                patched[..with_hits].iter().copied(),
                &painted.drawn,
                &painted.kept_boxes,
            );
            HitsChanged::Unknown
        }
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
    let drawn = painted.drawn.take_counts();
    painted.index_counts = drawn.and(painted.extents.take_counts());
}

/// Whether the `patched` slots are more than a sixteenth of `painted`'s:
/// a splice that moved that many (a lane not framed) makes the drawing
/// steps, the extents index and the hits afresh, a pass, rather than
/// patch each, as the extents index does for a patch past `REBUILD`.
fn many(patched: &[usize], painted: &Painted) -> bool {
    patched.len() * 16 > painted.kept_boxes.len()
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
    batches_unsorted(painted);
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
