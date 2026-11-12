//! The free byte ranges of a [`Retained`](crate::Retained) buffer:
//! taken first fit, and merged with their neighbours when given back.

use std::ops::Range;

/// The ranges of a buffer of `size` bytes that no live range holds,
/// sorted by start, none touching another.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FreeRanges {
    free: Vec<Range<u64>>,
    size: u64,
}

impl FreeRanges {
    /// A buffer of `size` bytes, all free.
    pub(crate) fn new(size: u64) -> Self {
        Self::after(0, size)
    }

    /// A buffer of `size` bytes whose first `used` bytes are held.
    pub(crate) fn after(used: u64, size: u64) -> Self {
        let mut free = Vec::new();
        if used < size {
            free.push(used..size);
        }
        Self { free, size }
    }

    /// The start of `size` bytes taken from the first free range that
    /// holds them; `None` when none does.
    pub(crate) fn take(&mut self, size: u64) -> Option<u64> {
        let at = self
            .free
            .iter()
            .position(|free| free.end - free.start >= size)?;
        let start = self.free[at].start;
        self.free[at].start += size;
        if self.free[at].is_empty() {
            self.free.remove(at);
        }
        Some(start)
    }

    /// Gives `range` back, merged with the free ranges it touches.
    ///
    /// # Panics
    ///
    /// If `range` overlaps a free range or runs past the buffer: it was
    /// given back twice, or never taken.
    pub(crate) fn give(&mut self, range: Range<u64>) {
        let at = self.free.partition_point(|free| free.start < range.start);
        let before = at.checked_sub(1).map(|before| self.free[before].clone());
        let after = self.free.get(at).cloned();
        assert!(
            range.end <= self.size
                && before.as_ref().is_none_or(|b| b.end <= range.start)
                && after.as_ref().is_none_or(|a| range.end <= a.start),
            "retained range {range:?} given back overlaps the free ranges {:?} of a {}-byte \
             buffer: freed twice, or never taken (free_ranges.rs); reproduce with \
             `cargo xtask test`",
            self.free,
            self.size
        );
        self.insert(at, range, (before, after));
    }

    /// Bytes free in all, for the tests' view of the merges.
    #[cfg(test)]
    pub(crate) fn free_bytes(&self) -> u64 {
        self.free.iter().map(|free| free.end - free.start).sum()
    }

    /// Puts `range` at `at`, joined to `before` and `after` where they
    /// touch it.
    fn insert(
        &mut self,
        at: usize,
        range: Range<u64>,
        (before, after): (Option<Range<u64>>, Option<Range<u64>>),
    ) {
        let joins_before = before.is_some_and(|b| b.end == range.start);
        let joins_after = after.is_some_and(|a| a.start == range.end);
        match (joins_before, joins_after) {
            (true, true) => {
                self.free[at - 1].end = self.free[at].end;
                self.free.remove(at);
            }
            (true, false) => self.free[at - 1].end = range.end,
            (false, true) => self.free[at].start = range.start,
            (false, false) => self.free.insert(at, range),
        }
    }
}

#[cfg(test)]
#[path = "free_ranges_tests.rs"]
mod tests;
