//! Debug check that each lane's starts are the ones its children's
//! layouts were written with (`docs/plans/splice-layout.md`, section
//! 11.2; `lane_aside.rs`): a child's layout holds its relative offset
//! alone and its start is read from the lane's tree, and Taffy may serve
//! a container's layout from its cache, writing nothing, so a tree that
//! moved on from its last write puts rows where they were not laid out.
//! Each write records the starts (`note_written_starts`); the check
//! holds the tree to them. O(children log children) per lane, in debug
//! builds only, in the layout check (`layout_check.rs`).

use super::lane::Lane;
use super::lane_block_answer::narrow;
use super::lane_kind::LaneKind;
use super::lane_set::Lanes;
use super::layout_check::tolerance;

impl Lanes {
    /// Panics, naming the container and child, at the first written
    /// lane whose start differs from the one written.
    pub(crate) fn assert_starts_written(&self) {
        for (&container, lane) in &self.block {
            lane.assert_written(container);
        }
        for (&container, lane) in &self.flex {
            lane.assert_written(container);
        }
    }
}

impl<K: LaneKind> Lane<K> {
    /// Records each child's start as written.
    pub(super) fn note_written_starts(&mut self) {
        let starts = self
            .handles
            .iter()
            .map(|(&slot, &handle)| (slot, self.facts.top + self.tree.start(handle)));
        self.written_starts = starts.collect();
    }

    fn assert_written(&self, container: usize) {
        if !self.has_written() {
            return;
        }
        let terms = self.handles.len();
        for (&slot, &handle) in &self.handles {
            let lane = narrow(self.facts.top + self.tree.start(handle));
            let held = self.written_starts.get(&slot).copied().map(narrow);
            let held = held.unwrap_or_else(|| {
                panic!(
                    "lane child in slot {slot} of the container in slot {container} was never \
                     written by its lane (lane_write.rs `write_laid`); reproduce with \
                     `cargo test -p vapors-elements`"
                )
            });
            assert!(
                (lane - held).abs() <= tolerance(lane, held, terms),
                "lane child in slot {slot} of the container in slot {container} starts at \
                 {lane} in its lane but was written at {held}: a size query moved the lane's \
                 leaves on from its last write and the container's layout came from Taffy's \
                 cache (layout/lane_aside.rs); reproduce with `cargo test -p vapors-elements`"
            );
        }
    }
}
