//! The started rows' lane frames resolved in one pass (`frames_lane.rs`;
//! `docs/plans/splice-layout.md`, section 11.2, 4d).

use super::super::{FrameAt, Frames};
use crate::layout::SharedLanes;

impl Frames {
    /// Resolves the frames of this update's started children, one pass
    /// along each framed lane (sequential starts, `Lanes::offsets_from`),
    /// into the memo: the a11y take reads each started child's bounds,
    /// one fold of the lane each otherwise (section 11.2, 4d). A framed
    /// lane's starts are whole device pixels, so they equal the fold's.
    pub(crate) fn prime_started(&self) {
        let Some(shared) = &self.lanes.shared else {
            return;
        };
        for &(container, first) in &self.lanes.started {
            let Some(&base) = self.lanes.bases.get(&container) else {
                continue;
            };
            let outer = self.at(base.parent);
            let offsets = shared.borrow().offsets_from(container, first);
            let mut memo = self.lanes.memo.borrow_mut();
            for (slot, offset) in offsets {
                let at = self.lane_frame_at(base, offset, outer);
                if cfg!(debug_assertions) {
                    self.assert_primed_folds(shared, (container, slot), at);
                }
                memo.entry(slot).or_insert(at);
            }
        }
    }

    /// Panics, naming the frame, when `primed`, worked out in one pass,
    /// is not the frame the lane's fold for `slot` gives.
    fn assert_primed_folds(
        &self,
        shared: &SharedLanes,
        (container, slot): (usize, usize),
        primed: FrameAt,
    ) {
        let base = self.lanes.bases[&container];
        let folded = shared.borrow().frame_offset(container, slot);
        let folded = folded.map(|offset| self.lane_frame_at(base, offset, self.at(base.parent)));
        assert_eq!(
            Some(primed),
            folded,
            "lane frame {slot} primed in one pass along its lane (frames_lane.rs \
             `prime_started`) is not the frame its lane's fold gives; reproduce with \
             `cargo xtask test`"
        );
    }
}
