//! Batching the steps of one framed lane's rows (`framed_batches.rs`,
//! 4f): each item culled in its own row's view and batched under its
//! clip moved into the lane's coordinates, where the painter scissors
//! it; each shown row in its slot of a part's table
//! (`framed_batches_slots.rs`), a part for each [`ROWS_PER_BATCH`]
//! slots, and a batch split where its rows' slots change part.

use vapors_scene::{Batched, Item, ROWS_PER_BATCH, Rect};

use super::arena::NO_ROW;
use super::slots::LaneSlots;
use super::{LastSeen, Resolved, RowsOfBatches, Sources};
use crate::frames_draw::view_at;
use crate::scene_view::frame_items;

/// The batches of `run`, the steps of lane `container`'s rows, and each
/// batch's rows, each row in its slot from `slots`.
pub(super) fn rows_batched(
    run: &[Resolved],
    (container, (boxes, frames)): (usize, Sources<'_>),
    (view, slots): (Rect, &mut LaneSlots),
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
            parts.add(((at.frame, row), slots), &item, clip);
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
/// filled, its batches' rows, the number of the part its rows' slots are
/// in, and its table: the lane frame in each of its slots.
#[derive(Default)]
struct Parts {
    done: Vec<(Batched, RowsOfBatches)>,
    part: Batched,
    rows: RowsOfBatches,
    number: Option<u32>,
    table: Vec<usize>,
}

impl Parts {
    /// `item` under `clip` in row frame `frame` added, culled in the
    /// row's view (one test for the split and the batching); a shown
    /// item's row takes its slot from `slots`, and the part is closed
    /// first when that slot is in another part's table.
    fn add(
        &mut self,
        ((frame, row), slots): ((usize, Row), &mut LaneSlots),
        item: &Item,
        clip: Option<Rect>,
    ) {
        let is_culled = vapors_scene::culled(item, clip, row.view);
        if is_culled && !is_surface(item) {
            self.part.add_culled(item, None, true);
            return;
        }
        let slot = slots.slot(frame);
        let number = slot / ROWS_PER_TABLE;
        if self.number.is_some_and(|open| open != number) {
            self.close();
        }
        self.number = Some(number);
        let in_lane = clip.map(|clip| moved(clip, row.offset));
        let batches = self.part.batches.len();
        if self.part.add_culled(item, in_lane, is_culled) {
            self.note_row(
                (frame, slot % ROWS_PER_TABLE),
                self.part.batches.len() > batches,
            );
        }
    }

    /// The item just batched noted in row frame `frame`, entry `entry` of
    /// the part's table, in a new batch when `new_batch`.
    fn note_row(&mut self, (frame, entry): (usize, u32), new_batch: bool) {
        if new_batch {
            if self.rows.is_empty() {
                self.rows.open_part();
            }
            self.rows.start_batch(0);
        }
        self.rows.push_item(entry);
        let entry = entry as usize;
        if self.table.len() <= entry {
            self.table.resize(entry + 1, NO_ROW);
        }
        let held = self.table[entry];
        assert!(
            held == NO_ROW || held == frame,
            "rows {held} and {frame} are both in entry {entry} of a lane part's table: one slot \
             given to two rows (framed_batches_slots.rs `LaneSlots::slot`); the debug check \
             batching afresh shares the slots and cannot see it; reproduce with `cargo xtask test`"
        );
        self.table[entry] = frame;
    }

    /// The part being filled closed, its table written, and a new one
    /// started.
    fn close(&mut self) {
        #[cfg(debug_assertions)]
        self.assert_rows_held();
        if !self.rows.is_empty() {
            self.rows.close_part(0, &self.table);
        }
        let part = std::mem::take(&mut self.part);
        self.done.push((part, std::mem::take(&mut self.rows)));
        (self.number, self.table) = (None, Vec::new());
    }

    /// Panics, naming the batch, when a batch's item rows are not one an
    /// item of its run: `note_row` noted an item in another batch than the
    /// one `Batched` put it in.
    #[cfg(debug_assertions)]
    fn assert_rows_held(&self) {
        let rows = self.rows.iter();
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

/// Slots a part's table holds: `ROWS_PER_BATCH`, as a slot number.
#[allow(clippy::cast_possible_truncation)] // 128
const ROWS_PER_TABLE: u32 = ROWS_PER_BATCH as u32;

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
