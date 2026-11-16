//! Which rows of framed lanes can show in a view (`frames_lane.rs`): the
//! rows between the view's edges, widened by the ink noted past the
//! rows' boxes.

use rustc_hash::FxHashMap;
use vapors_scene::Rect;

use super::super::Frames;
use super::LaneBase;

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
    /// shown) per lane). One search a lane: each child's start comes from
    /// the same walk, and its frame, made from the lane's template, goes
    /// in the memo, so no later read of it folds the lane (4f-2b).
    pub(crate) fn lane_frames_meeting(&self, frame: usize, view: Rect) -> Vec<usize> {
        let Some(containers) = self.lanes.by_parent.get(&frame) else {
            return Vec::new();
        };
        let in_frame = self.view_in(frame, view);
        let meeting = |&container: &usize| self.lane_rows_meeting(container, in_frame);
        containers.iter().flat_map(meeting).collect()
    }

    /// The children of the framed lane of `container` meeting `in_frame`,
    /// the view in its parent frame, each child's frame memoised.
    pub(super) fn lane_rows_meeting(&self, container: usize, in_frame: Rect) -> Vec<usize> {
        let Some(shared) = &self.lanes.shared else {
            return Vec::new();
        };
        let base = self.lanes.bases[&container];
        let (from, to) = self.lane_interval(container, base, in_frame);
        let template = self.lane_template(base, self.at(base.parent));
        let rows = shared.borrow().children_between(container, from, to);
        let memoise = |(slot, offset): (usize, f64)| {
            let at = self.lane_child_at(template, base, offset);
            self.lanes.memo.borrow_mut().insert(slot, at);
            #[cfg(debug_assertions)]
            self.assert_memoised_folds(slot, at);
            slot
        };
        rows.into_iter().map(memoise).collect()
    }

    /// CSS pixels from `container`'s border box top that `in_frame`
    /// covers, a device pixel wider each way and widened by the ink noted
    /// past its rows.
    pub(super) fn lane_interval(
        &self,
        container: usize,
        base: LaneBase,
        in_frame: Rect,
    ) -> (f64, f64) {
        let (scale, origin) = (f64::from(self.scale), f64::from(base.origin));
        let [above, below] = self.lane_ink.0.get(&container).copied().unwrap_or_default();
        let from = (f64::from(in_frame.top) - 1.0) / scale - origin - f64::from(below);
        let to = (f64::from(in_frame.bottom) + 1.0) / scale - origin + f64::from(above);
        (from, to)
    }

    /// The rows of lane `container` that the view's top or bottom line may
    /// cut once their ink is counted: those meeting a band around each
    /// line, the lane's ink and a device pixel wide each way. A row that is
    /// none of these lies wholly inside the view, or wholly outside it,
    /// ink and all.
    pub(super) fn lane_edges(
        &self,
        container: usize,
        base: LaneBase,
        in_frame: Rect,
    ) -> Vec<usize> {
        let Some(shared) = &self.lanes.shared else {
            return Vec::new();
        };
        let (scale, origin) = (f64::from(self.scale), f64::from(base.origin));
        let [above, below] = self.lane_ink.0.get(&container).copied().unwrap_or_default();
        let (above, below, pixel) = (f64::from(above), f64::from(below), 1.0 / scale);
        let band = |line: f32| {
            let line = f64::from(line) / scale - origin;
            (line - below - pixel, line + above + pixel)
        };
        let lanes = shared.borrow();
        let rows = [band(in_frame.top), band(in_frame.bottom)]
            .into_iter()
            .flat_map(|(from, to)| lanes.children_between(container, from, to));
        rows.map(|(slot, _)| slot).collect()
    }

    /// Panics, naming the frame, when `at`, read from the memo, is not
    /// the frame the lanes give now: the lanes changed after the memo was
    /// filled, with no `set_lanes` to empty it.
    #[cfg(debug_assertions)]
    pub(super) fn assert_memo_holds(&self, frame: usize, at: super::super::FrameAt) {
        let now = self
            .lanes
            .shared
            .as_ref()
            .map(|lanes| lanes.borrow().changes());
        assert!(
            now.is_none_or(|now| now == self.lanes.generation),
            "lane frame {frame} read from the memo ({at:?}) was filled when the lanes had \
             changed {} times, and they have changed {now:?} times now: the lanes changed after the memo was filled and no \
             `set_lanes` emptied it (frames_lane.rs); reproduce with `cargo xtask test`",
            self.lanes.generation
        );
    }

    /// Panics, naming the frame, when `at`, made from the search's start,
    /// is not the frame a fold of the lane gives: two answers to where a
    /// row is.
    #[cfg(debug_assertions)]
    fn assert_memoised_folds(&self, slot: usize, at: super::super::FrameAt) {
        let folded = self.resolved_afresh(slot);
        assert!(
            folded == at,
            "lane frame {slot} from the search's start is {at:?}, the lane's fold gives \
             {folded:?}: `LaneTree::between_starts` and `start` disagree (layout/lane_tree*.rs); \
             reproduce with `cargo xtask test`"
        );
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

#[cfg(test)]
#[path = "frames_lane_meeting_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "frames_lane_meeting_stale_tests.rs"]
mod stale_tests;
