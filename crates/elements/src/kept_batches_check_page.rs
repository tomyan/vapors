//! The debug build's check that the kept batches, put on the page as
//! the painter draws them, are the scene batched on the page
//! (`kept_batches_check.rs`).

use vapors_scene::{Batched, Item};

use super::run_items;
use crate::framed_batches::BatchRows;
use crate::frames::{Frames, shifted};
use crate::frames_draw::{BatchFrame, FrameDraw, Framing};
use crate::kept_batches_keys::KeptBatched;

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

/// `kept`'s items put on the page as the painter draws them: each
/// batch's clip moved by its frame's (or lane template's) shift and cut
/// to its clip, the scissor; each item moved by that shift and its row's
/// offset, what the shader adds (`vapors-app`'s `painter_runs.rs`).
fn items_on_page(kept: &KeptBatched, frames: &Frames) -> Vec<Placed> {
    let batches = kept
        .batched
        .batches
        .iter()
        .zip(&kept.frames)
        .zip(kept.rows.iter());
    let on_page = batches.flat_map(|((batch, &frame), rows)| {
        let (draw, offsets) = (frames.draw(frame), row_offsets(frames, frame, rows));
        let items = run_items(&batch.run).into_iter().enumerate();
        let put = move |(at, item): (usize, Item)| {
            let offset = item_offset(&offsets, rows, at);
            (as_painted(&item, batch.clip, (draw, offset)), batch.culled)
        };
        items.map(put).collect::<Vec<_>>()
    });
    on_page.collect()
}

/// The offsets of a batch's rows, as the painter reads them; none for a
/// batch in one frame.
fn row_offsets(frames: &Frames, frame: BatchFrame, rows: BatchRows<'_>) -> Vec<[f32; 2]> {
    match frame.0 {
        Framing::Frame(_) => Vec::new(),
        Framing::Lane(container) => frames.row_offsets(container, rows.frames),
    }
}

/// The `at`th item's row's offset: none for a batch in one frame.
fn item_offset(offsets: &[[f32; 2]], rows: BatchRows<'_>, at: usize) -> [f32; 2] {
    if rows.items.is_empty() {
        return [0.0, 0.0];
    }
    let row = rows
        .items
        .get(at)
        .and_then(|&row| offsets.get(row as usize));
    *row.unwrap_or_else(|| {
        panic!(
            "item {at} of a lane batch has no row with an offset ({} items' rows, {} offsets): \
             each batched item names its row (framed_batches_rows.rs); reproduce with `cargo \
             xtask test`",
            rows.items.len(),
            offsets.len()
        )
    })
}

/// `item` under `clip` as drawn at `draw` with its row's `offset`: the
/// item moved by both, what the shader adds; the clip as the painter's
/// scissor (`FrameDraw::on_page`). Unmoved, to the bit, by a zero shift
/// (a key tells -0.0 from 0.0).
// Exact: a zero shift is exactly zero.
#[allow(clippy::float_cmp)]
fn as_painted(
    item: &Item,
    clip: Option<vapors_scene::Rect>,
    (draw, offset): (FrameDraw, [f32; 2]),
) -> (Item, Option<vapors_scene::Rect>) {
    let shift = [draw.shift[0] + offset[0], draw.shift[1] + offset[1]];
    let item = if shift == [0.0, 0.0] {
        *item
    } else {
        shifted(*item, shift)
    };
    (item, draw.on_page(clip))
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
