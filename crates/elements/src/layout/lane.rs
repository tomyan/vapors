//! A block or flex lane on a container's box slot (`docs/plans/splice-layout.md`,
//! section 2): the sum tree over its children's leaves, kept between
//! updates beside the boxes (`Placed::lanes`, by the container's slot),
//! edited where the container's box children were edited in place
//! (`slots_edit.rs`) and dropped where its box was built again
//! (`slots_subtree.rs`), to be made afresh from the children at the
//! next layout. `lane_layout.rs` fills it.

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use taffy::Layout;

use super::lane_frame::FrameFacts;
use super::lane_kind::LaneKind;
use super::lane_tree::{Handle, LaneTree};
use crate::lane_decline::LaneDecline;

/// The fewest children a lane frames (`Lane::framing`): enough that
/// framing pays for its frames, set from no measurement yet; the rows
/// page's list has thousands, a row's cells a handful.
pub(super) const FRAMED_CHILDREN: usize = 32;

/// One container's lane.
#[derive(Debug)]
pub(crate) struct Lane<K: LaneKind> {
    pub(super) tree: LaneTree<K::Summary>,
    /// By child slot, its handle; and back.
    pub(super) handles: FxHashMap<usize, Handle>,
    pub(super) slots: FxHashMap<Handle, usize>,
    /// What the leaves were made with; every leaf is made again when
    /// it changes.
    pub(super) share: Option<K::Share>,
    /// The height bases the leaves were made with, and the children
    /// whose styles read them; only those are made again when the
    /// bases change, as they do between a flex parent's size query and
    /// its layout.
    pub(super) heights: Option<K::Heights>,
    pub(super) height_readers: FxHashSet<usize>,
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
    /// What framing the children reads (`lane_frame.rs`).
    pub(crate) facts: FrameFacts,
    /// Children put since the lane was last put back among the lanes,
    /// whose container `Lanes` then notes.
    pub(super) born: Vec<usize>,
}

impl<K: LaneKind> Lane<K> {
    /// An empty lane whose children sit `gap` apart, every child to be
    /// laid out at its first layout.
    pub(super) fn new(gap: f64) -> Self {
        Self {
            tree: LaneTree::new(gap),
            handles: FxHashMap::default(),
            slots: FxHashMap::default(),
            share: None,
            heights: None,
            height_readers: FxHashSet::default(),
            inserted: Vec::new(),
            moved: Vec::new(),
            pending: FxHashMap::default(),
            relative: FxHashMap::default(),
            all: true,
            facts: FrameFacts::default(),
            born: Vec::new(),
        }
    }

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
        self.moved.retain(|&moved| moved != slot);
        self.inserted.retain(|&(inserted, _)| inserted != slot);
        self.moved.extend(next);
        self.pending.remove(&slot);
        self.relative.remove(&slot);
        self.height_readers.remove(&slot);
        self.facts.forget(slot);
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
        leaf: K::Summary,
    ) -> Handle {
        let handle = self.tree.insert_after(previous, leaf);
        self.handles.insert(slot, handle);
        self.slots.insert(handle, slot);
        self.moved.push(slot);
        self.born.push(slot);
        handle
    }

    /// Whether the children can be frames of their own; `None` for a
    /// lane of fewer than `FRAMED_CHILDREN` children, which keeps its
    /// children placed as before: placing a few costs little, and every
    /// framed lane is worked out once per update (`paint_frames.rs`), so
    /// framing the small lanes too (a cell holding its text is one)
    /// would cost O(rows) per update again.
    pub(super) fn framing(&self, origin: f64, scale: f32) -> Option<Result<(), LaneDecline>> {
        (self.handles.len() >= FRAMED_CHILDREN)
            .then(|| self.facts.framable(self.tree.gap, origin, scale))
    }

    /// The children whose box or overflow meets `from..=to`, CSS pixels
    /// from the container's border box top, by slot.
    pub(super) fn children_between(&self, from: f64, to: f64) -> Vec<usize> {
        let top = self.facts.top;
        let handles = self.tree.between(from - top, to - top);
        handles
            .into_iter()
            .map(|handle| self.slots[&handle])
            .collect()
    }

    /// The child in `slot`'s start from the lane's top, plus that top.
    pub(super) fn frame_offset(&self, slot: usize) -> Option<f64> {
        let handle = *self.handles.get(&slot)?;
        Some(self.facts.top + self.tree.start(handle))
    }

    /// The moved child nearest the start, whose start and every later
    /// one is written; none when nothing moved.
    pub(super) fn first_moved(&self) -> Option<Handle> {
        self.moved
            .iter()
            .map(|&slot| self.handle(slot))
            .map(|handle| (handle, self.tree.start(handle)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(handle, _)| handle)
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

#[cfg(test)]
#[path = "lane_tests.rs"]
mod tests;
