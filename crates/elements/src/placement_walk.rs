//! The walk that places boxes again (IU3 3b, `placement.rs`): from the
//! root, into each marked slot and every slot placed in a box whose
//! content moved, by containing block as `chain::by_containing_block`
//! walks.

use crate::layout::{BoxRect, Content};
use crate::node_map::NodeSet;
use crate::placement::{LaidOut, Placed, Placement};
use crate::scroll::Offsets;

/// Why the walk visits a slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Mark {
    #[default]
    None,
    /// A slot below it is marked.
    Below,
    /// Laid out, given a new layout or scrolled: its scroll offset is
    /// settled again.
    Dirty,
    /// Built again: settled, its content taken as moved, and never
    /// reported as moved.
    Rebuilt,
}

pub(crate) struct Walk<'a, 'b> {
    kept: &'a mut Placement,
    boxes: &'a LaidOut<'b>,
    scroll: &'a mut Offsets,
    /// Nothing kept: every slot is placed as if built again.
    every: bool,
    /// The scroll containers settled, when `every`.
    settled: NodeSet,
    placed: Placed,
}

impl<'a, 'b> Walk<'a, 'b> {
    pub(crate) fn new(
        kept: &'a mut Placement,
        boxes: &'a LaidOut<'b>,
        scroll: &'a mut Offsets,
        every: bool,
    ) -> Self {
        Self {
            kept,
            boxes,
            scroll,
            every,
            settled: NodeSet::default(),
            placed: Placed {
                moved: Vec::new(),
                count: 0,
            },
        }
    }

    /// Places from the root, if there is one.
    pub(crate) fn place_from_root(&mut self) {
        if !self.boxes.boxes.is_empty() {
            self.visit(0, (0.0, 0.0), self.every);
        }
    }

    /// What the walk placed. When every slot was placed, the offsets of
    /// containers not settled are dropped.
    pub(crate) fn done(self) -> Placed {
        if self.every {
            self.scroll.keep_only(&self.settled);
        }
        self.placed
    }

    /// Places `slot` in content whose origin is `origin`, when its
    /// mark or `origin_moved` says to, and the slots placed in it.
    fn visit(&mut self, slot: usize, origin: (f32, f32), origin_moved: bool) {
        let mark = if self.every {
            Mark::Rebuilt
        } else {
            self.kept.marks[slot]
        };
        if !origin_moved && mark == Mark::None {
            return;
        }
        self.placed.count += 1;
        let before = content_origin(self.kept.rects[slot], self.kept.offsets[slot]);
        if mark >= Mark::Dirty {
            self.kept.offsets[slot] = self.settled_offset(slot);
        }
        self.put(slot, origin, mark);
        let inner = content_origin(self.kept.rects[slot], self.kept.offsets[slot]);
        self.visit_placed_in(slot, inner, mark == Mark::Rebuilt || inner != before);
    }

    /// Visits the slots placed in the content of `slot`, whose origin
    /// is `inner`: its children in flow, then the boxes it hoists.
    fn visit_placed_in(&mut self, slot: usize, inner: (f32, f32), moved: bool) {
        let boxes = self.boxes.boxes;
        for &child in &boxes[slot].children {
            if self.kept.containing[child].is_none() {
                self.kept.up[child] = slot;
                self.visit(child, inner, moved);
            }
        }
        for &child in &boxes[slot].hoisted {
            self.kept.up[child] = slot;
            self.visit(child, inner, moved);
        }
    }

    /// Writes the rect and padding of `slot`, noting it as moved when
    /// either changed and it was not built again.
    fn put(&mut self, slot: usize, origin: (f32, f32), mark: Mark) {
        let layout = &self.boxes.boxes[slot].layout;
        let rect = BoxRect {
            x: origin.0 + layout.location.x,
            y: origin.1 + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };
        let changed = rect != self.kept.rects[slot] || layout.padding != self.kept.paddings[slot];
        if changed && mark != Mark::Rebuilt {
            self.placed.moved.push(slot);
        }
        (self.kept.rects[slot], self.kept.paddings[slot]) = (rect, layout.padding);
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
        self.scroll
            .settle_box(node, (self.boxes.boxes, slot), self.boxes.scale)
    }
}

/// Where the content of a box at `rect` scrolled by `offset` starts.
fn content_origin(rect: BoxRect, offset: [f32; 2]) -> (f32, f32) {
    (rect.x - offset[0], rect.y - offset[1])
}
