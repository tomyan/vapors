//! A block lane on a container's box slot (`docs/plans/splice-layout.md`,
//! section 2): the sum tree over its children's leaves, kept between
//! updates beside the boxes (`Placed::lanes`, by the container's slot),
//! edited where the container's box children were edited in place
//! (`slots_edit.rs`) and dropped where its box was built again
//! (`slots_subtree.rs`), to be made afresh from the children at the
//! next layout. `lane_layout.rs` fills it.

use rustc_hash::FxHashMap;
use taffy::{AlignItems, Direction, Layout, Line, Rect, Size, TextAlign};

use super::lane_block::Container;
use super::lane_block_summary::BlockSummary;
use super::lane_tree::{Handle, LaneTree};

/// The lanes beside the boxes, by the container's slot.
pub(crate) type Lanes = FxHashMap<usize, Lane>;

/// One container's lane.
#[derive(Debug)]
pub(crate) struct Lane {
    pub(super) tree: LaneTree<BlockSummary>,
    /// By child slot, its handle; and back.
    pub(super) handles: FxHashMap<usize, Handle>,
    pub(super) slots: FxHashMap<Handle, usize>,
    /// What the leaves were made with; every leaf is made again when
    /// it changes.
    pub(super) share: Option<Share>,
    /// Children put in since the last layout, each after the child in
    /// the slot given, or first.
    pub(super) inserted: Vec<(usize, Option<usize>)>,
    /// Children whose start may have changed since positions were last
    /// written: put in, laid out again, or after a removed one.
    pub(super) moved: Vec<usize>,
    /// Layouts of children laid out since positions were last written,
    /// their `location.y` the relative offset only.
    pub(super) pending: FxHashMap<usize, Layout>,
    /// By child slot, its `position: relative` offset along the axis,
    /// where it has one.
    pub(super) relative: FxHashMap<usize, f32>,
    /// Whether every child must be laid out again and every start
    /// written: a fresh lane, or one whose share changed.
    pub(super) all: bool,
}

impl Default for Lane {
    fn default() -> Self {
        Self {
            tree: LaneTree::new(0.0),
            handles: FxHashMap::default(),
            slots: FxHashMap::default(),
            share: None,
            inserted: Vec::new(),
            moved: Vec::new(),
            pending: FxHashMap::default(),
            relative: FxHashMap::default(),
            all: true,
        }
    }
}

/// What every child's leaf depends on besides the child: the
/// container's inputs the lane lays children out with.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Share {
    inner_width: f32,
    parent_size: Size<Option<f32>>,
    item_basis: Size<Option<f32>>,
    content_box_inset: Rect<f32>,
    resolved_inset: Rect<f32>,
    resolved_border: Rect<f32>,
    outer_width: f32,
    direction: Direction,
    text_align: TextAlign,
    justify_items: AlignItems,
    own_margins_collapse: Line<bool>,
    children_adjoin: Line<bool>,
    scrolls: bool,
}

impl Container {
    /// What its children's leaves depend on.
    pub(crate) fn share(&self) -> Share {
        Share {
            inner_width: self.inner_width,
            parent_size: self.parent_size,
            item_basis: self.item_basis,
            content_box_inset: self.content_box_inset,
            resolved_inset: self.resolved_inset,
            resolved_border: self.resolved_border,
            outer_width: self.outer_width,
            direction: self.direction,
            text_align: self.text_align,
            justify_items: self.justify_items,
            own_margins_collapse: self.own_margins_collapse,
            children_adjoin: self.children_adjoin,
            scrolls: self.scrolls,
        }
    }
}

impl Lane {
    /// Notes the child in `slot` put in after the child in `after`'s
    /// slot, or first; its leaf is made at the next layout.
    pub(crate) fn child_inserted(&mut self, slot: usize, after: Option<usize>) {
        self.inserted.push((slot, after));
    }

    /// Takes the child in `slot` out; `next`, the child now in its
    /// place, starts elsewhere.
    pub(crate) fn child_removed(&mut self, slot: usize, next: Option<usize>) {
        if let Some(handle) = self.handles.remove(&slot) {
            self.slots.remove(&handle);
            self.tree.remove(handle);
        }
        self.moved.extend(next);
        self.pending.remove(&slot);
        self.relative.remove(&slot);
    }

    /// The child in `slot`'s layout to write, and its relative offset:
    /// the layout laid out since the last write, whose `location.y` is
    /// that offset, else the one it holds, with the offset noted then.
    pub(super) fn take_write(&mut self, slot: usize, held: Layout) -> (Layout, f32) {
        match self.pending.remove(&slot) {
            Some(layout) => {
                let relative = layout.location.y;
                if relative == 0.0 {
                    self.relative.remove(&slot);
                } else {
                    self.relative.insert(slot, relative);
                }
                (layout, relative)
            }
            None => (held, self.relative.get(&slot).copied().unwrap_or(0.0)),
        }
    }

    /// Puts a leaf for the child in `slot` after `previous`.
    pub(super) fn put(
        &mut self,
        slot: usize,
        previous: Option<Handle>,
        leaf: BlockSummary,
    ) -> Handle {
        let handle = self.tree.insert_after(previous, leaf);
        self.handles.insert(slot, handle);
        self.slots.insert(handle, slot);
        self.moved.push(slot);
        handle
    }

    /// The handle of the child in `slot`.
    ///
    /// # Panics
    /// No child of the lane holds `slot`: an edit named a child the
    /// lane never had (`child_inserted` with a slot `put_edited` did
    /// not put).
    pub(super) fn handle(&self, slot: usize) -> Handle {
        *self.handles.get(&slot).unwrap_or_else(|| {
            panic!("slot {slot} is not a child of this lane: `child_inserted` named a sibling the lane never had")
        })
    }
}
