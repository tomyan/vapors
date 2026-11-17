//! Batches over the rows of a lane (`docs/plans/splice-layout.md`
//! section 11.2, 4f): each item drawn at its batch's frame entry and its
//! row's entry in the batch's range of the transform table
//! (`painter_transforms.rs`, `docs/plans/flat-scene-f2.md`), the range
//! keyed by the lane part's first batch, so a part's batches share one.

use vapors_elements::{BatchKey, BatchRows};
use vapors_renderer::Shift;

/// Where one batch's items are drawn: its frame and that frame's shift,
/// its rows' offsets and the key their range is held by, and each item's
/// row (all empty for a batch in one frame).
pub(super) struct Moved<'a> {
    /// The frame the batch is in, whose entry in the transform table its
    /// items name.
    pub(super) frame: vapors_elements::BatchFrame,
    pub(super) shift: Shift,
    pub(super) offsets: &'a [[f32; 2]],
    /// The key the rows' range is held by: the lane part's first batch's,
    /// or the batch's own.
    pub(super) rows_key: BatchKey,
    pub(super) rows: BatchRows<'a>,
}

impl Moved<'_> {
    /// The row of the `at`th item, for the pipelines that draw an item
    /// at a time (images and surfaces): `None` for a batch in one frame
    /// (no rows).
    ///
    /// # Panics
    ///
    /// When the batch has rows and the item has none, or its row no
    /// offset: drawn in no row, it would land in the lane's first.
    pub(super) fn item_row(&self, at: usize) -> Option<u32> {
        let items = self.rows.items();
        if items.is_empty() {
            return None;
        }
        let row = items
            .get(at)
            .copied()
            .filter(|&row| (row as usize) < self.offsets.len());
        Some(row.unwrap_or_else(|| {
            panic!(
                "item {at} of a lane batch has no row with an offset ({} items' rows, {} \
                 offsets): each item of a batch over a lane's rows names its row, culled \
                 surfaces included (vapors-elements framed_batches_rows.rs); reproduce with \
                 `cargo xtask test`",
                items.len(),
                self.offsets.len()
            )
        }))
    }
}

#[cfg(test)]
#[path = "painter_rows_tests.rs"]
mod tests;
