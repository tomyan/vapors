//! A hit test through the kept hits (`kept_hits.rs`): only the frames
//! that can show at its point are asked (`Frames::meeting`), and a debug
//! build holds the answer to testing every hit.

use vapors_dom::NodeId;

use super::{FramedHit, KeptHits, holds};
use crate::frames::Frames;
use crate::paint_labels::Label;

impl KeptHits {
    /// The last hit in painting order whose rect on the page holds
    /// `[x, y]` (its left and top edges, not its right and bottom),
    /// among the frames that can show there.
    pub(crate) fn hit(&self, point: [f32; 2], frames: &Frames) -> Option<NodeId> {
        let (found, _) = self.hit_counted(point, frames);
        if cfg!(debug_assertions) {
            self.assert_hit_as_scanned(point, frames, found);
        }
        found
    }

    /// Panics, naming the point and both answers, when `found`, through
    /// the frames that can show at `point`, is not the answer of testing
    /// every hit (`kept_hits_check` for hits, as `kept_batches_check` is
    /// for culling). O(hits), in a debug build only.
    fn assert_hit_as_scanned(&self, point: [f32; 2], frames: &Frames, found: Option<NodeId>) {
        let every = self.by_frame.values().flatten();
        let holding =
            every.filter(|(_, (frame, (_, rect)))| holds(&frames.hit(*frame, *rect), point));
        let scanned = holding
            .max_by_key(|&&(label, _)| label)
            .map(|(_, (_, (node, _)))| *node);
        assert_eq!(
            found, scanned,
            "the hit test at {point:?} through the frames that can show there found {found:?}, \
             testing every hit finds {scanned:?}: Frames::meeting misses a frame holding a hit \
             (frames_nested.rs, frames_lane.rs); reproduce with `cargo xtask test`"
        );
    }

    /// `hit`, and how many hits it tested: those of the frames that can
    /// show at `point`, not every frame's.
    pub(super) fn hit_counted(&self, point: [f32; 2], frames: &Frames) -> (Option<NodeId>, usize) {
        let scale = frames.scale();
        let [x, y] = point.map(|v| v * scale);
        let view = vapors_scene::Rect::new(x, y, x + 1.0, y + 1.0);
        let lists = frames.meeting(view).into_iter();
        let tested: Vec<&(Label, FramedHit)> = lists
            .filter_map(|frame| self.by_frame.get(&frame))
            .flatten()
            .collect();
        let holding = tested
            .iter()
            .filter(|(_, (frame, (_, rect)))| holds(&frames.hit(*frame, *rect), point));
        let found = holding.max_by_key(|&&&(label, _)| label);
        (found.map(|(_, (_, (node, _)))| *node), tested.len())
    }
}
