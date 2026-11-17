//! The paint-order splices a rows workload's frames made, summed from
//! each frame's `UpdateCounts` (`docs/plans/paint-order-labels.md`,
//! section 5): how many were patched rather than made afresh, and the
//! labels relabelled, so `--scaling` records them beside the counts.

use vapors_elements::{SpliceDecline, UpdateCounts};

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
    /// Frames whose splice declined for a freed box's stacking.
    pub declined_freed_stacked: usize,
    /// Frames whose splice declined for an inserted box's stacking.
    pub declined_inserted_stacked: usize,
    /// Frames whose splice declined for any other reason.
    pub declined_other: usize,
    /// Frames whose update walked paint order afresh (`order_walks`).
    pub order_walks: usize,
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
            order_walks: self.order_walks + counts.order_walks,
            ..self.declined(counts.splice_decline)
        }
    }

    /// `self` with a decline for `reason` counted, when there is one.
    fn declined(self, reason: Option<SpliceDecline>) -> Self {
        match reason {
            None => self,
            Some(SpliceDecline::FreedStacked) => Self {
                declined_freed_stacked: self.declined_freed_stacked + 1,
                ..self
            },
            Some(SpliceDecline::InsertedStacked) => Self {
                declined_inserted_stacked: self.declined_inserted_stacked + 1,
                ..self
            },
            Some(_) => Self {
                declined_other: self.declined_other + 1,
                ..self
            },
        }
    }

    /// The line `--scaling` prints for `rows` rows.
    #[must_use]
    pub fn line(&self, rows: usize) -> String {
        format!(
            "scaling-splices: {rows} rows: {} frames, {} splices, {} patched, \
             paint_relabelled {}, paint_relabel_all {}, declined freed_stacked {} \
             inserted_stacked {} other {}, order_walks {}",
            self.frames,
            self.splices,
            self.patched,
            self.relabelled,
            self.relabel_all,
            self.declined_freed_stacked,
            self.declined_inserted_stacked,
            self.declined_other,
            self.order_walks
        )
    }
}

#[cfg(test)]
#[path = "splice_tally_tests.rs"]
mod tests;
