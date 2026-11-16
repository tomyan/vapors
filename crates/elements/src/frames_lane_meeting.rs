//! Which rows of framed lanes can show in a view (`frames_lane.rs`): the
//! rows between the view's edges, widened by the ink noted past the
//! rows' boxes.

use rustc_hash::FxHashMap;
use vapors_scene::Rect;

use super::super::Frames;

/// How far each framed lane's children's ink reaches past their boxes,
/// the most seen: a history, which a paint made afresh does not have,
/// so frames compare equal whatever it holds. It only widens culling,
/// which the debug build checks against testing every step
/// (`kept_batches_check.rs`).
#[derive(Debug, Default)]
pub(crate) struct LaneInk(FxHashMap<usize, [f32; 2]>);

impl PartialEq for LaneInk {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Frames {
    /// The lane frames in frame `frame` that can show in `view`, device
    /// pixels on the page: the children of each framed lane in it whose
    /// box or overflow meets the view as it falls in the frame, a device
    /// pixel wider each way (`LaneTree::between`, O(log children +
    /// shown) per lane).
    pub(crate) fn lane_frames_meeting(&self, frame: usize, view: Rect) -> Vec<usize> {
        let (Some(containers), Some(shared)) =
            (self.lanes.by_parent.get(&frame), &self.lanes.shared)
        else {
            return Vec::new();
        };
        let in_frame = self.view_in(frame, view);
        let scale = f64::from(self.scale);
        let lanes = shared.borrow();
        let meeting = |container: &usize| {
            let origin = f64::from(self.lanes.bases[container].origin);
            let [above, below] = self.lane_ink.0.get(container).copied().unwrap_or_default();
            let from = (f64::from(in_frame.top) - 1.0) / scale - origin - f64::from(below);
            let to = (f64::from(in_frame.bottom) + 1.0) / scale - origin + f64::from(above);
            lanes.children_between(*container, from, to)
        };
        containers.iter().flat_map(meeting).collect()
    }

    /// Notes that ink in a child of the framed lane container
    /// `container` reaches `over` past its border box, above and below,
    /// CSS pixels: culling widens by the most noted (a box shadow, a
    /// glyph's overhang), which can only grow until paint order is
    /// walked, so it stays sound when the child that set it is gone.
    pub(crate) fn note_lane_ink(&mut self, container: usize, over: [f32; 2]) {
        let most = self.lane_ink.0.entry(container).or_default();
        *most = [most[0].max(over[0]), most[1].max(over[1])];
    }

    /// The lane ink `from` noted, taken into frames made afresh after a
    /// splice: the kept rows painted nothing again to note it with, and
    /// ink only grows until a walk, so it stays sound for the rows the
    /// splice took out.
    pub(crate) fn take_lane_ink(&mut self, from: &mut Frames) {
        self.lane_ink = std::mem::take(&mut from.lane_ink);
    }
}
