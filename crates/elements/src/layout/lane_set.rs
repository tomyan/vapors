//! The lanes beside the boxes (`lane.rs` is one lane): a map per kind
//! by the container's slot, and each child's container, which paint
//! reads to resolve a lane frame from the child's slot alone
//! (`docs/plans/splice-layout.md`, section 11).

use rustc_hash::FxHashMap;

use super::LayoutBox;
use super::lane::Lane;
use super::lane_flex_share::FlexLane;
use super::lane_share::BlockLane;
use super::lane_view::LaneView;
use crate::lane_decline::LaneDecline;

/// The lanes as layout keeps them and paint reads them: paint resolves
/// a lane frame's shift from its start when it is read, at draw too
/// (`docs/plans/splice-layout.md`, section 11.2), so both hold them.
pub(crate) type SharedLanes = std::rc::Rc<std::cell::RefCell<Lanes>>;

/// The lanes beside the boxes, by the container's slot, a map per
/// kind.
#[derive(Debug, Default)]
pub(crate) struct Lanes {
    pub(super) block: FxHashMap<usize, Lane<BlockLane>>,
    pub(super) flex: FxHashMap<usize, Lane<FlexLane>>,
    /// Each child's container, as last put; an entry whose container no
    /// longer has a lane holding the child is stale, and read as none.
    pub(super) parents: FxHashMap<usize, usize>,
    /// Containers built again since lending, whose lanes are dropped
    /// once their boxes are lent (`Placed::drop_lanes`).
    pub(super) rebuilt: Vec<usize>,
    /// How many times a lane was put back after layout, dropped, or told
    /// of a child put in or taken out: a frames' memo filled at one count
    /// is stale at another (`Frames::lane_at` asserts it in a debug
    /// build).
    pub(super) changes: u64,
}

impl Lanes {
    /// How many times the lanes changed (`changes`).
    #[cfg(debug_assertions)]
    pub(crate) fn changes(&self) -> u64 {
        self.changes
    }

    /// The lane of the container in `container`, of either kind.
    pub(super) fn lane(&self, container: usize) -> Option<&dyn LaneView> {
        if let Some(lane) = self.block.get(&container) {
            return Some(lane);
        }
        self.flex.get(&container).map(|lane| lane as &dyn LaneView)
    }

    fn lane_mut(&mut self, container: usize) -> Option<&mut dyn LaneView> {
        if let Some(lane) = self.block.get_mut(&container) {
            return Some(lane);
        }
        self.flex
            .get_mut(&container)
            .map(|lane| lane as &mut dyn LaneView)
    }

    /// Drops the lane of the container in `slot`, of either kind. A lane
    /// that wrote its children first puts their starts back in their
    /// layouts in `boxes`, which then hold Taffy's values with no lane to
    /// add them: a container built again and lent its cache is not laid
    /// out again (section 11.2).
    pub(crate) fn remove(&mut self, slot: usize, boxes: &mut [LayoutBox]) {
        if let Some(lane) = self.block.remove(&slot) {
            self.changes += 1;
            lane.restore_starts(slot, boxes);
        }
        if let Some(lane) = self.flex.remove(&slot) {
            self.changes += 1;
            lane.restore_starts(slot, boxes);
        }
    }

    /// Drops the lane of the freed container in `slot`, restoring
    /// nothing: its children are freed or built again elsewhere, and
    /// their slots may hold other boxes now.
    pub(crate) fn forget(&mut self, slot: usize) {
        let dropped = self.block.remove(&slot).is_some() | self.flex.remove(&slot).is_some();
        self.changes += u64::from(dropped);
    }

    /// Where the child in `child` starts in the border box of the lane
    /// container in `container`: the lane's top and the child's start,
    /// CSS pixels; `None` when `container` has no lane holding `child`.
    /// Answered for a lane that never wrote too, whose children hold
    /// Taffy's values: callers outside framing read `written_start`.
    pub(crate) fn frame_offset(&self, container: usize, child: usize) -> Option<f64> {
        self.lane(container)?.frame_offset(child)
    }

    /// The children of the lane container in `container` whose box or
    /// overflow meets `from..=to`, CSS pixels from its border box top,
    /// by slot, each with its offset (`frame_offset`); none when it has no
    /// lane.
    pub(crate) fn children_between(
        &self,
        container: usize,
        from: f64,
        to: f64,
    ) -> Vec<(usize, f64)> {
        let lane = self.lane(container);
        lane.map_or_else(Vec::new, |lane| lane.children_between(from, to))
    }

    /// Whether the lane container in `container` has a lane holding the
    /// child in `child`: a lookup, where `frame_offset` folds the lane.
    pub(crate) fn holds(&self, container: usize, child: usize) -> bool {
        self.lane(container).is_some_and(|lane| lane.holds(child))
    }

    /// The slot of a lane container holding `children` children, for
    /// the tests that ask one.
    #[cfg(test)]
    pub(crate) fn container_of_children(&self, children: usize) -> usize {
        let block = self
            .block
            .iter()
            .map(|(&slot, lane)| (slot, lane.handles.len()));
        let flex = self
            .flex
            .iter()
            .map(|(&slot, lane)| (slot, lane.handles.len()));
        let mut lanes = block.chain(flex);
        let found = lanes.find(|&(_, held)| held == children);
        found.expect("a lane holds that many children").0
    }

    /// Whether `container`'s box, with `children` children, holds the box
    /// in `child`, from its lane; `None` when it has no lane holding every
    /// one of its children.
    pub(crate) fn holds_among(
        &self,
        container: usize,
        (child, children): (usize, usize),
    ) -> Option<bool> {
        let lane = self.lane(container).filter(|lane| lane.len() == children)?;
        Some(lane.holds(child))
    }

    /// The index of the child in `child` among the `children` children
    /// of `container`, from its lane; `None` when it has no lane holding
    /// every one of its children (a lane over a run of them counts from
    /// the run's start) or holding the child.
    pub(crate) fn index_of(
        &self,
        container: usize,
        (child, children): (usize, usize),
    ) -> Option<usize> {
        let lane = self.lane(container).filter(|lane| lane.len() == children)?;
        lane.index_of(child)
    }

    /// The children of the lane container in `container` from the one
    /// in `first` on, by slot, in order; `None` when it has no lane
    /// holding `first`: a `started` entry whose lane a later ask of the
    /// same layout dropped, its starts put back (`restore_starts`).
    pub(crate) fn children_from(&self, container: usize, first: usize) -> Option<Vec<usize>> {
        let lane = self.lane(container).filter(|lane| lane.holds(first))?;
        Some(lane.children_from(first))
    }

    /// Calls `each` with the children of the framed lane container in
    /// `container` from the one in `first` on, each with its
    /// `frame_offset`, in one pass along the lane; a panic naming it when
    /// it has no lane, as the frames hold its base only while placement
    /// framed it.
    pub(crate) fn each_offset_from(
        &self,
        (container, first): (usize, usize),
        each: &mut dyn FnMut(usize, f64),
    ) {
        let lane = self.lane(container).unwrap_or_else(|| {
            panic!(
                "the frames hold a lane base for slot {container}, which has no lane \
                 (paint_frames.rs `lane_bases`); reproduce with `cargo xtask test`"
            )
        });
        lane.each_offset_from(first, each);
    }

    /// Whether the children of the lane container in `container` can be
    /// frames of their own, its content origin along the axis `origin`
    /// at `scale` (`lane_frame.rs`); `None` when it has no lane, or too
    /// few children to frame (`Lane::framing`).
    pub(crate) fn framing(
        &self,
        container: usize,
        origin: f64,
        scale: f32,
    ) -> Option<Result<(), LaneDecline>> {
        self.lane(container)?.framing(origin, scale)
    }

    /// The `position: relative` offset along the axis of the child in
    /// `child` of the lane container in `container`; 0 for none.
    pub(crate) fn relative(&self, container: usize, child: usize) -> f32 {
        self.lane(container)
            .map_or(0.0, |lane| lane.relative(child))
    }

    /// Tells the lane of the container in `owner`, if it has one, of
    /// the child put in at `slot` after `after` (`Lane::child_inserted`).
    pub(crate) fn child_inserted(&mut self, owner: usize, slot: usize, after: Option<usize>) {
        if let Some(lane) = self.lane_mut(owner) {
            lane.child_inserted(slot, after);
            self.changes += 1;
        }
    }

    /// Tells the lane of the container in `owner`, if it has one, of
    /// the child taken out of `slot` (`Lane::child_removed`).
    pub(crate) fn child_removed(&mut self, owner: usize, slot: usize, next: Option<usize>) {
        if let Some(lane) = self.lane_mut(owner) {
            lane.child_removed(slot, next);
            self.changes += 1;
        }
    }
}
