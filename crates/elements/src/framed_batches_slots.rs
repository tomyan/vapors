//! Each framed lane's rows' slots in its parts' tables of row offsets
//! (`framed_batches.rs`; `docs/plans/splice-layout.md` section 11.2,
//! 4f-2e): a row takes a slot when it is first batched, keeps it while it
//! is shown, and gives it back when it leaves the view, so an insert gives
//! the new row a free slot and changes no other row's index (no kept
//! bytes written again). Slot `s` is entry `s % ROWS_PER_BATCH` of part
//! `s / ROWS_PER_BATCH`'s table.

use rustc_hash::{FxHashMap, FxHashSet};

/// Each framed lane's slots, by container.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowSlots {
    lanes: FxHashMap<usize, LaneSlots>,
}

/// One lane's rows' slots, its slots given back, and the next slot never
/// taken.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LaneSlots {
    of: FxHashMap<usize, u32>,
    free: Vec<u32>,
    next: u32,
}

impl RowSlots {
    /// Lane `container`'s slots.
    pub(crate) fn lane(&mut self, container: usize) -> &mut LaneSlots {
        self.lanes.entry(container).or_default()
    }

    /// Every slot held by a row not among `shown` (the frames of the
    /// steps shown) given back, and lanes with none held dropped. One set
    /// serves every lane: a frame shown as another lane's row (a box moved
    /// between lanes) keeps its slot here until it leaves the view, which
    /// draws nothing wrong, as this lane's batches no longer name it.
    /// Testing each frame's lane costs about 4k instructions an insert
    /// frame on `FlexInsert` (#423's review, L2).
    pub(crate) fn keep_only(&mut self, shown: impl Iterator<Item = usize>) {
        let shown: FxHashSet<usize> = shown.collect();
        for lane in self.lanes.values_mut() {
            lane.keep_only(&shown);
        }
        self.lanes.retain(|_, lane| !lane.of.is_empty());
    }

    /// How many rows hold slots, over every lane, and the most slots any
    /// lane has handed out: bounded by the rows shown when slots are given
    /// back.
    #[cfg(test)]
    pub(crate) fn held(&self) -> (usize, u32) {
        let rows = self.lanes.values().map(|lane| lane.of.len()).sum();
        let most = self.lanes.values().map(|lane| lane.next).max().unwrap_or(0);
        (rows, most)
    }
}

impl LaneSlots {
    /// Row `row`'s slot: the one it holds, else the lowest given back, else
    /// the next never taken.
    pub(crate) fn slot(&mut self, row: usize) -> u32 {
        if let Some(&slot) = self.of.get(&row) {
            return slot;
        }
        let slot = self.free.pop().unwrap_or_else(|| {
            self.next += 1;
            self.next - 1
        });
        self.of.insert(row, slot);
        slot
    }

    /// The slots of the rows not in `shown` given back, lowest last out.
    fn keep_only(&mut self, shown: &FxHashSet<usize>) {
        let free = &mut self.free;
        self.of.retain(|row, &mut slot| {
            let keep = shown.contains(row);
            if !keep {
                free.push(slot);
            }
            keep
        });
        free.sort_unstable_by(|a, b| b.cmp(a));
    }
}

#[cfg(test)]
#[path = "framed_batches_slots_tests.rs"]
mod tests;
