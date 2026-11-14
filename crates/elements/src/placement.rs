//! Placement kept across updates (IU3 3b, `docs/plans/iu3.md` "3b:
//! kept placement"): each box's rect, padding and scroll offset are
//! kept, and worked out again only for the boxes layout wrote
//! (`layout/written.rs`), built again or scrolled, and the boxes placed
//! in a box whose content moved (`placement_walk.rs`).

use crate::layout::{BoxRect, LayoutBox, containing_blocks};
pub(crate) use crate::placement_changes::{Changes, Placed};
use crate::placement_marks::Marked;
use crate::placement_walk::{Mark, Walk};
use crate::scroll::Offsets;
use crate::slot_list;

/// No slot: the root's `up`, and a new or freed slot's before it is placed.
pub(crate) const NO_SLOT: usize = usize::MAX;

/// A slot's rect before it is placed.
const NO_RECT: BoxRect = BoxRect {
    x: 0.0,
    y: 0.0,
    width: 0.0,
    height: 0.0,
};

/// Each slot's border box in its scroll frame (IU4: as if no box above
/// it were scrolled, `docs/plans/iu4.md` section 3), its padding and
/// scroll offset, and what placing it again needs. A scroll moves no
/// content: paint adds each frame's rounded offsets on read
/// (`frames.rs`).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Placement {
    pub(crate) rects: Vec<BoxRect>,
    /// Each slot's padding widths: a `%` padding follows its containing
    /// block's width, which can change with its border box unchanged.
    pub(crate) paddings: Vec<taffy::Rect<f32>>,
    /// Each slot's scroll offset as applied, in CSS pixels.
    pub(crate) offsets: Vec<[f32; 2]>,
    /// The slot whose content each slot is placed in.
    pub(crate) up: Vec<usize>,
    /// Each out-of-flow slot's containing block (`containing_blocks`).
    pub(crate) containing: Vec<Option<usize>>,
    /// Marks for the walk, all `Mark::None` between updates.
    pub(crate) marks: Vec<Mark>,
}

/// The laid-out boxes, which of them scroll, and the scale.
pub(crate) struct LaidOut<'a> {
    pub(crate) boxes: &'a [LayoutBox],
    pub(crate) scrolls: &'a dyn Fn(usize) -> bool,
    pub(crate) scale: f32,
}

impl<'a> LaidOut<'a> {
    pub(crate) fn new(
        boxes: &'a [LayoutBox],
        scrolls: &'a dyn Fn(usize) -> bool,
        scale: f32,
    ) -> Self {
        Self {
            boxes,
            scrolls,
            scale,
        }
    }
}

impl Placement {
    /// Places the boxes `changes` names, and those placed in a box
    /// whose content moved, settling `scroll` for them.
    pub(crate) fn place(
        &mut self,
        boxes: &LaidOut<'_>,
        scroll: &mut Offsets,
        changes: &Changes<'_>,
    ) -> Placed {
        if changes.every {
            *self = Self::every(boxes.boxes);
        }
        let chained = self.patch(boxes.boxes, changes);
        // The walk reads no mark when every slot is placed.
        let marked = if changes.every {
            Marked::default()
        } else {
            self.mark(changes)
        };
        let mut walk = Walk::new(self, boxes, scroll, changes.every, &marked.under);
        walk.place_from_root();
        let mut placed = walk.done();
        placed.chained = chained;
        self.assert_visited(&marked.slots);
        placed
    }

    /// Panics, naming the slot and the slot it is placed in, when a
    /// marked slot still holds its mark: the walk clears a mark as it
    /// visits, so one left is a slot the walk did not reach.
    fn assert_visited(&self, marked: &[usize]) {
        if let Some(&slot) = marked.iter().find(|&&slot| self.marks[slot] != Mark::None) {
            panic!(
                "the placement walk did not visit marked slot {slot}, placed in slot {}: the \
                 chain to it is stale (placement.rs, `patch`); `cargo test -p vapors-elements` \
                 reproduces it",
                self.up[slot]
            );
        }
    }

    /// Nothing kept, for `boxes`: every slot is marked built again.
    fn every(boxes: &[LayoutBox]) -> Self {
        let len = boxes.len();
        Self {
            rects: vec![NO_RECT; len],
            paddings: vec![taffy::Rect::zero(); len],
            offsets: vec![[0.0; 2]; len],
            up: vec![NO_SLOT; len],
            containing: containing_blocks(boxes),
            marks: vec![Mark::None; len],
        }
    }

    /// Sizes the vectors to `boxes`, and patches the chain: a freed
    /// slot is placed in no slot; a slot built again keeps the slot it
    /// was placed in, so a rebuilt subtree whose layout is lent unchanged
    /// (no ancestor laid out) is still marked to the root; and the
    /// slots placed in a slot built again are chained to it, so a slot
    /// newly taken, or whose box moved under another parent, is marked
    /// to the root too.
    fn patch(&mut self, boxes: &[LayoutBox], changes: &Changes<'_>) -> usize {
        self.resize(boxes.len());
        for &(slot, _) in changes.freed {
            self.up[slot] = NO_SLOT;
        }
        self.patch_containing(boxes, changes);
        self.chain_children(boxes, changes)
    }

    /// Chains each in-flow child of a slot built again or laid out to
    /// it, each such slot once (a slot laid out more than once is listed
    /// more than once); how many children. A slot changes the slot it is
    /// placed in only when its box is built again under another parent,
    /// or its parent is laid out with it newly in flow. A hoisted box is
    /// chained by `patch_containing`.
    fn chain_children(&mut self, boxes: &[LayoutBox], changes: &Changes<'_>) -> usize {
        let mut slots: Vec<usize> = changes.rebuilt.to_vec();
        slots.extend_from_slice(&changes.written.laid_out);
        slots.sort_unstable();
        slots.dedup();
        let mut chained = 0;
        for slot in slots {
            for &child in &boxes[slot].children {
                if self.containing[child].is_none() {
                    self.up[child] = slot;
                    chained += 1;
                }
            }
        }
        chained
    }

    /// Sizes the vectors to `len` slots.
    fn resize(&mut self, len: usize) {
        slot_list::resize(&mut self.rects, len, NO_RECT);
        slot_list::resize(&mut self.paddings, len, taffy::Rect::zero());
        slot_list::resize(&mut self.offsets, len, [0.0; 2]);
        slot_list::resize(&mut self.up, len, NO_SLOT);
        slot_list::resize(&mut self.containing, len, None);
        slot_list::resize(&mut self.marks, len, Mark::None);
    }

    /// Clears the containing block of each slot built again or freed,
    /// then sets it from each hoisted list layout wrote, chaining each
    /// hoisted box to its block.
    fn patch_containing(&mut self, boxes: &[LayoutBox], changes: &Changes<'_>) {
        let freed = changes.freed.iter().map(|(slot, _)| slot);
        for &slot in changes.rebuilt.iter().chain(freed) {
            self.containing[slot] = None;
        }
        for &block in &changes.written.hoisting {
            for &hoisted in &boxes[block].hoisted {
                self.containing[hoisted] = Some(block);
                self.up[hoisted] = block;
            }
        }
    }
}
