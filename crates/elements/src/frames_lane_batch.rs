//! What batching over a lane's rows reads of the lane frames
//! (`frames_lane.rs`; `framed_batches.rs`, 4f): a row's lane and the
//! lane's template, which each row's offset is from.

use super::super::{FrameAt, Frames};

impl Frames {
    /// Which setting of the lanes these frames hold: another after every
    /// paint, and the same while nothing is painted (`next_epoch`).
    pub(crate) fn epoch(&self) -> u64 {
        self.lanes.epoch
    }

    /// The framed lane container whose child frame `frame` is, if it is
    /// one: what batches over a lane's rows are grouped by (4f).
    pub(crate) fn lane_container_of(&self, frame: usize) -> Option<usize> {
        let container = self.lanes.shared.as_ref()?.borrow().container_of(frame)?;
        self.lanes
            .bases
            .contains_key(&container)
            .then_some(container)
    }

    /// Where the batches over framed lane `container`'s rows are drawn:
    /// the frame its content is in, cut to the clip it gives its content
    /// (`lane_template`). A row's offset from it is its frame's shift
    /// less this one's, an exact integer (`lane_child_at`).
    ///
    /// # Panics
    ///
    /// When `container` is no framed lane container.
    pub(crate) fn lane_template_of(&self, container: usize) -> FrameAt {
        let base = *self.lanes.bases.get(&container).unwrap_or_else(|| {
            panic!(
                "a batch names lane {container}, which these frames hold no base for: a batch \
                 kept from another paint (frames_lane.rs); reproduce with `cargo xtask test`"
            )
        });
        self.lane_template(base, self.at(base.parent))
    }
}
