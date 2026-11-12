//! The scroll offsets of the mirror's scroll containers, written once
//! every element is in place (R6 PR 5a, review-v243 H2): `scrollTo` on
//! a detached element does nothing, and Chromium resets an element's
//! offset when it is appended again, so a container's offset is
//! written after this apply when its record changed it, or its
//! element or one above it was made or moved by this apply.

use crate::a11y_mirror_dom::{Mirror, MirrorDom};

impl<D: MirrorDom> Mirror<D> {
    /// Writes the offset of every scroll container that needs it.
    pub(crate) fn write_scrolls(&mut self, dom: &mut D) {
        let due: Vec<(u64, [f32; 2])> = self
            .records
            .values()
            .filter_map(|record| Some((record.node.0, record.scroll?.offset)))
            .filter(|&(node, offset)| self.scroll_due(node, offset))
            .collect();
        for (node, offset) in due {
            if let Some(element) = self.elements.get(&node) {
                dom.set_scroll(element, offset);
            }
        }
        self.scroll_changed.clear();
    }

    /// Whether `node`'s offset must be written: its record changed it,
    /// or its element or one above it was made or moved this apply and
    /// the offset is not the 0 a placed element reads.
    fn scroll_due(&self, node: u64, offset: [f32; 2]) -> bool {
        if self.scroll_changed.contains(&node) {
            return true;
        }
        let reset = self.moved.contains(&node)
            || self.order.ancestors(node).any(|a| self.moved.contains(&a));
        // exact: any offset but 0 is one a reset loses
        reset && offset.map(f32::to_bits) != [0.0_f32.to_bits(); 2]
    }
}
