//! A table's entries on the CPU side (`table.rs`): which are taken, what
//! each holds, and which changed. Entry 0 holds `V::FIRST`, taken for
//! good. Shared by the transform table (`flat-scene-f2.md`) and the
//! clip table (`flat-scene-f3.md`).

use crate::Holding;
use crate::free_ranges::FreeRanges;

/// What one entry of a table holds.
pub trait EntryValue: Copy + PartialEq + std::fmt::Debug {
    /// Bytes an entry takes in the buffer.
    const BYTES: u64;
    /// Entry 0's value, held for good, and a new entry's.
    const FIRST: Self;
    /// What a given-back entry is written with, when the table poisons
    /// them, so a stale reference reads it rather than a reused value.
    const POISON: Option<Self>;
    /// Appends its bytes, as the shader reads them.
    fn write_bytes(&self, out: &mut Vec<u8>);
    /// The refusal's holding when the table cannot grow past `limit`.
    fn holding(limit: &'static str) -> Holding;
}

/// The entries: their values, the free ranges, the encoder that last
/// wrote each, the given-back spans not yet written with the poison, and
/// how many values changed since last asked.
#[derive(Debug)]
pub(crate) struct Entries<V> {
    values: Vec<V>,
    free: FreeRanges,
    written_in: Vec<u64>,
    poisoned: Vec<(u32, u32)>,
    changed: u64,
}

impl<V: EntryValue> Entries<V> {
    /// `capacity` entries, entry 0 `V::FIRST` and taken.
    pub(crate) fn new(capacity: u32) -> Self {
        let size = u64::from(capacity);
        Self {
            values: vec![V::FIRST; capacity as usize],
            free: FreeRanges::after(1, size),
            written_in: vec![0; capacity as usize],
            poisoned: Vec::new(),
            changed: 0,
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

    /// Gives back the `count` entries from `at`, set to `V::POISON` when
    /// the table poisons, to be written before the next draw.
    pub(crate) fn give(&mut self, at: u32, count: u32) {
        self.free
            .give(u64::from(at)..u64::from(at) + u64::from(count));
        if let Some(poison) = V::POISON {
            let span = (at, at + count - 1);
            self.values[at as usize..=span.1 as usize].fill(poison);
            self.poisoned.push(span);
            self.changed += u64::from(count);
        }
    }

    /// The spans given back since last asked, each to write with the
    /// poison: apart, so the live entries between two are not written.
    pub(crate) fn take_poisoned(&mut self) -> Vec<(u32, u32)> {
        std::mem::take(&mut self.poisoned)
    }

    /// How many values changed since last asked: those set to another
    /// value, and those given back with the poison.
    pub(crate) fn take_changed(&mut self) -> u64 {
        std::mem::take(&mut self.changed)
    }

    /// Room for `capacity` entries, the new ones free.
    pub(crate) fn grow(&mut self, capacity: u32) {
        self.values.resize(capacity as usize, V::FIRST);
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
    pub(crate) fn set(&mut self, at: u32, values: &[V], encoder: u64) -> Option<(u32, u32)> {
        let start = at as usize;
        let mut changed: Option<(u32, u32)> = None;
        for (offset, value) in values.iter().enumerate() {
            let index = start + offset;
            if self.values[index] == *value {
                continue;
            }
            assert!(
                self.written_in[index] != encoder,
                "table entry {index} is written twice in one frame with different values: the \
                 draws recorded before the second write would read it too (table_entries.rs); \
                 reproduce with `cargo xtask test`"
            );
            (self.values[index], self.written_in[index]) = (*value, encoder);
            self.changed += 1;
            let index = u32::try_from(index).expect("an entry of the table");
            changed = Some(changed.map_or((index, index), |(first, _)| (first, index)));
        }
        changed
    }

    /// The value of entry `at`.
    pub(crate) fn get(&self, at: u32) -> V {
        self.values[at as usize]
    }

    /// The values from `first` to `last`, inclusive, as bytes.
    pub(crate) fn bytes(&self, (first, last): (u32, u32)) -> Vec<u8> {
        let mut out = Vec::new();
        for value in &self.values[first as usize..=last as usize] {
            value.write_bytes(&mut out);
        }
        out
    }
}
