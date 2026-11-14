//! Scroll offsets settled as the placement walk reaches each scroll
//! container (`placement_walk.rs`): clamped to the laid-out range, and
//! noted when the offset as applied, or the range, moved.

use super::Walk;
use crate::layout::Content;

impl Walk<'_, '_> {
    /// Settles the scroll offset of `slot` again, noting it when the
    /// offset as applied moved.
    // Exact: an offset kept is the same bits.
    #[allow(clippy::float_cmp)]
    pub(super) fn settle(&mut self, slot: usize) {
        let offset = self.settled_offset(slot);
        if std::mem::replace(&mut self.kept.offsets[slot], offset) != offset {
            self.placed.offsets_moved.push(slot);
        }
    }

    /// The scroll offset of `slot` as applied, settled again: none for
    /// a box that does not scroll, whose element's offset is dropped.
    fn settled_offset(&mut self, slot: usize) -> [f32; 2] {
        let Content::Element(node) = self.boxes.boxes[slot].content else {
            return [0.0; 2];
        };
        if !(self.boxes.scrolls)(slot) {
            self.scroll.forget(node);
            return [0.0; 2];
        }
        self.settled.insert(node);
        let scale = self.boxes.scale;
        let (applied, changed) = self
            .scroll
            .settle_box(node, (self.boxes.boxes, slot), scale);
        if changed {
            self.placed.states_changed.push(slot);
        }
        applied
    }
}
