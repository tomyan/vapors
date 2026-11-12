//! Placement kept across updates (IU3 3b, `docs/plans/iu3.md` "3b:
//! kept placement"): each box's rect, padding and scroll offset are
//! kept, and worked out again only for the boxes layout wrote
//! (`layout/written.rs`), built again or scrolled, and the boxes placed
//! in a box whose content moved (`placement_walk.rs`).

use crate::changed_slots::ChangedSlots;
use crate::layout::{BoxRect, Key, LayoutBox, Written, containing_blocks};
use crate::placement_walk::{Mark, Walk};
use crate::scroll::Offsets;

/// No slot: the root's `up`, and a new or freed slot's before it is placed.
pub(crate) const NO_SLOT: usize = usize::MAX;

/// A slot's rect before it is placed.
const NO_RECT: BoxRect = BoxRect {
    x: 0.0,
    y: 0.0,
    width: 0.0,
    height: 0.0,
};

/// Each slot's border box, scroll offsets applied, its padding and
/// scroll offset, and what placing it again needs.
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
/// changed, and how many slots it placed.
pub(crate) struct Placed {
    pub(crate) moved: Vec<usize>,
    pub(crate) count: usize,
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
        self.patch(boxes.boxes, changes);
        let marked = self.mark(changes);
        let mut walk = Walk::new(self, boxes, scroll, changes.every);
        walk.place_from_root();
        let placed = walk.done();
        for slot in marked {
            self.marks[slot] = Mark::None;
        }
        placed
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
    /// (no ancestor laid out) is still marked to the root.
    fn patch(&mut self, boxes: &[LayoutBox], changes: &Changes<'_>) {
        self.resize(boxes.len());
        for &(slot, _) in changes.freed {
            self.up[slot] = NO_SLOT;
        }
        self.patch_containing(boxes, changes);
    }

    /// Sizes the vectors to `len` slots.
    fn resize(&mut self, len: usize) {
        self.rects.resize(len, NO_RECT);
        self.paddings.resize(len, taffy::Rect::zero());
        self.offsets.resize(len, [0.0; 2]);
        self.up.resize(len, NO_SLOT);
        self.containing.resize(len, None);
        self.marks.resize(len, Mark::None);
    }

    /// Clears the containing block of each slot built again or freed,
    /// then sets it from each hoisted list layout wrote.
    fn patch_containing(&mut self, boxes: &[LayoutBox], changes: &Changes<'_>) {
        let freed = changes.freed.iter().map(|(slot, _)| slot);
        for &slot in changes.rebuilt.iter().chain(freed) {
            self.containing[slot] = None;
        }
        for &block in &changes.written.hoisting {
            for &hoisted in &boxes[block].hoisted {
                self.containing[hoisted] = Some(block);
            }
        }
    }

    /// Marks the slots `changes` names, and those up each one's chain;
    /// returns every slot marked.
    fn mark(&mut self, changes: &Changes<'_>) -> Vec<usize> {
        let written = &changes.written;
        let dirty = written.laid_out.iter().chain(&written.changed);
        let mut marked = Vec::new();
        for &slot in dirty.chain(changes.scrolled) {
            self.mark_up(slot, Mark::Dirty, &mut marked);
        }
        for &slot in changes.rebuilt {
            self.mark_up(slot, Mark::Rebuilt, &mut marked);
        }
        marked
    }

    /// Marks `slot` with `mark`, and the slots up its chain not yet
    /// marked with `Mark::Below`.
    fn mark_up(&mut self, slot: usize, mark: Mark, marked: &mut Vec<usize>) {
        let first = self.marks[slot] == Mark::None;
        if first {
            marked.push(slot);
        }
        self.marks[slot] = self.marks[slot].max(mark);
        let mut up = if first { self.up[slot] } else { NO_SLOT };
        while up != NO_SLOT && self.marks[up] == Mark::None {
            self.marks[up] = Mark::Below;
            marked.push(up);
            up = self.up[up];
        }
    }
}
