//! The transform table's entries on the CPU side (`transform_table.rs`,
//! `docs/plans/flat-scene-f2.md`): which are taken, what each holds, and
//! which changed. Entry 0 is the identity, held for good.

use crate::free_ranges::FreeRanges;

/// A transform: a 4x4 matrix, column-major, as WGSL's `mat4x4<f32>`.
pub type Matrix = [f32; 16];

/// The identity.
pub const IDENTITY: Matrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// A translation by `[dx, dy]` device pixels.
#[must_use]
pub fn translation([dx, dy]: [f32; 2]) -> Matrix {
    let mut matrix = IDENTITY;
    (matrix[12], matrix[13]) = (dx, dy);
    matrix
}

/// The entries: their values, the free ranges, and the encoder that last
/// wrote each.
#[derive(Debug)]
pub(crate) struct Entries {
    values: Vec<Matrix>,
    free: FreeRanges,
    written_in: Vec<u64>,
}

impl Entries {
    /// `capacity` entries, entry 0 the identity and taken.
    pub(crate) fn new(capacity: u32) -> Self {
        let size = u64::from(capacity);
        Self {
            values: vec![IDENTITY; capacity as usize],
            free: FreeRanges::after(1, size),
            written_in: vec![0; capacity as usize],
        }
    }

    pub(crate) fn capacity(&self) -> u32 {
        u32::try_from(self.values.len()).expect("a table under 2^32 entries")
    }

    /// The first of `count` entries taken; `None` when no free range
    /// holds them, and the table must grow.
    pub(crate) fn take(&mut self, count: u32) -> Option<u32> {
        let at = self.free.take(u64::from(count))?;
        Some(u32::try_from(at).expect("an entry of the table"))
    }

    /// Gives back the `count` entries from `at`.
    pub(crate) fn give(&mut self, at: u32, count: u32) {
        self.free
            .give(u64::from(at)..u64::from(at) + u64::from(count));
    }

    /// Room for `capacity` entries, the new ones free.
    pub(crate) fn grow(&mut self, capacity: u32) {
        self.values.resize(capacity as usize, IDENTITY);
        self.written_in.resize(capacity as usize, 0);
        self.free.grow(u64::from(capacity));
    }

    /// Sets the entries from `at` to `values` for the draws of encoder
    /// `encoder`; the span that changed, if any.
    ///
    /// # Panics
    ///
    /// When an entry already written for `encoder` would change: a write
    /// lands before any draw of the submission runs, so a draw recorded
    /// earlier would read the later value.
    pub(crate) fn set(&mut self, at: u32, values: &[Matrix], encoder: u64) -> Option<(u32, u32)> {
        let start = at as usize;
        let mut changed: Option<(u32, u32)> = None;
        for (offset, value) in values.iter().enumerate() {
            let index = start + offset;
            // Exact: an entry unchanged to the bit needs no write.
            #[allow(clippy::float_cmp)]
            let same = self.values[index] == *value;
            if same {
                continue;
            }
            assert!(
                self.written_in[index] != encoder,
                "transform entry {index} is written twice in one frame with different values: \
                 the draws recorded before the second write would read it too \
                 (transform_entries.rs); reproduce with `cargo xtask test`"
            );
            (self.values[index], self.written_in[index]) = (*value, encoder);
            let index = u32::try_from(index).expect("an entry of the table");
            changed = Some(changed.map_or((index, index), |(first, _)| (first, index)));
        }
        changed
    }

    /// The values from `first` to `last`, inclusive, as bytes.
    pub(crate) fn bytes(&self, (first, last): (u32, u32)) -> Vec<u8> {
        self.values[first as usize..=last as usize]
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

#[cfg(test)]
#[path = "transform_entries_tests.rs"]
mod tests;
