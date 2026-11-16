//! Batches over the rows of a lane (`docs/plans/splice-layout.md`
//! section 11.2, 4f): each item drawn at its batch's shift and its row's
//! offset. A batch's table of row offsets ([`RowOffsets`]) is kept with
//! its key and made again only when its rows' offsets change (a row
//! above them grown or put in): each draw still reads the rows' offsets
//! and compares them with the kept ones, but builds no table and hashes
//! none for the uniform slots.

use std::collections::HashMap;

use vapors_elements::{BatchKey, BatchRows};
use vapors_renderer::{RowOffsets, Shift};

/// Each batch's table of row offsets, by key, and the frame it was last
/// drawn in.
#[derive(Default)]
pub(super) struct RowTables {
    tables: HashMap<BatchKey, Table>,
    frame: u64,
}

struct Table {
    offsets: Vec<[f32; 2]>,
    rows: RowOffsets,
    drawn: u64,
}

impl RowTables {
    /// A new frame: tables not drawn in it are dropped at its end.
    pub(super) fn begin(&mut self) {
        self.frame += 1;
    }

    /// The table of batch `key`'s rows at `offsets`: the one kept when
    /// its offsets are these, else made again.
    pub(super) fn table(&mut self, key: BatchKey, offsets: &[[f32; 2]]) -> RowOffsets {
        let frame = self.frame;
        let table = self.tables.entry(key).or_insert_with(|| Table {
            offsets: offsets.to_vec(),
            rows: RowOffsets::whole(offsets),
            drawn: frame,
        });
        if table.offsets != offsets {
            (table.offsets, table.rows) = (offsets.to_vec(), RowOffsets::whole(offsets));
        }
        table.drawn = frame;
        table.rows
    }

    /// Drops the tables of the batches this frame did not draw.
    pub(super) fn release_undrawn(&mut self) {
        let frame = self.frame;
        self.tables.retain(|_, table| table.drawn == frame);
    }
}

/// Where one batch's items are drawn: its frame's shift, the table of its
/// rows' offsets, those offsets, and each item's row (all empty for a
/// batch in one frame).
pub(super) struct Moved<'a> {
    pub(super) shift: Shift,
    pub(super) table: &'a RowOffsets,
    pub(super) offsets: &'a [[f32; 2]],
    pub(super) rows: BatchRows<'a>,
}

impl Moved<'_> {
    /// Where the `at`th item is drawn, for the pipelines that draw an
    /// item at a time (images and surfaces): the shift for a batch in one
    /// frame (no rows); else the shift and its row's offset.
    ///
    /// # Panics
    ///
    /// When the batch has rows and the item has none, or its row no
    /// offset: drawn at the shift alone, it would land in the lane's
    /// first row.
    pub(super) fn item_shift(&self, at: usize) -> Shift {
        let items = self.rows.items();
        if items.is_empty() {
            return self.shift;
        }
        let offset = items
            .get(at)
            .and_then(|&row| self.offsets.get(row as usize));
        let [dx, dy] = *offset.unwrap_or_else(|| {
            panic!(
                "item {at} of a lane batch has no row with an offset ({} items' rows, {} \
                 offsets): each item of a batch over a lane's rows names its row, culled \
                 surfaces included (vapors-elements framed_batches_rows.rs); reproduce with \
                 `cargo xtask test`",
                items.len(),
                self.offsets.len()
            )
        });
        let [x, y] = self.shift.pixels();
        Shift::whole([x + dx, y + dy])
    }
}

#[cfg(test)]
#[path = "painter_rows_tests.rs"]
mod tests;
