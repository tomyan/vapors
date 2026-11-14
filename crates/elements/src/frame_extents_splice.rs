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
    /// not draw.
    fn entry_at(&self, step: Step, label: Label) -> Option<(usize, Result<Pos, usize>)> {
        let (frame, top, tall) = self.at_of(step)?;
        let index = &self.frames[&frame];
        let at = if tall {
            Err(index.tall_position(label))
        } else {
            Ok(index.position(top, label))
        };
        Some((frame, at))
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
