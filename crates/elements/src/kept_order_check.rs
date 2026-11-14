//! The debug build's check on kept paint order (`kept_order.rs`), part
//! of `kept_paint_check.rs`: the kept steps, clip tree, clips and scroll
//! chains against ones made afresh, each naming its first differing
//! step or slot; and the kept labels, which must increase along the
//! steps.

use crate::kept_order::KeptOrder;
use crate::paint_order::Step;

/// The parts of `kept` that differ from `fresh`'s, each with its first
/// difference; the walk counts are not compared.
pub(crate) fn differing(kept: &KeptOrder, fresh: &KeptOrder) -> Vec<String> {
    let mut parts = Vec::new();
    parts.extend(first_differing_step(&kept.steps, &fresh.steps));
    parts.extend(labels_out_of_order(kept));
    parts.extend(
        kept.tree
            .first_differing(&fresh.tree)
            .map(|slot| format!("clip tree (containing blocks) at slot {slot}")),
    );
    parts.extend(kept.clips.first_differing(&fresh.clips));
    if kept.chains != fresh.chains {
        parts.push("scroll chains".into());
    }
    parts
}

/// `kept`'s first two steps whose labels do not strictly increase, or a
/// label count that is not the step count (`paint_labels.rs`); the
/// labels are compared with no fresh order's, whose labels are a walk's.
fn labels_out_of_order(kept: &KeptOrder) -> Option<String> {
    if kept.labels.len() != kept.steps.len() {
        return Some(format!(
            "labels: {} for {} steps",
            kept.labels.len(),
            kept.steps.len()
        ));
    }
    let at = kept.labels.windows(2).position(|pair| pair[0] >= pair[1])?;
    Some(format!(
        "labels out of order at step {at}: {:?} labelled {}, then {:?} labelled {}",
        kept.steps[at],
        kept.labels[at],
        kept.steps[at + 1],
        kept.labels[at + 1]
    ))
}

/// The first step at which `kept` and `fresh` differ, with both.
fn first_differing_step(kept: &[Step], fresh: &[Step]) -> Option<String> {
    let at = (0..kept.len().max(fresh.len())).find(|&at| kept.get(at) != fresh.get(at))?;
    Some(format!(
        "steps at {at}: kept {:?}, fresh {:?}",
        kept.get(at),
        fresh.get(at)
    ))
}
