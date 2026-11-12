//! What placement is given and what it reports (`placement.rs`): the
//! changes since the kept placement, and the slots placed.

use crate::changed_slots::ChangedSlots;
use crate::layout::{Key, Written};

/// What changed since the kept placement.
pub(crate) struct Changes<'a> {
    /// Nothing is kept: every slot is placed.
    pub(crate) every: bool,
    pub(crate) rebuilt: &'a [usize],
    pub(crate) freed: &'a [(usize, Key)],
    pub(crate) written: &'a Written,
    /// The slots of the containers scrolled since the last update.
    pub(crate) scrolled: &'a [usize],
}

/// What placing did: the slots not built again whose rect or padding
/// changed (moved by a scroll of a container above them alone apart),
/// and how many slots it placed.
pub(crate) struct Placed {
    pub(crate) moved: Vec<usize>,
    /// Kept slots moved by scrolls alone: their rects changed by the
    /// offsets of the containers above them, their layout kept, so
    /// their place in their content is the same (the a11y mirror reads
    /// none of them, R6).
    pub(crate) scrolled: Vec<usize>,
    pub(crate) count: usize,
    /// In-flow children chained before the walk (`chain_children`).
    pub(crate) chained: usize,
}

impl<'a> Changes<'a> {
    /// What `changed` names, what layout `written`, and the containers
    /// `scrolled`.
    pub(crate) fn since(
        changed: &'a ChangedSlots,
        written: &'a Written,
        scrolled: &'a [usize],
    ) -> Self {
        Self {
            every: changed.every,
            rebuilt: &changed.rebuilt,
            freed: &changed.freed,
            written,
            scrolled,
        }
    }
}
