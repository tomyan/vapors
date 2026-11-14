//! Debug check that each lane's starts are the ones its children's
//! layouts were written with (`docs/plans/splice-layout.md`, section
//! 11.2; `lane_aside.rs`): lane frames read a child's start from the
//! lane's tree, and Taffy may serve a container's layout from its cache,
//! writing nothing, so a tree that moved on from its last write puts
//! framed rows where they were not laid out. O(children) per lane, in
//! the layout check (`layout_check.rs`).

use super::LayoutBox;
use super::lane::Lane;
use super::lane_block_answer::narrow;
use super::lane_kind::LaneKind;
use super::lane_set::Lanes;
use super::layout_check::tolerance;

impl Lanes {
    /// Panics, naming the container and child, at the first written
    /// lane whose start and the child's written layout differ.
    pub(crate) fn assert_starts_written(&self, boxes: &[LayoutBox]) {
        for (&container, lane) in &self.block {
            lane.assert_written(container, boxes);
        }
        for (&container, lane) in &self.flex {
            lane.assert_written(container, boxes);
        }
    }
}

impl<K: LaneKind> Lane<K> {
    fn assert_written(&self, container: usize, boxes: &[LayoutBox]) {
        let Some(&first) = self.tree.handles().first() else {
            return;
        };
        if self.written.is_none() {
            return;
        }
        let terms = self.handles.len();
        for (handle, start) in self.tree.starts_from(first) {
            let slot = self.slots[&handle];
            let relative = self.relative.get(&slot).copied().unwrap_or(0.0);
            let lane = narrow(self.facts.top + start) + relative;
            let held = boxes[slot].layout.location.y;
            assert!(
                (lane - held).abs() <= tolerance(lane, held, terms),
                "lane child in slot {slot} of the container in slot {container} starts at \
                 {lane} in its lane but its layout was written at {held}: a size query moved \
                 the lane's leaves on from its last write and the container's layout came \
                 from Taffy's cache (layout/lane_aside.rs); reproduce with \
                 `cargo test -p vapors-elements`"
            );
        }
    }
}
