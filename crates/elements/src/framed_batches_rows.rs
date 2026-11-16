//! Batching the steps of one framed lane's rows (`framed_batches.rs`,
//! 4f): each item culled in its own row's view and batched under its
//! clip moved into the lane's coordinates, where the painter scissors
//! it; a part for each [`ROWS_PER_BATCH`] rows with an item batched, so
//! the split falls where the items shown fall, whichever steps are given
//! (every drawn step, or those the extents index finds showing).

use vapors_scene::{Batched, Item, ROWS_PER_BATCH, Rect};

use super::{BatchRows, Resolved, Sources};
use crate::frames_draw::view_at;
use crate::scene_view::frame_items;

/// The batches of `run`, the steps of lane `container`'s rows, and each
/// batch's rows, a part for each [`ROWS_PER_BATCH`] rows shown.
pub(super) fn rows_batched(
    run: &[Resolved],
    (container, (boxes, frames)): (usize, Sources<'_>),
    view: Rect,
) -> Vec<(Batched, Vec<BatchRows>)> {
    let template = frames.lane_template_of(container).shift;
    let mut last: Option<(usize, Row)> = None;
    let mut parts = Parts::default();
    for at in run {
        let row = match last {
            Some((frame, row)) if frame == at.frame => row,
            _ => {
                let frame = frames.at(at.frame);
                let offset = [frame.shift[0] - template[0], frame.shift[1] - template[1]];
                let row = Row {
                    view: view_at(frame, view),
                    offset,
                };
                last = Some((at.frame, row));
                row
            }
        };
        for (item, clip) in frame_items(boxes, at.step).1 {
            parts.add((at.frame, row), &item, clip);
        }
    }
    parts.finish()
}

/// Where a row's items are culled, its view in its frame, and its
/// offset from the lane's template: what its clips are moved by.
#[derive(Clone, Copy)]
struct Row {
    view: Rect,
    offset: [f32; 2],
}

/// The parts made so far, the one being filled, each of its batched
/// items' row frame, and the rows it holds: counted each time the row of
/// a batched item changes, which counts a row its items return to twice
/// (a split early, never a table overrun) and costs no set.
#[derive(Default)]
struct Parts {
    done: Vec<(Batched, Vec<usize>)>,
    part: Batched,
    item_rows: Vec<usize>,
    rows: usize,
}

impl Parts {
    /// `item` under `clip` in row frame `frame` added, culled in the
    /// row's view; the part is closed first when a shown item would
    /// bring it a row past [`ROWS_PER_BATCH`].
    fn add(&mut self, (frame, row): (usize, Row), item: &Item, clip: Option<Rect>) {
        let new_row = self.item_rows.last() != Some(&frame);
        let shows = !vapors_scene::culled(item, clip, row.view) || is_surface(item);
        if shows && new_row && self.rows == ROWS_PER_BATCH {
            self.close();
        }
        let in_lane = clip.map(|clip| moved(clip, row.offset));
        if self.part.add_clipped(item, (clip, in_lane), row.view) {
            self.rows += usize::from(self.item_rows.last() != Some(&frame));
            self.item_rows.push(frame);
        }
    }

    /// The part being filled closed, and a new one started.
    fn close(&mut self) {
        let part = std::mem::take(&mut self.part);
        self.done.push((part, std::mem::take(&mut self.item_rows)));
        self.rows = 0;
    }

    /// Every part, each batch with its rows.
    fn finish(mut self) -> Vec<(Batched, Vec<BatchRows>)> {
        self.close();
        let with_rows = |(part, item_rows): (Batched, Vec<usize>)| {
            let rows = rows_of(&part, &item_rows);
            (part, rows)
        };
        self.done.into_iter().map(with_rows).collect()
    }
}

fn is_surface(item: &Item) -> bool {
    matches!(item, Item::Surface { .. })
}

/// `clip` moved by `offset`; unmoved, to the bit, by none (a key tells
/// -0.0 from 0.0).
// Exact: a zero offset is exactly zero.
#[allow(clippy::float_cmp)]
fn moved(clip: Rect, offset: [f32; 2]) -> Rect {
    if offset == [0.0, 0.0] {
        clip
    } else {
        clip.offset(offset)
    }
}

/// Each of `part`'s batches' rows, from each batched item's row frame in
/// order.
fn rows_of(part: &Batched, item_rows: &[usize]) -> Vec<BatchRows> {
    let mut items = item_rows.iter().copied();
    let rows = |batch: &vapors_scene::Batch| batch_rows(items.by_ref().take(batch.run.len()));
    part.batches.iter().map(rows).collect()
}

/// The rows of one batch whose items are in row frames `item_rows`: its
/// rows numbered as they first appear in it. A row's items are mostly
/// consecutive, so the last row is tried first, then the batch's rows
/// (at most [`ROWS_PER_BATCH`]) in turn.
fn batch_rows(item_rows: impl Iterator<Item = usize>) -> BatchRows {
    let mut rows = BatchRows::default();
    for row in item_rows {
        let last = rows.frames.len().checked_sub(1);
        let found = last
            .filter(|&at| rows.frames[at] == row)
            .or_else(|| rows.frames.iter().rposition(|&frame| frame == row));
        let at = found.unwrap_or_else(|| {
            rows.frames.push(row);
            rows.frames.len() - 1
        });
        rows.items.push(u32::try_from(at).expect("under 2^32 rows"));
    }
    rows
}
