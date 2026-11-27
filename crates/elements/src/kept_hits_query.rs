//! A hit test through the kept hits (`kept_hits.rs`): only the frames
//! that can show at its point are asked (`Frames::meeting`), and a debug
//! build holds the answer to testing every hit.

use vapors_dom::NodeId;

use super::{FramedHit, KeptHits, holds};
use crate::frames::Frames;
use crate::layout::BoxRect;
use crate::paint_labels::Label;

/// The padding box and radii of a box that rounds its content, by slot.
pub(crate) type Rounding<'a> = &'a dyn Fn(usize) -> Option<(BoxRect, [f32; 4])>;

impl KeptHits {
    /// The last hit in painting order whose rect on the page holds
    /// `[x, y]` (its left and top edges, not its right and bottom),
    /// among the frames that can show there.
    /// A hit under a rounded clip holds it only inside that clip's
    /// rounded padding box, which `rounding` gives by the box's slot
    /// (`kept_clips.rs` `Clips::rounding`).
    pub(crate) fn hit(
        &self,
        point: [f32; 2],
        frames: &Frames,
        rounding: Rounding<'_>,
    ) -> Option<NodeId> {
        let (found, _) = self.hit_counted(point, frames, rounding);
        if cfg!(debug_assertions) {
            self.assert_hit_as_scanned(point, (frames, rounding), found);
        }
        found
    }

    /// Whether `hit` holds `point` on the page: its rect, and the rounded
    /// clip its part is under, if any.
    fn holds_at(
        &self,
        &(label, (frame, (_, rect))): &(Label, FramedHit),
        point: [f32; 2],
        (frames, rounding): (&Frames, Rounding<'_>),
    ) -> bool {
        if !holds(&frames.hit(frame, rect), point) {
            return false;
        }
        let Some(&slot) = self.rounded.get(&label) else {
            return true;
        };
        let (padding, radii) = rounding(slot).unwrap_or_else(|| {
            panic!(
                "label {label}'s hits are under the rounded clip of box slot {slot}, which rounds \
                 nothing (kept_clips.rs `rounding`); reproduce with `cargo xtask test`"
            )
        });
        let [x, y] = point;
        let at = BoxRect {
            x,
            y,
            width: 0.0,
            height: 0.0,
        };
        crate::clip_rounded::holds(frames.on_page(frame, padding), radii, at)
    }

    /// Panics, naming the point and both answers, when `found`, through
    /// the frames that can show at `point`, is not the answer of testing
    /// every hit (`kept_hits_check` for hits, as `kept_batches_check` is
    /// for culling). O(hits), in a debug build only.
    fn assert_hit_as_scanned(
        &self,
        point: [f32; 2],
        (frames, rounding): (&Frames, Rounding<'_>),
        found: Option<NodeId>,
    ) {
        let every = self.by_frame.values().flatten();
        let holding = every.filter(|hit| self.holds_at(hit, point, (frames, rounding)));
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
    pub(super) fn hit_counted(
        &self,
        point: [f32; 2],
        frames: &Frames,
        rounding: Rounding<'_>,
    ) -> (Option<NodeId>, usize) {
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
            .filter(|hit| self.holds_at(hit, point, (frames, rounding)));
        let found = holding.max_by_key(|&&&(label, _)| label);
        (found.map(|(_, (_, (node, _)))| *node), tested.len())
    }
}
