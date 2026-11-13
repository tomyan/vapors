//! A block container laid out by its lane (`docs/plans/splice-layout.md`,
//! section 2 (a) and (d)): the container's share settled
//! (`lane_block.rs`), only the children put in, laid out again or
//! sharing new inputs laid out and their leaves put in the tree
//! (`lane_block_child.rs`), the starts of the children from the first
//! that moved written into their layouts, and the container's answer
//! folded (`lane_block_answer.rs`). A container, or one of its
//! children, the lane does not lay out is handed to Taffy's block
//! algorithm, counted with the reason (`LaneCounts`).

use taffy::{
    BlockContext, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId, RunMode,
    compute_block_layout,
};

use super::lane::Lane;
use super::lane_block::{Container, Settled};
use super::lane_block_answer::{lane_answer, narrow};
use super::lane_block_summary::BlockSummary;
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
            Settled::Short(output) => Ok(output),
            Settled::Declined(decline) => Err(decline),
            Settled::Laid(container) => self
                .lane_run(id, &container)
                .and_then(|run| lane_answer(&container, &run)),
        };
        match laid {
            Ok(output) => {
                self.lane_counts.lanes += 1;
                output
            }
            Err(decline) => {
                self.lane_counts.decline(decline);
                self.lanes.remove(&usize::from(id));
                compute_block_layout(self, id, inputs, block)
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

    /// The container's run from its lane, filled for this ask; a
    /// declined lane is dropped, to be made afresh next time.
    fn lane_run(&mut self, id: NodeId, c: &Container) -> Result<BlockSummary, LaneDecline> {
        let slot = usize::from(id);
        let mut lane = self.lanes.remove(&slot).unwrap_or_default();
        let filled = self.fill(slot, c, &mut lane);
        if filled.is_ok() {
            self.lanes.insert(slot, lane);
        }
        filled
    }

    fn fill(
        &mut self,
        slot: usize,
        c: &Container,
        lane: &mut Lane,
    ) -> Result<BlockSummary, LaneDecline> {
        let share = c.share();
        if lane.share.as_ref() != Some(&share) {
            lane.share = Some(share);
            lane.all = true;
        }
        if lane.all {
            self.fill_all(slot, c, lane)?;
        } else {
            self.fill_inserted(c, lane)?;
            self.fill_stale(c, lane)?;
        }
        let run = lane.tree.total();
        if c.run_mode == RunMode::PerformLayout {
            self.write_starts(c, lane, &run);
        }
        Ok(run)
    }

    /// Every child laid out and put in a fresh tree, in order.
    fn fill_all(&mut self, slot: usize, c: &Container, lane: &mut Lane) -> Result<(), LaneDecline> {
        *lane = Lane {
            share: lane.share.take(),
            all: false,
            ..Lane::default()
        };
        let mut previous = None;
        for child in self.boxes[slot].children.clone() {
            let leaf = self.leaf_of(child, c, lane)?;
            previous = Some(lane.put(child, previous, leaf));
        }
        Ok(())
    }

    /// The children put in since the last layout, each after its
    /// previous sibling's leaf.
    fn fill_inserted(&mut self, c: &Container, lane: &mut Lane) -> Result<(), LaneDecline> {
        for (child, after) in std::mem::take(&mut lane.inserted) {
            let leaf = self.leaf_of(child, c, lane)?;
            let previous = after.map(|after| lane.handle(after));
            lane.put(child, previous, leaf);
        }
        Ok(())
    }

    /// The children built or cleared this update laid out again, their
    /// leaves set.
    fn fill_stale(&mut self, c: &Container, lane: &mut Lane) -> Result<(), LaneDecline> {
        let stale: Vec<usize> = (self.stale.iter().copied())
            .filter(|slot| lane.handles.contains_key(slot))
            .collect();
        for child in stale {
            let leaf = self.leaf_of(child, c, lane)?;
            lane.tree.set(lane.handle(child), leaf);
            lane.moved.push(child);
        }
        Ok(())
    }

    /// The child in `slot` laid out: its leaf, with its layout held
    /// for `write_starts`.
    fn leaf_of(
        &mut self,
        slot: usize,
        c: &Container,
        lane: &mut Lane,
    ) -> Result<BlockSummary, LaneDecline> {
        let laid = self.lane_child(NodeId::from(slot), c)?;
        lane.pending.insert(slot, laid.layout);
        BlockSummary::leaf(laid.leaf)
    }

    /// Writes the layout of every child from the first that moved on:
    /// its start from the tree, the container's inset and, when the
    /// container's top margin collapses with its first child's, less
    /// that margin, which Taffy applies outside the container.
    fn write_starts(&mut self, c: &Container, lane: &mut Lane, run: &BlockSummary) {
        let Some(first) = first_moved(lane) else {
            return;
        };
        let escape = if c.own_margins_collapse.start {
            run.top().resolve()
        } else {
            0.0
        };
        let top = f64::from(c.resolved_inset.top) - escape;
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

/// The moved child nearest the start, whose start and every later one
/// is written; none when nothing moved.
fn first_moved(lane: &Lane) -> Option<super::lane_tree::Handle> {
    lane.moved
        .iter()
        .map(|&slot| lane.handle(slot))
        .map(|handle| (handle, lane.tree.start(handle)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(handle, _)| handle)
}
