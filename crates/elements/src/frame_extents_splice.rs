//! The extents index following a splice (`frame_extents.rs`,
//! `kept_order_labelled.rs`): the steps it took out taken out, those it
//! relabelled given their new labels in place (a relabel keeps every
//! entry's order by top then label), so a row put in is one patch, not
//! the index made afresh. The steps it put in are entered by the patch
//! for their slots that follows (`paint_partial.rs` `drawn_updated`).

use super::Extents;
use crate::blocks::Pos;
use crate::kept_order_splice::SpliceEdits;
use crate::paint_labels::Label;
use crate::paint_order::Step;

impl Extents {
    /// `self` following `edits`, then made for `slots` slots.
    pub(crate) fn splice(&mut self, edits: &SpliceEdits, slots: usize) {
        for &(label, step) in &edits.removed {
            self.remove(label, step);
        }
        self.relabel(&edits.relabelled);
        self.at.resize(slots, [None; 3]);
    }

    /// The entries of `relabelled`'s steps that draw, each relabelled from
    /// its old label to its new: every entry found by its old label first,
    /// while they are in order, then given its new one.
    fn relabel(&mut self, relabelled: &[(Step, Label, Label)]) {
        let found: Vec<_> = relabelled
            .iter()
            .map(|&(step, old, new)| Some((self.entry_at(step, old)?, new)))
            .collect();
        for (place, new) in found.into_iter().flatten() {
            self.label_entry(place, new);
        }
    }

    /// Where `step`'s entry, labelled `label`, is: its frame, and its
    /// position among the short entries or the tall; none when it does
    /// not draw. A short entry found there with another label or step is
    /// refused, naming both: a relabel would write over it.
    fn entry_at(&self, step: Step, label: Label) -> Option<(usize, Result<Pos, usize>)> {
        let (frame, top, tall) = self.at_of(step)?;
        let index = &self.frames[&frame];
        if tall {
            return Some((frame, Err(index.tall_position(label))));
        }
        let at = index.position(top, label);
        let found = index.entries.get(at).map(|entry| (entry.label, entry.step));
        assert_eq!(
            found,
            Some((label, step)),
            "the extents index finds another entry where {step:?}, labelled {label}, was \
             noted: its `at` or its order is stale (frame_extents_splice.rs); reproduce with \
             `cargo xtask test`"
        );
        Some((frame, Ok(at)))
    }

    /// The entry at `place` given the label `label`.
    fn label_entry(&mut self, (frame, at): (usize, Result<Pos, usize>), label: Label) {
        let index = self
            .frames
            .get_mut(&frame)
            .expect("a frame for every entry");
        match at {
            Ok(at) => {
                index
                    .entries
                    .get_mut(at)
                    .expect("an entry where found")
                    .label = label;
            }
            Err(at) => index.tall[at].label = label,
        }
    }
}

#[cfg(test)]
#[path = "frame_extents_splice_tests.rs"]
mod tests;
