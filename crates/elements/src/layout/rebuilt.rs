//! What an update's box build changed, by slot (IU3 3c-3b,
//! `docs/plans/iu3.md` "Changed slots"): the slots a built box took,
//! the slots whose box is gone, and whether the tree's shape changed.
//! Paint patches what it keeps from these (`changed_slots.rs`).

use super::boxes::{Content, LayoutBox};
use super::isolated::Changed;
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
    /// The slots whose box has other children than the box taken from
    /// it, each with the children it had: what the order splice reads
    /// (`order_splice.rs`).
    pub(crate) reshaped: Vec<(usize, Vec<usize>)>,
    /// The owners whose rebuilt subtree changed, for layout to settle
    /// (`isolated.rs`); none when every box was built.
    pub(crate) changed: Vec<Changed>,
    /// Owners whose box children were edited in place
    /// (`child_edits.rs`) rather than built from every child.
    pub(crate) edited: usize,
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
            reshaped: Vec::new(),
            changed: Vec::new(),
            edited: 0,
        }
    }

    /// The boxes put at `slots` in `boxes` in place of those `taken`
    /// out, once every subtree is put.
    pub(crate) fn subtrees(boxes: &[LayoutBox], slots: Vec<usize>, taken: &Taken) -> Self {
        let freed = taken.unclaimed();
        let newly_taken = slots.iter().any(|slot| !taken.boxes.contains_key(slot));
        let reshaped: Vec<(usize, Vec<usize>)> = slots
            .iter()
            .filter_map(|&slot| reshaped(boxes, slot, taken).map(|old| (slot, old.to_vec())))
            .collect();
        Self {
            every: false,
            shape_changed: !freed.is_empty() || newly_taken || !reshaped.is_empty(),
            slots,
            freed,
            reshaped,
            changed: Vec::new(),
            edited: 0,
        }
    }

    /// How many boxes were built.
    pub(crate) fn count(&self) -> usize {
        self.slots.len()
    }
}

/// The children the box taken from `slot` had, when the box at `slot`
/// has other children; `None` when they are the same or the slot is
/// newly taken. A kept key keeps its slot, so equal children are the
/// same boxes in the same order.
fn reshaped<'a>(boxes: &[LayoutBox], slot: usize, taken: &'a Taken) -> Option<&'a [usize]> {
    let (_, old) = taken.boxes.get(&slot)?;
    (old.children != boxes[slot].children).then_some(old.children.as_slice())
}
