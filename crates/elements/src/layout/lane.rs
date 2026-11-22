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
use super::lane_tree::{Handle, HandleSlots, LaneTree};
use crate::lane_decline::LaneDecline;

/// The fewest children a lane frames (`Lane::framing`): enough that
/// framing pays for its frames, set from no measurement yet; the rows
/// page's list has thousands, a row's cells a handful.
pub(super) const FRAMED_CHILDREN: usize = 32;

/// How far down its scroll frame a lane's children may start before it
/// frames them whatever their number (`Lane::framing`): 2^16 CSS px,
/// below which an f32 corner resolves 1/128 px; past it a child's
/// boxes, placed in the scroll frame, lose fractions of a pixel (at 2^25
/// an f32 steps by 4 px; a million 52 px rows reach 52,000,000).
pub(super) const FRAMED_DEPTH: f64 = 65_536.0;

/// One container's lane.
#[derive(Debug)]
pub(crate) struct Lane<K: LaneKind> {
    pub(super) tree: LaneTree<K::Summary>,
    /// By child slot, its handle; and back.
    pub(super) handles: FxHashMap<usize, Handle>,
    pub(super) slots: HandleSlots,
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
    /// Children whose start may have changed since layouts were last
    /// written: put in, laid out again, or after a removed one.
    pub(super) moved: Vec<usize>,
    /// Layouts of children laid out since layouts were last written,
    /// their `location.y` the relative offset only.
    pub(super) pending: FxHashMap<usize, Layout>,
    /// By child slot, its `position: relative` offset along the axis,
    /// where it has one.
    pub(super) relative: FxHashMap<usize, f32>,
    /// Whether every child must be laid out again and its layout
    /// written: a fresh lane, or one whose share changed.
    pub(super) all: bool,
    /// The share and height bases the children's layouts were last
    /// written with; `None` before the first write. The tree holds the
    /// leaves of that write (`lane_aside.rs`).
    pub(super) written: Option<(K::Share, K::Heights)>,
    /// Each child's start, its lane's top added, as last written: what
    /// the debug check holds the tree to (`lane_written_check.rs`).
    #[cfg(debug_assertions)]
    pub(super) written_starts: FxHashMap<usize, f64>,
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
            slots: HandleSlots::default(),
            share: None,
            heights: None,
            height_readers: FxHashSet::default(),
            inserted: Vec::new(),
            moved: Vec::new(),
            pending: FxHashMap::default(),
            relative: FxHashMap::default(),
            all: true,
            written: None,
            #[cfg(debug_assertions)]
            written_starts: FxHashMap::default(),
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
            self.slots.remove(handle);
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

    /// The share and height bases of the lane's last write; `None` before
    /// its first. The one answer to whether this lane wrote its
    /// children's layouts: framing, `written_start`, the aside answer
    /// and the debug check read it.
    pub(super) fn written_bases(&self) -> Option<&(K::Share, K::Heights)> {
        self.written.as_ref()
    }

    /// Whether this lane wrote its children's layouts (`written_bases`).
    pub(super) fn has_written(&self) -> bool {
        self.written_bases().is_some()
    }

    /// Notes the child in `slot`'s `position: relative` offset along
    /// the axis, `relative`, as its layout is written.
    pub(super) fn note_relative(&mut self, slot: usize, relative: f32) {
        if relative == 0.0 {
            self.relative.remove(&slot);
        } else {
            self.relative.insert(slot, relative);
        }
    }

    /// The children from the one in `slot` on, by slot, in order.
    pub(super) fn children_from(&self, slot: usize) -> Vec<usize> {
        let Some(&handle) = self.handles.get(&slot) else {
            return Vec::new();
        };
        let handles = self.tree.handles_from(handle);
        handles.iter().map(|handle| self.slots[handle]).collect()
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
    /// lane of fewer than `FRAMED_CHILDREN` children none of which starts
    /// `FRAMED_DEPTH` down its scroll frame, which keeps its
    /// children placed as before: placing a few costs little, and every
    /// framed lane is worked out once per update (`paint_frames.rs`), so
    /// framing the small lanes too (a cell holding its text is one)
    /// would cost O(rows) per update again.
    pub(super) fn framing(&self, origin: f64, scale: f32) -> Option<Result<(), LaneDecline>> {
        let many = self.handles.len() >= FRAMED_CHILDREN;
        let framable = self.has_written() && (many || self.reaches_depth(origin));
        framable.then(|| self.facts.framable(self.tree.gap, origin, scale))
    }

    /// Whether a child starts `FRAMED_DEPTH` or more down the scroll
    /// frame, the container's content at `origin`: such a lane frames
    /// its children however few, so their boxes are placed in frames of
    /// their own, near zero, and shifted in f64. Read only for a lane of
    /// fewer than `FRAMED_CHILDREN`, so the scan of its starts is short.
    ///
    /// RELAXED: a deep lane that cannot frame (`framable` declines it,
    /// `LaneFramePixel` for a fractional start or a scale that makes one,
    /// such as 52 px rows at 1.1) is placed in f32 scroll-frame
    /// coordinates as before, off by up to 4 px at 52,000,000 px, and
    /// only counted as a decline (`Placed::unframed`), not refused.
    /// Placement keeping deep rects exact without a lane frame (f64
    /// scroll-frame coordinates) retires this.
    fn reaches_depth(&self, origin: f64) -> bool {
        let starts = self.handles.values().map(|&handle| self.tree.start(handle));
        let last = starts.fold(0.0, f64::max);
        origin + self.facts.top + last >= FRAMED_DEPTH
    }

    /// The children whose box or overflow meets `from..=to`, CSS pixels
    /// from the container's border box top, by slot, each with its start
    /// from the lane's top plus that top (as `frame_offset` gives it).
    pub(super) fn children_between(&self, from: f64, to: f64) -> Vec<(usize, f64)> {
        let top = self.facts.top;
        let starts = self.tree.between_starts(from - top, to - top);
        starts
            .into_iter()
            .map(|(handle, start)| (self.slots[&handle], top + start))
            .collect()
    }

    /// Whether the lane holds the child in `slot`.
    pub(super) fn holds(&self, slot: usize) -> bool {
        self.handles.contains_key(&slot)
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
