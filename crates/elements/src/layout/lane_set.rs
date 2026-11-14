//! The lanes beside the boxes (`lane.rs` is one lane): a map per kind
//! by the container's slot, and each child's container, which paint
//! reads to resolve a lane frame from the child's slot alone
//! (`docs/plans/splice-layout.md`, section 11).

use rustc_hash::FxHashMap;

use super::lane::Lane;
use super::lane_flex_share::FlexLane;
use super::lane_share::BlockLane;
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
}

impl Lanes {
    /// Drops the lane of the container in `slot`, of either kind.
    pub(crate) fn remove(&mut self, slot: usize) {
        self.block.remove(&slot);
        self.flex.remove(&slot);
    }

    /// Where the child in `child` starts in the border box of the lane
    /// container in `container`: the lane's top and the child's start,
    /// CSS pixels; `None` when `container` has no lane holding `child`.
    pub(crate) fn frame_offset(&self, container: usize, child: usize) -> Option<f64> {
        let block = self
            .block
            .get(&container)
            .and_then(|lane| lane.frame_offset(child));
        block.or_else(|| {
            self.flex
                .get(&container)
                .and_then(|lane| lane.frame_offset(child))
        })
    }

    /// The children of the lane container in `container` whose box or
    /// overflow meets `from..=to`, CSS pixels from its border box top,
    /// by slot; none when it has no lane.
    pub(crate) fn children_between(&self, container: usize, from: f64, to: f64) -> Vec<usize> {
        let block = self
            .block
            .get(&container)
            .map(|lane| lane.children_between(from, to));
        let flex = || {
            self.flex
                .get(&container)
                .map(|lane| lane.children_between(from, to))
        };
        block.or_else(flex).unwrap_or_default()
    }

    /// The lane container holding the child in `child`, if any.
    pub(crate) fn container_of(&self, child: usize) -> Option<usize> {
        let container = *self.parents.get(&child)?;
        self.frame_offset(container, child).map(|_| container)
    }

    /// Whether the children of the lane container in `container` can be
    /// frames of their own, its content origin along the axis `origin`
    /// at `scale` (`lane_frame.rs`); `None` when it has no lane, or too
    /// few children to frame (`Lane::framing`). or too
    /// few children to frame (`Lane::framing`).
    pub(crate) fn framing(
        &self,
        container: usize,
        origin: f64,
        scale: f32,
    ) -> Option<Result<(), LaneDecline>> {
        let block = self
            .block
            .get(&container)
            .and_then(|lane| lane.framing(origin, scale));
        block.or_else(|| {
            self.flex
                .get(&container)
                .and_then(|lane| lane.framing(origin, scale))
        })
    }

    /// The `position: relative` offset along the axis of the child in
    /// `child` of the lane container in `container`; 0 for none.
    pub(crate) fn relative(&self, container: usize, child: usize) -> f32 {
        let block = self
            .block
            .get(&container)
            .and_then(|lane| lane.relative.get(&child));
        let flex = || {
            self.flex
                .get(&container)
                .and_then(|lane| lane.relative.get(&child))
        };
        block.or_else(flex).copied().unwrap_or(0.0)
    }

    /// Tells the lane of the container in `owner`, if it has one, of
    /// the child put in at `slot` after `after` (`Lane::child_inserted`).
    pub(crate) fn child_inserted(&mut self, owner: usize, slot: usize, after: Option<usize>) {
        if let Some(lane) = self.block.get_mut(&owner) {
            lane.child_inserted(slot, after);
        }
        if let Some(lane) = self.flex.get_mut(&owner) {
            lane.child_inserted(slot, after);
        }
    }

    /// Tells the lane of the container in `owner`, if it has one, of
    /// the child taken out of `slot` (`Lane::child_removed`).
    pub(crate) fn child_removed(&mut self, owner: usize, slot: usize, next: Option<usize>) {
        if let Some(lane) = self.block.get_mut(&owner) {
            lane.child_removed(slot, next);
        }
        if let Some(lane) = self.flex.get_mut(&owner) {
            lane.child_removed(slot, next);
        }
    }
}
