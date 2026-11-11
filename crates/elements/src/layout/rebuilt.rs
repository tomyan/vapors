//! What an update's box build changed, by slot (IU3 3c-3b,
//! `docs/plans/iu3.md` "Changed slots"): the slots a built box took,
//! the slots whose box is gone, and whether the tree's shape changed.
//! Paint patches what it keeps from these (`changed_slots.rs`).

use super::boxes::{Content, LayoutBox};
use super::slots::{Key, Taken};

/// The slots `KeptBoxes::rebuild` placed and freed.
#[derive(Debug)]
pub(crate) struct Rebuilt {
    /// Every box was built: what each slot held before is not known.
    pub(crate) every: bool,
    /// The slots a built box took, in no particular order.
    pub(crate) slots: Vec<usize>,
    /// The slots whose box is gone, with the key each held.
    pub(crate) freed: Vec<(usize, Key)>,
    /// A slot was freed or newly taken, or a rebuilt box's children
    /// differ from the box it replaced.
    pub(crate) shape_changed: bool,
}

impl Rebuilt {
    /// Every box of `boxes` built.
    pub(crate) fn every(boxes: &[LayoutBox]) -> Self {
        let held = |(slot, layout_box): (usize, &LayoutBox)| {
            (!matches!(layout_box.content, Content::Vacant)).then_some(slot)
        };
        Self {
            every: true,
            slots: boxes.iter().enumerate().filter_map(held).collect(),
            freed: Vec::new(),
            shape_changed: true,
        }
    }

    /// The boxes put at `slots` in `boxes` in place of those `taken`
    /// out, once every subtree is put.
    pub(crate) fn subtrees(boxes: &[LayoutBox], slots: Vec<usize>, taken: &Taken) -> Self {
        let freed = taken.unclaimed();
        let shape_changed =
            !freed.is_empty() || slots.iter().any(|&slot| reshaped(boxes, slot, taken));
        Self {
            every: false,
            slots,
            freed,
            shape_changed,
        }
    }

    /// How many boxes were built.
    pub(crate) fn count(&self) -> usize {
        self.slots.len()
    }
}

/// Whether the box at `slot` newly took it, or has other children than
/// the box taken from it. A kept key keeps its slot, so equal children
/// are the same boxes in the same order.
fn reshaped(boxes: &[LayoutBox], slot: usize, taken: &Taken) -> bool {
    taken
        .boxes
        .get(&slot)
        .is_none_or(|(_, old)| old.children != boxes[slot].children)
}
