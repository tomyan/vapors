//! The kept hits following a splice (`kept_hits.rs`,
//! `kept_order_labelled.rs`): the hits of the box steps it took out taken
//! out, those of the box steps it relabelled given their new labels in
//! place (a relabel keeps each frame's list in order), so a row put in is
//! one patch, not every hit gathered again. The hits of the steps it put
//! in are put in by the patch for their slots that follows
//! (`paint_partial.rs` `drawn_updated`).

use super::KeptHits;
use crate::kept_order_splice::SpliceEdits;
use crate::paint_labels::Label;
use crate::paint_order::Step;

impl KeptHits {
    /// `self` following `edits`.
    pub(crate) fn splice(&mut self, edits: &SpliceEdits) {
        for &(label, step) in &edits.removed {
            if matches!(step, Step::Box(_)) {
                self.take(label);
            }
        }
        let boxes = edits
            .relabelled
            .iter()
            .filter(|(step, ..)| matches!(step, Step::Box(_)));
        let relabelled: Vec<(Label, Label)> = boxes.map(|&(_, old, new)| (old, new)).collect();
        self.relabel(&relabelled);
    }

    /// The hits of each `(old, new)` of `relabelled` labelled `new`: every
    /// one found by its old label first, while each frame's list is in
    /// order and no new label stands for an old one, then relabelled.
    fn relabel(&mut self, relabelled: &[(Label, Label)]) {
        let held: Vec<(usize, Label, Label)> = relabelled
            .iter()
            .filter_map(|&(old, new)| Some((self.frames.remove(&old)?, old, new)))
            .collect();
        let found: Vec<(usize, std::ops::Range<usize>, Label)> = held
            .into_iter()
            .map(|(frame, old, new)| (frame, self.positions(frame, old), new))
            .collect();
        for (frame, range, new) in found {
            self.frames.insert(new, frame);
            let list = self
                .by_frame
                .get_mut(&frame)
                .expect("a list for every noted frame");
            for entry in &mut list[range] {
                entry.0 = new;
            }
        }
    }

    /// Where the hits labelled `label` are in frame `frame`'s list.
    fn positions(&self, frame: usize, label: Label) -> std::ops::Range<usize> {
        let list = &self.by_frame[&frame];
        let start = list.partition_point(|&(at, _)| at < label);
        start..list.partition_point(|&(at, _)| at <= label)
    }
}
