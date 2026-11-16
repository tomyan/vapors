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
pub struct RowOffsets {
    bytes: [u8; Self::CAPACITY * 8],
    /// The bytes' hash (`word_hash.rs`), made once: a
    /// draw looks its uniform up by it.
    hash: u64,
    /// Every row at no offset, as a batch in one frame has.
    none: bool,
}

impl RowOffsets {
    /// The rows one batch's table holds (`vapors_scene::ROWS_PER_BATCH`,
    /// which batching splits runs by): two to each `vec4<f32>` of the
    /// uniform, 1 KiB.
    pub const CAPACITY: usize = vapors_scene::ROWS_PER_BATCH;

    /// Every row at no offset: items in one frame.
    pub const NONE: Self = Self {
        bytes: [0; Self::CAPACITY * 8],
        // zero bytes hash to 0
        hash: 0,
        none: true,
    };

    /// The table of `offsets`, row `i` at `offsets[i]`.
    ///
    /// # Panics
    ///
    /// When there are more than [`RowOffsets::CAPACITY`], or one is not
    /// whole and finite: the caller splits a longer run, and a kept item
    /// moved off whole pixels would leave the grid it was snapped to.
    #[must_use]
    pub fn whole(offsets: &[[f32; 2]]) -> Self {
        assert_whole(offsets);
        let mut table = Self::NONE;
        let floats = offsets.iter().flatten();
        for (at, value) in floats.enumerate() {
            table.bytes[at * 4..at * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        table.hash = crate::word_hash::word_hash(&table.bytes);
        table.none = offsets.iter().flatten().all(|&value| value == 0.0);
        table
    }

    /// Whether every row is at no offset.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.none
    }

    /// The bytes' hash, as [`crate::UniformSlots::slot_with_rows`] finds
    /// a uniform by.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// `source`, a shader whose table is `array<vec4<f32>, ROW_PAIRS>`,
    /// with `ROW_PAIRS` written as `CAPACITY / 2`: the table's length
    /// has this one source.
    ///
    /// # Panics
    ///
    /// When `source` does not name `ROW_PAIRS` exactly once.
    #[must_use]
    pub fn shader(source: &str) -> String {
        assert_eq!(
            source.matches("ROW_PAIRS").count(),
            1,
            "a shader with row offsets names its table's length as ROW_PAIRS once \
             (row_offsets.rs); reproduce with `cargo xtask test`"
        );
        source.replace("ROW_PAIRS", &(Self::CAPACITY / 2).to_string())
    }

    /// The table's bytes as the uniform holds it: two rows to each
    /// sixteen bytes, little-endian.
    #[must_use]
    pub fn bytes(&self) -> &[u8; Self::CAPACITY * 8] {
        &self.bytes
    }
}

/// Panics unless `offsets` fit the table and are whole, finite device
/// pixels, naming the first that is not.
fn assert_whole(offsets: &[[f32; 2]]) {
    assert!(
        offsets.len() <= RowOffsets::CAPACITY,
        "{} row offsets for one batch, over the table's {}: split the run into batches of at \
         most that many rows (row_offsets.rs); reproduce with `cargo xtask test`",
        offsets.len(),
        RowOffsets::CAPACITY
    );
    let whole = |offset: &[f32; 2]| offset.iter().all(|v| v.is_finite() && v.fract() == 0.0);
    if let Some(at) = offsets.iter().position(|offset| !whole(offset)) {
        panic!(
            "row {at}'s offset of {:?} device pixels is not whole (row_offsets.rs); reproduce \
             with `cargo xtask test`",
            offsets[at]
        );
    }
}

#[cfg(test)]
#[path = "row_offsets_tests.rs"]
mod tests;
