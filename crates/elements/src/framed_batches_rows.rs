//! Batching the steps of one framed lane's rows (`framed_batches.rs`,
//! 4f): each item culled in its own row's view and batched under its
//! clip moved into the lane's coordinates, where the painter scissors
//! it; a part for each [`ROWS_PER_BATCH`] rows with an item batched, so
//! the split falls where the items shown fall, whichever steps are given
//! (every drawn step, or those the extents index finds showing).

use vapors_scene::{Batched, Item, ROWS_PER_BATCH, Rect};

use super::{LastSeen, Resolved, RowsOfBatches, Sources};
use crate::frames_draw::view_at;
use crate::scene_view::frame_items;

/// The batches of `run`, the steps of lane `container`'s rows, and each
/// batch's rows, a part for each [`ROWS_PER_BATCH`] rows shown.
pub(super) fn rows_batched(
    run: &[Resolved],
    (container, (boxes, frames)): (usize, Sources<'_>),
    view: Rect,
) -> Vec<(Batched, RowsOfBatches)> {
    let template = frames.lane_template_of(container).shift;
    let mut last = LastSeen::default();
    let mut parts = Parts::default();
    for at in run {
        let row = last.get(at.frame, || {
            let frame = frames.at(at.frame);
            let offset = [frame.shift[0] - template[0], frame.shift[1] - template[1]];
            Row {
                view: view_at(frame, view),
                offset,
            }
        });
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

/// The parts made so far, each batch with its rows; the part being
/// filled and its batches' rows; the row of the last item batched; and
/// the rows the part holds, counted each time that row changes, which
/// counts a row its items return to twice (a split early, never a table
/// overrun) and costs no set.
#[derive(Default)]
struct Parts {
    done: Vec<(Batched, RowsOfBatches)>,
    part: Batched,
    part_rows: RowsOfBatches,
    last_row: Option<usize>,
    rows: usize,
}

impl Parts {
    /// `item` under `clip` in row frame `frame` added, culled in the
    /// row's view (one test for the split and the batching); the part is
    /// closed first when a shown item would bring it a row past
    /// [`ROWS_PER_BATCH`].
    fn add(&mut self, (frame, row): (usize, Row), item: &Item, clip: Option<Rect>) {
        let is_culled = vapors_scene::culled(item, clip, row.view);
        let new_row = self.last_row != Some(frame);
        if (!is_culled || is_surface(item)) && new_row && self.rows == ROWS_PER_BATCH {
            self.close();
        }
        let in_lane = clip.map(|clip| moved(clip, row.offset));
        let batches = self.part.batches.len();
        if self.part.add_culled(item, in_lane, is_culled) {
            self.note_row(frame, self.part.batches.len() > batches);
        }
    }

    /// The item just batched noted in row frame `frame`, in a new batch
    /// when `new_batch`.
    fn note_row(&mut self, frame: usize, new_batch: bool) {
        if new_batch {
            self.part_rows.start_batch();
        }
        self.part_rows.push_item(frame);
        if self.last_row != Some(frame) {
            (self.last_row, self.rows) = (Some(frame), self.rows + 1);
        }
    }

    /// The part being filled closed, and a new one started.
    fn close(&mut self) {
        #[cfg(debug_assertions)]
        self.assert_rows_held();
        let part = std::mem::take(&mut self.part);
        self.done.push((part, std::mem::take(&mut self.part_rows)));
        (self.last_row, self.rows) = (None, 0);
    }

    /// Panics, naming the batch, when a batch's item rows are not one an
    /// item of its run: `note_row` noted an item in another batch than the
    /// one `Batched` put it in.
    #[cfg(debug_assertions)]
    fn assert_rows_held(&self) {
        let rows = self.part_rows.iter();
        for (at, (batch, rows)) in self.part.batches.iter().zip(rows).enumerate() {
            assert_eq!(
                rows.items.len(),
                batch.run.len(),
                "batch {at} of a lane part holds {} item rows for {} items: an item was noted in \
                 another batch than it was batched in (framed_batches_rows.rs `note_row`); \
                 reproduce with `cargo xtask test`",
                rows.items.len(),
                batch.run.len()
            );
        }
    }

    /// Every part, each batch with its rows.
    fn finish(mut self) -> Vec<(Batched, RowsOfBatches)> {
        self.close();
        self.done
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
