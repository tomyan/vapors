//! The paint-order splices a rows workload's frames made, summed from
//! each frame's `UpdateCounts` (`docs/plans/paint-order-labels.md`,
//! section 5): how many were patched rather than made afresh, and the
//! labels relabelled, so `--scaling` records them beside the counts.

use vapors_elements::UpdateCounts;

/// Splices and relabels summed over the frames run, warmup included.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpliceTally {
    /// Frames run.
    pub frames: usize,
    /// Frames whose update spliced paint order.
    pub splices: usize,
    /// Of those, the ones that patched the drawing steps, the extents
    /// index and the hits rather than make them afresh.
    pub patched: usize,
    /// Labels a relabel moved (`paint_relabelled`).
    pub relabelled: usize,
    /// Relabels of every label (`paint_relabel_all`).
    pub relabel_all: usize,
}

impl SpliceTally {
    /// `self` with one more frame's `counts` added.
    #[must_use]
    pub fn and(self, counts: &UpdateCounts) -> Self {
        Self {
            frames: self.frames + 1,
            splices: self.splices + counts.order_splices,
            patched: self.patched + counts.order_splices_patched,
            relabelled: self.relabelled + counts.paint_relabelled,
            relabel_all: self.relabel_all + counts.paint_relabel_all,
        }
    }

    /// The line `--scaling` prints for `rows` rows.
    #[must_use]
    pub fn line(&self, rows: usize) -> String {
        format!(
            "scaling-splices: {rows} rows: {} frames, {} splices, {} patched, \
             paint_relabelled {}, paint_relabel_all {}",
            self.frames, self.splices, self.patched, self.relabelled, self.relabel_all
        )
    }
}

#[cfg(test)]
#[path = "splice_tally_tests.rs"]
mod tests;
