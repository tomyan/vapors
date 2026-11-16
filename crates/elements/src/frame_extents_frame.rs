//! One frame's index in the extents index (`frame_extents.rs`): finding,
//! taking out and placing an entry, and the reaches made again after a
//! change.

use super::{Blocks, Entry, FrameIndex, missing, order};
use crate::blocks::Pos;
use crate::paint_labels::Label;

impl FrameIndex {
    /// The entry of `label`, at `top`, tall or not, taken out.
    pub(super) fn taken(&mut self, label: Label, (top, tall): (f32, bool)) -> Entry {
        if tall {
            return self.tall.remove(self.tall_position(label));
        }
        let (gone, next) = self.entries.remove(self.position(top, label));
        self.reach_from(next, 0);
        gone
    }

    /// Where the entry of `label`, at `top`, is, or a panic naming it.
    pub(super) fn position(&self, top: f32, label: Label) -> Pos {
        let found = self
            .entries
            .binary_search_by(|entry| entry.top.total_cmp(&top).then(entry.label.cmp(&label)));
        found.unwrap_or_else(|_| missing(label))
    }

    /// Where the tall entry of `label` is, or a panic naming it.
    pub(super) fn tall_position(&self, label: Label) -> usize {
        let found = self.tall.iter().position(|entry| entry.label == label);
        found.unwrap_or_else(|| missing(label))
    }

    /// True when `entry` sorts between the entries around `at`, so it
    /// can take the label of the one there.
    pub(super) fn fits(&self, at: Pos, entry: &Entry) -> bool {
        let before = self.entries.prev(at).and_then(|at| self.entries.get(at));
        let after = self.entries.iter_from(at).nth(1);
        before.is_none_or(|before| order(before, entry).is_lt())
            && after.is_none_or(|after| order(entry, after).is_lt())
    }

    /// Each entry's reach from `from` on: the first `settled` of them
    /// set whatever they were, then stopping at the first whose reach
    /// is unchanged, as those after it keep theirs too.
    pub(super) fn reach_from(&mut self, from: Pos, settled: usize) {
        let before = self.entries.prev(from).and_then(|at| self.entries.get(at));
        let mut reach = before.map_or(f32::NEG_INFINITY, |entry| entry.reach);
        for (count, entry) in self.entries.iter_mut_from(from).enumerate() {
            let now = reach.max(entry.bottom);
            if count >= settled && now.to_bits() == entry.reach.to_bits() {
                return;
            }
            (entry.reach, reach) = (now, now);
        }
    }
}

impl FrameIndex {
    /// Takes `entries`, in any order, as the short entries, each with
    /// its reach.
    pub(super) fn sorted(&mut self, mut entries: Vec<Entry>) {
        entries.sort_unstable_by(order);
        self.entries = Blocks::from_sorted(entries);
        self.reach_from(Pos::default(), usize::MAX);
    }
}
