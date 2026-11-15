//! The offsets of the rows a batch's items are in, as the pipelines take
//! them (`docs/plans/splice-layout.md` section 11.2, 4f): a batch may
//! hold items of several lane frames of one lane, each instance naming
//! its row by an index into this table, and the shader adds the row's
//! offset to the frame's shift. Whole device pixels, as a shift is
//! (`shift.rs`), and at most [`RowOffsets::CAPACITY`] rows: a longer run
//! is split into batches by the caller, never truncated or wrapped.

/// Each row's offset from the batch's frame, device pixels, whole; row 0
/// first. Rows not given are at no offset. Held as the uniform's bytes,
/// made once, as every draw writes or looks them up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowOffsets([u8; Self::CAPACITY * 8]);

impl RowOffsets {
    /// The rows one batch's table holds: two to each `vec4<f32>` of the
    /// uniform, 1 KiB.
    pub const CAPACITY: usize = 128;

    /// Every row at no offset: items in one frame.
    pub const NONE: Self = Self([0; Self::CAPACITY * 8]);

    /// The table of `offsets`, row `i` at `offsets[i]`.
    ///
    /// # Panics
    ///
    /// When there are more than [`RowOffsets::CAPACITY`], or one is not
    /// whole and finite: the caller splits a longer run, and a kept item
    /// moved off whole pixels would leave the grid it was snapped to.
    #[must_use]
    pub fn whole(offsets: &[[f32; 2]]) -> Self {
        assert!(
            offsets.len() <= Self::CAPACITY,
            "{} row offsets for one batch, over the table's {}: split the run into batches \
             of at most that many rows (row_offsets.rs); reproduce with `cargo xtask test`",
            offsets.len(),
            Self::CAPACITY
        );
        let mut table = Self::NONE;
        for (at, &offset) in offsets.iter().enumerate() {
            assert!(
                offset.iter().all(|v| v.is_finite() && v.fract() == 0.0),
                "row {at}'s offset of {offset:?} device pixels is not whole (row_offsets.rs); \
                 reproduce with `cargo xtask test`"
            );
            for (axis, value) in offset.iter().enumerate() {
                let at = (at * 2 + axis) * 4;
                table.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        table
    }

    /// The table's bytes as the uniform holds it: two rows to each
    /// sixteen bytes, little-endian.
    #[must_use]
    pub fn bytes(&self) -> &[u8; Self::CAPACITY * 8] {
        &self.0
    }
}

#[cfg(test)]
#[path = "row_offsets_tests.rs"]
mod tests;
