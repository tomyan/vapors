//! Scroll offsets settled as the placement walk reaches each scroll
//! container (`placement_walk.rs`): clamped to the laid-out range, and
//! noted when the offset as applied, or the range, moved.

use super::{Mark, Walk};
use crate::layout::Content;

impl Walk<'_, '_> {
    /// Settles the scroll offset of `slot`, marked `mark`, again, noting
    /// it when the offset as applied moved.
    // Exact: an offset kept is the same bits.
    #[allow(clippy::float_cmp)]
    pub(super) fn settle(&mut self, slot: usize, mark: Mark) {
        let offset = self.settled_offset(slot, mark);
        if std::mem::replace(&mut self.kept.offsets[slot], offset) != offset {
            self.placed.offsets_moved.push(slot);
        }
    }

    /// The scroll offset of `slot` as applied, settled again: none for
    /// a box that does not scroll, whose element's offset is dropped.
    /// One that only scrolled (`Mark::Scrolled`) is clamped to the range
    /// it kept; any other has its range measured from its content.
    fn settled_offset(&mut self, slot: usize, mark: Mark) -> [f32; 2] {
        let Content::Element(node) = self.boxes.boxes[slot].content else {
            return [0.0; 2];
        };
        if !(self.boxes.scrolls)(slot) {
            self.scroll.forget(node);
            return [0.0; 2];
        }
        self.settled.insert(node);
        let scale = self.boxes.scale;
        let (applied, changed) = if mark == Mark::Scrolled {
            self.scroll.settle_scrolled(node, scale)
        } else {
            self.placed.ranges_measured += 1;
            let laid = (self.boxes.boxes, self.boxes.lanes, slot);
            self.scroll.settle_box(node, laid, scale)
        };
        if changed {
            self.placed.states_changed.push(slot);
        }
        applied
    }
}
