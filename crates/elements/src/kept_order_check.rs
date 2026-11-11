//! The debug build's check on kept paint order (`kept_order.rs`), part
//! of `kept_paint_check.rs`: the kept steps, clip tree, clips and scroll
//! chains against ones made afresh, each naming its first differing
//! step or slot.

use crate::kept_order::KeptOrder;
use crate::paint_order::Step;

/// The parts of `kept` that differ from `fresh`'s, each with its first
/// difference; the walk counts are not compared.
pub(crate) fn differing(kept: &KeptOrder, fresh: &KeptOrder) -> Vec<String> {
    let mut parts = Vec::new();
    parts.extend(first_differing_step(&kept.steps, &fresh.steps));
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

/// The first step at which `kept` and `fresh` differ, with both.
fn first_differing_step(kept: &[Step], fresh: &[Step]) -> Option<String> {
    let at = (0..kept.len().max(fresh.len())).find(|&at| kept.get(at) != fresh.get(at))?;
    Some(format!(
        "steps at {at}: kept {:?}, fresh {:?}",
        kept.get(at),
        fresh.get(at)
    ))
}
