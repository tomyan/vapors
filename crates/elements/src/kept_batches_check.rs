//! The debug build's checks on kept batches (`kept_batches.rs`): every
//! draw's batches and their frames equal batching the whole scene
//! afresh in its frames (`framed_batches.rs`); and their items, put on
//! the page by their frames' shifts and clips, are the items batching
//! the scene on the page draws, in the same order, so the frames put
//! every item where the page has it. And the steps the extents index
//! finds showing in a view are those testing every step finds.

use vapors_scene::{Batched, Item, Run};

use crate::framed_batches::hidden_items;
use crate::frames::Frames;
use crate::frames_draw::BatchFrame;
use crate::kept_batches::{Shown, Sources};
use crate::kept_batches_keys::KeptBatched;

/// Panics, naming the first differing batch, frame or the culled
/// counts, when `kept` is not `fresh`.
pub(crate) fn assert_same(kept: &KeptBatched, fresh: &(Batched, Vec<BatchFrame>)) {
    let difference = first_difference(&kept.batched, &fresh.0).or_else(|| {
        let at = (0..kept.frames.len().max(fresh.1.len()))
            .find(|&at| kept.frames.get(at) != fresh.1.get(at))?;
        let (kept, fresh) = (kept.frames.get(at), fresh.1.get(at));
        Some(format!("batch {at} is in frame {kept:?}, afresh {fresh:?}"))
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

/// Panics, naming the first differing item, when `kept`'s items put on
/// the page by `frames` are not `page`'s: a frame's shift or clip is
/// not the one its items were painted for, or culling in the frame
/// differs from culling on the page.
pub(crate) fn assert_on_page(kept: &KeptBatched, frames: &Frames, page: &Batched) {
    let (on_page, page_items) = (items_on_page(kept, frames), page_items(page));
    let longest = on_page.len().max(page_items.len());
    let differing = (0..longest).find(|&at| on_page.get(at) != page_items.get(at));
    let culled = (kept.batched.culled, page.culled);
    if differing.is_none() && culled.0 == culled.1 {
        return;
    }
    let at = differing.unwrap_or(longest);
    on_page_differs(at, (on_page.get(at), page_items.get(at)), culled);
}

/// An item, its clip and whether its batch is culled.
type Placed = ((Item, Option<vapors_scene::Rect>), bool);

/// `kept`'s items put on the page by their frames in `frames`.
fn items_on_page(kept: &KeptBatched, frames: &Frames) -> Vec<Placed> {
    let batches = kept.batched.batches.iter().zip(&kept.frames);
    let on_page = batches.flat_map(|(batch, frame)| {
        let put = move |item| (frames.item(frame.0, &item, batch.clip), batch.culled);
        run_items(&batch.run).into_iter().map(put)
    });
    on_page.collect()
}

/// `page`'s items, each with its batch's clip.
fn page_items(page: &Batched) -> Vec<Placed> {
    let items = page.batches.iter().flat_map(|batch| {
        let item = move |item| ((item, batch.clip), batch.culled);
        run_items(&batch.run).into_iter().map(item)
    });
    items.collect()
}

/// Panics naming item `at`, as put on the page and as batched there,
/// and the culled counts.
fn on_page_differs(
    at: usize,
    (kept, page): (Option<&Placed>, Option<&Placed>),
    culled: (u64, u64),
) {
    panic!(
        "the kept batches put on the page by their frames differ from batching the scene on \
         the page at item {at}: {kept:?}, on the page {page:?} (culled {} and {}); a frame's \
         shift or clip is not the one its items were painted for (frames.rs, frames_draw.rs), \
         or culling in a frame's view differs from culling on the page (framed_batches.rs); \
         reproduce with `cargo xtask test`",
        culled.0, culled.1
    );
}

/// Panics, naming the first step one has and the other does not, when
/// the steps `shown` in `view` through the extents index are not the
/// steps of `sources` testing every one shows: a step's extent is wrong
/// or a reach was not kept (`frame_extents.rs`).
pub(crate) fn assert_candidates(shown: &[Shown], view: vapors_scene::Rect, sources: Sources<'_>) {
    let boxes = (sources.boxes, sources.frames);
    let in_view =
        |&(_, step): &(usize, crate::paint_order::Step)| hidden_items(step, boxes, view).is_none();
    let every: Vec<_> = sources.drawn.placed().filter(in_view).collect();
    let found: Vec<_> = shown
        .iter()
        .map(|&(place, step, _)| (place, step))
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
fn run_items(run: &Run) -> Vec<Item> {
    match run {
        Run::Shadows(shadows) => shadows.iter().map(|&shadow| Item::Shadow(shadow)).collect(),
        Run::Quads(quads) => quads.iter().map(|&quad| Item::Quad(quad)).collect(),
        Run::Shapes(shapes) | Run::Meshes(shapes) => {
            shapes.iter().map(|&shape| Item::Shape(shape)).collect()
        }
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
