//! The debug build's checks on kept batches (`kept_batches.rs`): every
//! draw's batches and their frames equal batching the whole scene
//! afresh in its frames (`framed_batches.rs`); and their items, put on
//! the page by their frames' shifts and clips, are the items batching
//! the scene on the page draws, in the same order, so the frames put
//! every item where the page has it. And the steps the extents index
//! finds showing in a view are those testing every step finds.

use vapors_scene::{Batched, Item, Run};

use crate::framed_batches::{FramedBatched, hidden_items};
use crate::kept_batches::{Shown, Sources};
use crate::kept_batches_keys::KeptBatched;

/// Panics, naming the first differing batch, frame or the culled
/// counts, when `kept` is not `fresh`.
pub(crate) fn assert_same(kept: &KeptBatched, fresh: &FramedBatched) {
    let difference = first_difference(&kept.batched, &fresh.batched).or_else(|| {
        let longest = kept.frames.len().max(fresh.frames.len());
        let differs = |at: &usize| {
            kept.frames.get(*at) != fresh.frames.get(*at)
                || kept.rows.get(*at) != fresh.rows.get(*at)
        };
        let at = (0..longest).find(differs)?;
        let (frame, rows) = (
            (kept.frames.get(at), fresh.frames.get(at)),
            (kept.rows.get(at), fresh.rows.get(at)),
        );
        Some(format!(
            "batch {at} is in {:?} with rows {:?}, afresh {:?} with {:?}",
            frame.0, rows.0, frame.1, rows.1
        ))
    });
    if let Some(difference) = difference {
        panic!(
            "the kept batches differ from batching the scene afresh: {difference}; a slot \
             painted again was not patched into the kept batches or a change that \
             batching reads was not dropped from them (kept_batches.rs, \
             paint_partial.rs); reproduce with `cargo xtask test`"
        );
    }
}

/// Panics, naming the first step one has and the other does not, when
/// the steps `shown` in `view` through the extents index are not the
/// steps of `sources` testing every one shows: a step's extent is wrong
/// or a reach was not kept (`frame_extents.rs`).
pub(crate) fn assert_candidates(shown: &[Shown], view: vapors_scene::Rect, sources: Sources<'_>) {
    let boxes = (sources.boxes, sources.frames);
    let in_view = |&(_, step): &(crate::paint_labels::Label, crate::paint_order::Step)| {
        hidden_items(step, boxes, view).is_none()
    };
    let every: Vec<_> = sources.drawn.labelled().filter(in_view).collect();
    let found: Vec<_> = shown
        .iter()
        .map(|&(label, step, _)| (label, step))
        .collect();
    let longest = every.len().max(found.len());
    let Some(at) = (0..longest).find(|&at| every.get(at) != found.get(at)) else {
        return;
    };
    panic!(
        "the extents index found {:?} showing at {at} in view {view:?} where testing every step \
         finds {:?}: a step's extent is wrong or a reach was not kept (frame_extents.rs, \
         paint_partial.rs drawn_updated); reproduce with `cargo xtask test`",
        found.get(at),
        every.get(at)
    );
}

/// The items of `run`.
pub(crate) fn run_items(run: &Run) -> Vec<Item> {
    match run {
        Run::Shadows(shadows) => shadows.iter().map(|&shadow| Item::Shadow(shadow)).collect(),
        Run::Quads(quads) => quads.iter().map(|&quad| Item::Quad(quad)).collect(),
        Run::Texts(items) | Run::Images(items) | Run::Icons(items) | Run::Surfaces(items) => {
            items.clone()
        }
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

#[path = "kept_batches_check_page.rs"]
mod page;
pub(crate) use page::assert_on_page;
