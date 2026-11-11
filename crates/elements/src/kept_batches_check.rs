//! The debug build's check on kept batches (`kept_batches.rs`): every
//! draw's batches equal batching the whole scene afresh.

use vapors_scene::Batched;

/// Panics, naming the first differing batch or the culled counts, when
/// `kept` is not `fresh`.
pub(crate) fn assert_same(kept: &Batched, fresh: &Batched) {
    if let Some(difference) = first_difference(kept, fresh) {
        panic!(
            "the kept batches differ from batching the scene afresh: {difference}; a slot \
             painted again was not patched into the kept batches or a change that \
             batching reads was not dropped from them (kept_batches.rs, \
             paint_partial.rs); reproduce with `cargo xtask test`"
        );
    }
}

/// The first batch at which `kept` and `fresh` differ, or their culled
/// counts when only those do.
fn first_difference(kept: &Batched, fresh: &Batched) -> Option<String> {
    let longest = kept.batches.len().max(fresh.batches.len());
    let differing = (0..longest).find(|&at| kept.batches.get(at) != fresh.batches.get(at));
    if let Some(at) = differing {
        let (kept, fresh) = (kept.batches.get(at), fresh.batches.get(at));
        return Some(format!("batch {at} is {kept:?}, afresh {fresh:?}"));
    }
    (kept.culled != fresh.culled)
        .then(|| format!("culled {} items, afresh {}", kept.culled, fresh.culled))
}
