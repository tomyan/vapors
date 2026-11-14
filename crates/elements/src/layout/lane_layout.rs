//! A block container or a flex column laid out by its lane
//! (`docs/plans/splice-layout.md`, section 2 (a) and (d), section 10):
//! the container's inputs settled (`lane_block.rs`, `lane_flex.rs`),
//! only the children put in, laid out again or sharing new inputs laid
//! out and their leaves put in the tree (`lane_fill.rs`), the starts of
//! the children from the first that moved written into their layouts,
//! and the container's answer folded (`lane_kind.rs`). A container, or
//! one of its children, the lane does not lay out is handed to Taffy's
//! algorithm, counted with the reason (`LaneCounts`).

use taffy::{
    BlockContext, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, compute_block_layout,
    compute_flexbox_layout,
};

use super::lane::Lane;
use super::lane_block::Settled;
use super::lane_block_answer::narrow;
use super::lane_flex::FlexSettled;
use super::lane_flex_share::FlexLane;
use super::lane_kind::LaneKind;
use super::lane_share::BlockLane;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// How many containers a lane laid out this layout, how many were
/// declined, and the first decline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LaneCounts {
    pub(crate) lanes: usize,
    pub(crate) declined: usize,
    pub(crate) decline: Option<LaneDecline>,
}

impl LaneCounts {
    pub(super) fn decline(&mut self, decline: LaneDecline) {
        self.declined += 1;
        self.decline = self.decline.or(Some(decline));
    }
}

impl BoxTree<'_> {
    /// The block container at `id` laid out by its lane where it
    /// qualifies, else by Taffy.
    pub(super) fn lane_or_block(
        &mut self,
        id: NodeId,
        inputs: LayoutInput,
        block: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        if !self.lanes_enabled || usize::from(id) == 0 {
            return compute_block_layout(self, id, inputs, block);
        }
        let settled = match self.qualifies(id) {
            Ok(()) => self.lane_container(id, inputs, block.is_some()),
            Err(decline) => Settled::Declined(decline),
        };
        let laid = match settled {
            // Known without its children: no lane read, none counted
            Settled::Short(output) => return output,
            Settled::Declined(decline) => Err(decline),
            Settled::Laid(container) => self.lane_answer::<BlockLane>(id, &container),
        };
        self.counted(id, laid)
            .unwrap_or_else(|| compute_block_layout(self, id, inputs, block))
    }

    /// The flex container at `id` laid out by its lane where it is a
    /// column a lane lays out (`lane_flex.rs`), else by Taffy.
    pub(super) fn lane_or_flex(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        if !self.lanes_enabled {
            return compute_flexbox_layout(self, id, inputs);
        }
        let laid = match self.lane_column(id, inputs) {
            // Known without its items: no lane read, none counted
            FlexSettled::Short(output) => return output,
            FlexSettled::Declined(decline) => Err(decline),
            FlexSettled::Laid(column) => self.lane_answer::<FlexLane>(id, &column),
        };
        self.counted(id, laid)
            .unwrap_or_else(|| compute_flexbox_layout(self, id, inputs))
    }

    /// The container's answer counted as a lane's, or its decline
    /// counted and its lane dropped: then `None`, for Taffy to lay it
    /// out.
    fn counted(
        &mut self,
        id: NodeId,
        laid: Result<LayoutOutput, LaneDecline>,
    ) -> Option<LayoutOutput> {
        match laid {
            Ok(output) => {
                self.lane_counts.lanes += 1;
                Some(output)
            }
            Err(decline) => {
                self.lane_counts.decline(decline);
                self.lanes.remove(usize::from(id));
                None
            }
        }
    }

    /// Whether the container's own style lets a lane lay it out
    /// (section 4): no `align-content` but `normal` or `start`, which
    /// shift nothing. A text field is a block container whose one
    /// child is its run, laid out by a lane as by Taffy. The root
    /// (`html`, slot 0) is laid out by Taffy and not
    /// counted either way: its parent asks it nothing, `settle` lays
    /// it out again whenever anything below changed, and a lane there
    /// would save nothing; so is every container of the debug check's
    /// fresh layout (`lanes_enabled` off), which compares a lane with
    /// Taffy.
    fn qualifies(&self, id: NodeId) -> Result<(), LaneDecline> {
        use taffy::AlignContentKeyword::{Normal, Start};
        let node = &self.boxes[usize::from(id)];
        if !matches!(node.style.align_content.keyword, Normal | Start) {
            return Err(LaneDecline::AlignContent);
        }
        Ok(())
    }

    /// The container's answer from its lane, filled for this ask; a
    /// declined lane is dropped, to be made afresh next time.
    fn lane_answer<K: LaneKind>(
        &mut self,
        id: NodeId,
        c: &K::Container,
    ) -> Result<LayoutOutput, LaneDecline> {
        let slot = usize::from(id);
        let mut lane = K::lanes(self.lanes)
            .remove(&slot)
            .unwrap_or_else(|| Lane::new(K::gap(c)));
        let run = self.fill(slot, c, &mut lane)?;
        K::lanes(self.lanes).insert(slot, lane);
        K::answer(c, &run)
    }

    fn fill<K: LaneKind>(
        &mut self,
        slot: usize,
        c: &K::Container,
        lane: &mut Lane<K>,
    ) -> Result<K::Summary, LaneDecline> {
        let share = K::share(c);
        if lane.share.as_ref() != Some(&share) {
            lane.share = Some(share);
            lane.all = true;
        }
        let heights_changed = lane.heights.replace(K::heights(c)) != Some(K::heights(c));
        if lane.all {
            self.fill_all(slot, c, lane)?;
        } else {
            self.fill_changed(c, lane, heights_changed)?;
        }
        let run = lane.tree.total();
        if K::writes(c) {
            self.write_starts(c, lane, &run);
        }
        Ok(run)
    }

    /// Writes the layout of every child from the first that moved on:
    /// its start from the tree, from where the kind measures starts.
    fn write_starts<K: LaneKind>(
        &mut self,
        c: &K::Container,
        lane: &mut Lane<K>,
        run: &K::Summary,
    ) {
        let Some(first) = lane.first_moved() else {
            return;
        };
        let top = K::top(c, run);
        for (handle, start) in lane.tree.starts_from(first) {
            let slot = lane.slots[&handle];
            let (mut layout, relative) = lane.take_write(slot, self.boxes[slot].layout);
            layout.location.y = narrow(top + start) + relative;
            self.set_unrounded_layout(NodeId::from(slot), &layout);
        }
        lane.moved.clear();
        lane.pending.clear();
    }
}
