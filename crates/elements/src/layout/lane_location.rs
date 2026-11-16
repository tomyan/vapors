//! Where Taffy puts a lane child (`docs/plans/splice-layout.md`, section
//! 11.2): its layout holds its relative offset alone along the axis once
//! its lane wrote it, and the start is in the lane.

use taffy::{Layout, Point};

use super::lane_block_answer::narrow;
use super::lane_set::Lanes;

impl Lanes {
    /// Where Taffy puts the child in `slot`, laid out as `layout`: its
    /// lane's start added back when that lane wrote it (`written_start`,
    /// the predicate `restore_starts` and framing read too). The one
    /// place that rebuilds Taffy's value: placement of a lane not framed,
    /// scroll ranges and the layout and placement checks read it.
    pub(crate) fn taffy_location(&self, slot: usize, layout: &Layout) -> Point<f32> {
        let start = self
            .container_of(slot)
            .and_then(|container| self.lane(container)?.written_start(slot));
        let mut location = layout.location;
        location.y += start.map_or(0.0, narrow);
        location
    }

    /// The lane container holding the child in `child`, if any.
    pub(crate) fn container_of(&self, child: usize) -> Option<usize> {
        let container = *self.parents.get(&child)?;
        self.holds(container, child).then_some(container)
    }
}
