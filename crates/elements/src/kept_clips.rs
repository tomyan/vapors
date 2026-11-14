//! Each box's clip kept by slot (IU3 3c-3c, `docs/plans/iu3.md`): the
//! clip it paints under (its own) and the clip it gives its content
//! (its inner), made for the whole tree when the shape changes, else
//! redone for each changed slot and, below it, for each box whose clip
//! comes from a box whose inner clip came out different. A changed
//! slot below one whose inner came out equal is redone as its own
//! start. Starts go shallowest first, so a box is redone after the box
//! its clip comes from. Each box's scroll frame (`frames.rs`) is kept
//! beside its clips: its clip parent's when that does not scroll, else
//! the clip parent itself.

use crate::slot_list;
use rustc_hash::FxHashSet;

use crate::clip_tree::ClipTree;
use crate::frames::PAGE;
use crate::layout::BoxRect;

/// What clips read of each box.
pub(crate) trait ClipInputs {
    /// What box `slot` gives its content, from `own`, its own clip.
    fn inner(&self, slot: usize, own: Option<BoxRect>) -> Option<BoxRect>;
    /// Whether box `slot` scrolls, so its content is a frame of its own.
    fn scrolls(&self, slot: usize) -> bool;
}

/// The boxes' clip inputs.
pub(crate) type Inner<'a> = &'a dyn ClipInputs;

/// Each slot's own and inner clip, CSS pixels, `None` for none, and its
/// scroll frame.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Clips {
    own: Vec<Option<BoxRect>>,
    inner: Vec<Option<BoxRect>>,
    frame: Vec<usize>,
}

impl Clips {
    /// Each per-slot list's length and capacity, for the growth test.
    #[cfg(test)]
    pub(crate) fn slot_lists(&self) -> Vec<(&'static str, usize, usize)> {
        vec![
            ("clips.own", self.own.len(), self.own.capacity()),
            ("clips.inner", self.inner.len(), self.inner.capacity()),
            ("clips.frame", self.frame.len(), self.frame.capacity()),
        ]
    }
    /// Every box's clips; the slots that redid.
    pub(crate) fn of(tree: &ClipTree, inner: Inner<'_>) -> (Self, Vec<usize>) {
        let mut clips = Self {
            own: vec![None; tree.len()],
            inner: vec![None; tree.len()],
            frame: vec![PAGE; tree.len()],
        };
        let mut redone = Vec::new();
        if tree.len() > 0 {
            clips.descend(0, (tree, inner), &mut Redo::Every, &mut redone);
        }
        (clips, redone)
    }

    /// `self` for `len` slots, the `freed` ones' clips cleared: an order
    /// splice (`order_splice.rs`); the added slots are redone after.
    pub(crate) fn spliced(&mut self, len: usize, freed: impl Iterator<Item = usize>) {
        slot_list::resize(&mut self.own, len, None);
        slot_list::resize(&mut self.inner, len, None);
        slot_list::resize(&mut self.frame, len, PAGE);
        for slot in freed {
            (self.own[slot], self.inner[slot], self.frame[slot]) = (None, None, PAGE);
        }
    }

    /// Clips of slots with these `inner` clips in these frames, for
    /// tests.
    #[cfg(test)]
    pub(crate) fn with(inner: Vec<Option<BoxRect>>, frame: Vec<usize>) -> Self {
        Self {
            own: vec![None; inner.len()],
            inner,
            frame,
        }
    }

    /// Box `slot`'s own clip: the one it paints under.
    pub(crate) fn own(&self, slot: usize) -> Option<BoxRect> {
        self.own[slot]
    }

    /// The slots the clips are kept for: one past the last.
    pub(crate) fn len(&self) -> usize {
        self.frame.len()
    }

    /// Box `slot`'s inner clip: the one it gives its content.
    pub(crate) fn inner(&self, slot: usize) -> Option<BoxRect> {
        self.inner[slot]
    }

    /// Box `slot`'s scroll frame: the slot of the scroll container its
    /// part is painted in, or `PAGE`.
    pub(crate) fn frame(&self, slot: usize) -> usize {
        self.frame[slot]
    }

    /// Redoes the clips of the `changed` slots and below them where an
    /// inner clip changed; the slots that redid.
    pub(crate) fn redo_under(
        &mut self,
        mut changed: Vec<usize>,
        tree: &ClipTree,
        inner: Inner<'_>,
    ) -> Vec<usize> {
        changed.sort_unstable_by_key(|&slot| tree.depth(slot));
        let mut done = Redo::Changed(FxHashSet::default());
        let mut redone = Vec::new();
        for slot in changed {
            self.descend(slot, (tree, inner), &mut done, &mut redone);
        }
        redone
    }

    /// Redoes `start` and the boxes below it that `redo` reaches,
    /// noting each in `redone`.
    fn descend(
        &mut self,
        start: usize,
        (tree, inner): (&ClipTree, Inner<'_>),
        redo: &mut Redo,
        redone: &mut Vec<usize>,
    ) {
        let mut open = vec![start];
        while let Some(slot) = open.pop() {
            if self.visit(slot, (tree, inner), redo, &mut open) {
                redone.push(slot);
            }
        }
    }

    /// Redoes `slot` unless `redo` already has, and opens its
    /// dependents where `redo` reaches them; whether it was redone.
    fn visit(
        &mut self,
        slot: usize,
        (tree, inner): (&ClipTree, Inner<'_>),
        redo: &mut Redo,
        open: &mut Vec<usize>,
    ) -> bool {
        if !redo.first_visit(slot) {
            return false;
        }
        if self.redo(slot, tree, inner) || matches!(redo, Redo::Every) {
            open.extend_from_slice(tree.dependents(slot));
        }
        true
    }

    /// Box `slot`'s clips and frame worked out again from its clip
    /// parent's; whether its inner clip or its frame changed, either of
    /// which its dependents take. A box that starts or stops scrolling
    /// is restacked, so paint order is walked again and its clips made
    /// by `Clips::of`, never redone here.
    fn redo(&mut self, slot: usize, tree: &ClipTree, inner: Inner<'_>) -> bool {
        let parent = tree.parent(slot);
        // A scroll container's content starts a frame with no clip of
        // its own: the frame's clip is applied on read (`frames.rs`).
        let own = parent
            .filter(|&parent| !inner.scrolls(parent))
            .and_then(|parent| self.inner[parent]);
        let frame = parent.map_or(PAGE, |parent| self.frame_inside(parent, inner));
        let new = inner.inner(slot, own);
        self.own[slot] = own;
        let was = (self.inner[slot], self.frame[slot]);
        (self.inner[slot], self.frame[slot]) = (new, frame);
        was != (new, frame)
    }

    /// The frame of the content of box `slot`: its own when it scrolls,
    /// else the one it is in.
    fn frame_inside(&self, slot: usize, inner: Inner<'_>) -> usize {
        if inner.scrolls(slot) {
            slot
        } else {
            self.frame[slot]
        }
    }

    /// The first slot whose clips differ from `other`'s, with both.
    pub(crate) fn first_differing(&self, other: &Self) -> Option<String> {
        let at = |clips: &Self, slot: usize| {
            let get = |clips: &[Option<BoxRect>]| clips.get(slot).copied();
            (
                get(&clips.own),
                get(&clips.inner),
                clips.frame.get(slot).copied(),
            )
        };
        let slots = self.own.len().max(other.own.len());
        let slot = (0..slots).find(|&slot| at(self, slot) != at(other, slot))?;
        Some(format!(
            "clips at slot {slot}: kept (own, inner, frame) {:?}, fresh {:?}",
            at(self, slot),
            at(other, slot)
        ))
    }
}

/// Which boxes a descent redoes.
enum Redo {
    /// Every box below the start, once.
    Every,
    /// Each box once per update, where its clip parent's inner changed;
    /// the boxes already redone.
    Changed(FxHashSet<usize>),
}

impl Redo {
    /// Whether `slot` is redone now: not yet redone this update.
    fn first_visit(&mut self, slot: usize) -> bool {
        match self {
            Self::Every => true,
            Self::Changed(done) => done.insert(slot),
        }
    }
}
