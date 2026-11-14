//! The lanes beside the boxes (`lane.rs` is one lane): a map per kind
//! by the container's slot, and each child's container, which paint
//! reads to resolve a lane frame from the child's slot alone
//! (`docs/plans/splice-layout.md`, section 11).

use rustc_hash::FxHashMap;
use taffy::{Layout, Point};

use super::LayoutBox;
use super::lane::Lane;
use super::lane_block_answer::narrow;
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
    /// Containers built again since lending, whose lanes are dropped
    /// once their boxes are lent (`Placed::drop_lanes`).
    pub(super) rebuilt: Vec<usize>,
    /// Children's `frame_offset`s worked out in one pass along a lane
    /// for this update (`offsets_from`), which `taffy_location` reads
    /// before folding the lane for one child; emptied when layout
    /// starts (`clear_offsets`), as the lanes change then.
    pub(super) offsets: std::cell::RefCell<FxHashMap<usize, f64>>,
}

impl Lanes {
    /// Drops the lane of the container in `slot`, of either kind. A lane
    /// that wrote its children first puts their starts back in their
    /// layouts in `boxes`, which then hold Taffy's values with no lane to
    /// add them: a container built again and lent its cache is not laid
    /// out again (section 11.2).
    pub(crate) fn remove(&mut self, slot: usize, boxes: &mut [LayoutBox]) {
        if let Some(lane) = self.block.remove(&slot) {
            lane.restore_starts(boxes);
        }
        if let Some(lane) = self.flex.remove(&slot) {
            lane.restore_starts(boxes);
        }
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

    /// Whether the lane container in `container` has a lane holding the
    /// child in `child`: a lookup, where `frame_offset` folds the lane.
    pub(crate) fn holds(&self, container: usize, child: usize) -> bool {
        let block = self.block.get(&container).map(|lane| lane.holds(child));
        let flex = || self.flex.get(&container).map(|lane| lane.holds(child));
        block.or_else(flex).unwrap_or(false)
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

    /// Notes that the child in `child` holds a scroll container, in the
    /// lane holding it, if any (`FrameFacts::note_scroller`).
    pub(crate) fn note_scroller(&mut self, child: usize) {
        let Some(container) = self.container_of(child) else {
            return;
        };
        if let Some(lane) = self.block.get_mut(&container) {
            lane.facts.note_scroller(child);
        }
        if let Some(lane) = self.flex.get_mut(&container) {
            lane.facts.note_scroller(child);
        }
    }

    /// The children of the lane container in `container` from the one
    /// in `first` on, by slot, in order; none when it has no lane
    /// holding `first`.
    pub(crate) fn children_from(&self, container: usize, first: usize) -> Vec<usize> {
        let block = self
            .block
            .get(&container)
            .map(|lane| lane.children_from(first));
        let flex = || {
            self.flex
                .get(&container)
                .map(|lane| lane.children_from(first))
        };
        block.or_else(flex).unwrap_or_default()
    }

    /// The children of the lane container in `container` from the one
    /// in `first` on, by slot, their offsets noted for `taffy_location`
    /// this update: O(children from `first`), not a fold each.
    pub(crate) fn offsets_from(&self, container: usize, first: usize) -> Vec<usize> {
        let block = self
            .block
            .get(&container)
            .map(|lane| lane.offsets_from(first));
        let flex = || {
            self.flex
                .get(&container)
                .map(|lane| lane.offsets_from(first))
        };
        let offsets = block.or_else(flex).unwrap_or_default();
        let slots = offsets.iter().map(|&(slot, _)| slot).collect();
        self.offsets.borrow_mut().extend(offsets);
        slots
    }

    /// Forgets the offsets `offsets_from` noted: layout changes the lanes.
    pub(crate) fn clear_offsets(&mut self) {
        self.offsets.get_mut().clear();
    }

    /// Where Taffy puts the child in `slot`, laid out as `layout`: a
    /// lane child's layout holds its relative offset alone along the
    /// axis (`docs/plans/splice-layout.md`, section 11.2), so its
    /// `frame_offset` is added back. The one place that rebuilds
    /// Taffy's value: placement of a lane not framed, scroll ranges and
    /// the layout and placement checks read it.
    pub(crate) fn taffy_location(&self, slot: usize, layout: &Layout) -> Point<f32> {
        let noted = self.offsets.borrow().get(&slot).copied();
        let start = noted.or_else(|| {
            self.container_of(slot)
                .and_then(|container| self.frame_offset(container, slot))
        });
        let mut location = layout.location;
        location.y += start.map_or(0.0, narrow);
        location
    }

    /// The lane container holding the child in `child`, if any.
    pub(crate) fn container_of(&self, child: usize) -> Option<usize> {
        let container = *self.parents.get(&child)?;
        self.holds(container, child).then_some(container)
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
