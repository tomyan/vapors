//! The reshaped slot's children before and after (`order_splice.rs`):
//! the runs put in, found between the head and tail the two share
//! (`layout/reshaped.rs`), so a change to a long child list reads only
//! the part between them.

use std::ops::Range;

use rustc_hash::{FxHashMap, FxHashSet};

use super::Declined;
use crate::layout::{Content, LayoutBox, Reshaped, Shared};

/// The reshaped slot's children now, and the slots it held between the
/// head and tail its children before and now share, each with its place
/// among those.
pub(super) struct Children<'a> {
    pub(super) owner: usize,
    pub(super) new: &'a [usize],
    shared: Shared,
    middle: FxHashMap<usize, usize>,
}

impl<'a> Children<'a> {
    pub(super) fn of(new: &'a [usize], reshaped: &Reshaped) -> Self {
        let shared = reshaped.shared;
        let middle = reshaped
            .old_between
            .iter()
            .enumerate()
            .map(|(rank, &slot)| (slot, rank))
            .collect();
        Self {
            owner: reshaped.slot,
            new,
            shared,
            middle,
        }
    }

    /// The runs of `new` not among the old children, as ranges of `new`;
    /// `Declined` when the kept slots are not in the same order, or an old
    /// slot not in `new` still holds a box (it moved to another parent).
    pub(super) fn inserted_runs(&self, boxes: &[LayoutBox]) -> Result<Vec<Range<usize>>, Declined> {
        let between = self.between();
        let now: FxHashSet<usize> = self.new[between.clone()].iter().copied().collect();
        let moved =
            |slot: &usize| !now.contains(slot) && !matches!(boxes[*slot].content, Content::Vacant);
        if self.middle.keys().any(moved) {
            return Err(Declined::Moved);
        }
        let mut runs: Vec<Range<usize>> = Vec::new();
        let mut last_rank = None;
        for at in between {
            match (self.middle.get(&self.new[at]), runs.last_mut()) {
                (Some(rank), _) if last_rank < Some(rank) => last_rank = Some(rank),
                (Some(_), _) => return Err(Declined::Reordered),
                (None, Some(run)) if run.end == at => run.end = at + 1,
                (None, _) => runs.push(at..at + 1),
            }
        }
        Ok(runs)
    }

    /// Whether the child at `at` in `new` was a child before.
    pub(super) fn kept_at(&self, at: usize) -> bool {
        !self.between().contains(&at) || self.middle.contains_key(&self.new[at])
    }

    /// Whether any child was a child before.
    pub(super) fn keeps_any(&self) -> bool {
        let kept = |at: usize| self.middle.contains_key(&self.new[at]);
        self.shared.head + self.shared.tail > 0 || self.between().any(kept)
    }

    /// The positions of `new` between the shared head and tail.
    fn between(&self) -> Range<usize> {
        self.shared.head..self.new.len() - self.shared.tail
    }
}

#[cfg(test)]
#[path = "order_splice_children_tests.rs"]
mod tests;
