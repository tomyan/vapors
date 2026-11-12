//! The walk that places boxes again (IU3 3b, `placement.rs`): from the
//! root, into each marked slot (`placement_marks.rs`) and every slot
//! placed in a box whose content moved, by containing block as
//! `chain::by_containing_block` walks.

use rustc_hash::FxHashMap;

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
    /// Scrolled, and nothing else: its scroll offset is settled again,
    /// and the slots placed in its content move by the scroll alone.
    Scrolled,
    /// Laid out or given a new layout: its scroll offset is settled
    /// again.
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
    /// The marked slots placed in each slot (`placement_marks.rs`).
    under: &'a FxHashMap<usize, Vec<usize>>,
    placed: Placed,
}

impl<'a, 'b> Walk<'a, 'b> {
    pub(crate) fn new(
        kept: &'a mut Placement,
        boxes: &'a LaidOut<'b>,
        scroll: &'a mut Offsets,
        every: bool,
        under: &'a FxHashMap<usize, Vec<usize>>,
    ) -> Self {
        Self {
            kept,
            boxes,
            scroll,
            every,
            under,
            settled: NodeSet::default(),
            placed: Placed {
                moved: Vec::new(),
                scrolled: Vec::new(),
                count: 0,
                chained: 0,
            },
        }
    }

    /// Places from the root, if there is one and it is marked, or every
    /// slot is placed.
    pub(crate) fn place_from_root(&mut self) {
        let held = !self.boxes.boxes.is_empty();
        if held && (self.every || self.kept.marks[0] != Mark::None) {
            self.visit(0, (0.0, 0.0), self.every, false);
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
    /// mark or `origin_moved` says to, and the slots placed in it. The
    /// mark is cleared as it is read, so `Placement::place` can tell a
    /// marked slot the walk did not reach. `by_scroll`: the origin
    /// moved by scrolls alone, so a slot not laid out again moved by
    /// them alone too.
    fn visit(&mut self, slot: usize, origin: (f32, f32), origin_moved: bool, by_scroll: bool) {
        let mark = if self.every {
            Mark::Rebuilt
        } else {
            std::mem::replace(&mut self.kept.marks[slot], Mark::None)
        };
        assert!(
            origin_moved || mark != Mark::None,
            "the placement walk visited slot {slot}, which is not marked and is placed in \
             content that did not move: the walk scans a box's every child, or the marked \
             slots under each slot (placement_marks.rs) list one that is not marked; \
             `cargo test -p vapors-elements` reproduces it"
        );
        self.placed.count += 1;
        let before = content_origin(self.kept.rects[slot], self.kept.offsets[slot]);
        if mark >= Mark::Scrolled {
            self.kept.offsets[slot] = self.settled_offset(slot);
        }
        let kept_layout = mark <= Mark::Scrolled;
        self.put(slot, origin, mark, by_scroll && kept_layout);
        let inner = content_origin(self.kept.rects[slot], self.kept.offsets[slot]);
        // its content moves by its own move and its scroll: by scrolls
        // alone when its layout is kept and its own move was
        let placed_by_scroll = kept_layout && (!origin_moved || by_scroll);
        let moved = mark == Mark::Rebuilt || inner != before;
        self.visit_placed_in(slot, inner, moved, placed_by_scroll);
    }

    /// Visits the slots placed in the content of `slot`, whose origin
    /// is `inner`: when the content `moved`, its children in flow, then
    /// the boxes it hoists; else only the marked slots among them.
    fn visit_placed_in(&mut self, slot: usize, inner: (f32, f32), moved: bool, by_scroll: bool) {
        if !moved {
            self.visit_marked_in(slot, inner);
            return;
        }
        let boxes = self.boxes.boxes;
        for &child in &boxes[slot].children {
            if self.kept.containing[child].is_none() {
                self.kept.up[child] = slot;
                self.visit(child, inner, moved, by_scroll);
            }
        }
        for &child in &boxes[slot].hoisted {
            self.kept.up[child] = slot;
            self.visit(child, inner, moved, by_scroll);
        }
    }

    /// Visits the marked slots placed in `slot`, whose content did not
    /// move; each is chained to it already (`placement_marks.rs`).
    fn visit_marked_in(&mut self, slot: usize, inner: (f32, f32)) {
        let under = self.under;
        for &child in under.get(&slot).map_or(&[][..], Vec::as_slice) {
            assert!(
                self.kept.marks[child] != Mark::None,
                "slot {child}, listed under slot {slot} as marked, holds no mark: it was \
                 visited already, or listed without being marked (placement_marks.rs); \
                 `cargo test -p vapors-elements` reproduces it"
            );
            self.visit(child, inner, false, false);
        }
    }

    /// Writes the rect and padding of `slot`, noting it as moved when
    /// either changed and it was not built again: as scrolled when
    /// `by_scroll`, else as moved.
    fn put(&mut self, slot: usize, origin: (f32, f32), mark: Mark, by_scroll: bool) {
        let layout = &self.boxes.boxes[slot].layout;
        let rect = BoxRect {
            x: origin.0 + layout.location.x,
            y: origin.1 + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };
        let changed = rect != self.kept.rects[slot] || layout.padding != self.kept.paddings[slot];
        if changed && mark != Mark::Rebuilt {
            if by_scroll {
                self.placed.scrolled.push(slot);
            } else {
                self.placed.moved.push(slot);
            }
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
