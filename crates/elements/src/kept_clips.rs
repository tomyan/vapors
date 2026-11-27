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

use crate::slot_list::SlotList;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::clip::BoxClip;
use crate::clip_refusals::RoundedRefusal;
use crate::clip_tree::ClipTree;
use crate::frames::PAGE;
use crate::layout::BoxRect;

/// What clips read of each box.
pub(crate) trait ClipInputs {
    /// What box `slot` gives its content, from `own`, its own clip, in a
    /// lane frame rounded by box `round`, if it is in one.
    fn inner(&self, slot: usize, own: Option<BoxClip>, round: Option<usize>) -> InnerClip;
    /// Whether box `slot` scrolls, so its content is a frame of its own.
    fn scrolls(&self, slot: usize) -> bool;
    /// Whether box `slot` is a lane child in a frame of its own: its own
    /// box and everything below it are in that frame
    /// (`docs/plans/splice-layout.md`, section 11).
    fn lane_frame(&self, slot: usize) -> bool;
}

/// What a box gives its content, as [`ClipInputs::inner`] works it out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InnerClip {
    /// The clip.
    pub(crate) clip: Option<BoxClip>,
    /// What a rounded clip refuses of the box (`clip_refusals.rs`).
    pub(crate) refused: Option<RoundedRefusal>,
    /// Its own padding box and radii, CSS pixels, when it rounds its
    /// content.
    pub(crate) rounding: Option<(BoxRect, [f32; 4])>,
    /// When it is a frame in a rounded lane frame, rounded by this box:
    /// checked against the corners after paint (`kept_clips_round.rs`).
    pub(crate) nested: Option<usize>,
}

/// The boxes' clip inputs.
pub(crate) type Inner<'a> = &'a dyn ClipInputs;

/// Each slot's own and inner clip, CSS pixels, `None` for none, and its
/// scroll frame.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Clips {
    own: SlotList<Option<BoxClip>>,
    inner: SlotList<Option<BoxClip>>,
    frame: SlotList<usize>,
    /// The slots a rounded clip refuses, and why: refused after paint
    /// (`clip_refusals.rs`).
    refused: FxHashMap<usize, RoundedRefusal>,
    /// Each box that rounds its content: its padding box and radii, CSS
    /// pixels in its content's frame, which hits are cut by.
    rounding: FxHashMap<usize, (BoxRect, [f32; 4])>,
    /// The frames in a rounded lane frame, and the box rounding it: each
    /// update checks them against its corners (`kept_clips_round.rs`).
    nested: FxHashMap<usize, usize>,
}

impl Clips {
    /// Each per-slot list's length and capacity, for the growth test.
    #[cfg(test)]
    pub(crate) fn slot_lists(&self) -> Vec<(&'static str, usize, usize)> {
        vec![
            self.own.named("clips.own"),
            self.inner.named("clips.inner"),
            self.frame.named("clips.frame"),
        ]
    }
    /// Every box's clips; the slots that redid.
    pub(crate) fn of(tree: &ClipTree, inner: Inner<'_>) -> (Self, Vec<usize>) {
        let mut clips = Self {
            own: vec![None; tree.len()].into(),
            inner: vec![None; tree.len()].into(),
            frame: vec![PAGE; tree.len()].into(),
            refused: FxHashMap::default(),
            rounding: FxHashMap::default(),
            nested: FxHashMap::default(),
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
        self.own.resize(len, None);
        self.inner.resize(len, None);
        self.frame.resize(len, PAGE);
        for slot in freed {
            (self.own[slot], self.inner[slot], self.frame[slot]) = (None, None, PAGE);
            self.refused.remove(&slot);
            self.rounding.remove(&slot);
            self.nested.remove(&slot);
        }
    }

    /// Clips of slots with these `inner` clips in these frames, for
    /// tests.
    #[cfg(test)]
    pub(crate) fn with(inner: Vec<Option<crate::layout::BoxRect>>, frame: Vec<usize>) -> Self {
        Self {
            own: vec![None; inner.len()].into(),
            inner: inner
                .into_iter()
                .map(|clip| clip.map(BoxClip::square))
                .collect::<Vec<_>>()
                .into(),
            frame: frame.into(),
            refused: FxHashMap::default(),
            rounding: FxHashMap::default(),
            nested: FxHashMap::default(),
        }
    }

    /// Box `slot`'s own clip: the one it paints under.
    pub(crate) fn own(&self, slot: usize) -> Option<BoxClip> {
        self.own[slot]
    }

    /// The slots the clips are kept for: one past the last.
    pub(crate) fn len(&self) -> usize {
        self.frame.len()
    }

    /// The padding box and radii, CSS pixels, of box `slot` when it
    /// rounds its content.
    pub(crate) fn rounding(&self, slot: usize) -> Option<(BoxRect, [f32; 4])> {
        self.rounding.get(&slot).copied()
    }

    /// Box `slot`'s inner clip: the one it gives its content.
    pub(crate) fn inner(&self, slot: usize) -> Option<BoxClip> {
        self.inner[slot]
    }

    /// Box `slot`'s scroll frame: the slot of the scroll container its
    /// part is painted in, or `PAGE`.
    pub(crate) fn frame(&self, slot: usize) -> usize {
        self.frame[slot]
    }

    /// Sets box `slot`'s frame, for tests of a broken frame chain.
    #[cfg(test)]
    pub(crate) fn set_frame(&mut self, slot: usize, frame: usize) {
        self.frame[slot] = frame;
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
        // its own, as a lane frame's own box does: the frame's clip is
        // applied on read (`frames.rs`).
        let lane_frame = inner.lane_frame(slot);
        let own = parent
            .filter(|&parent| !inner.scrolls(parent) && !lane_frame)
            .and_then(|parent| self.inner[parent]);
        let frame = match parent {
            _ if lane_frame => slot,
            Some(parent) => self.frame_inside(parent, inner),
            None => PAGE,
        };
        let round = self.round_frame(frame, tree, inner);
        let new = self.note(slot, inner.inner(slot, own, round));
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
            let get = |clips: &[Option<BoxClip>]| clips.get(slot).copied();
            (
                get(&clips.own),
                get(&clips.inner),
                clips.frame.get(slot).copied(),
                clips.refused.get(&slot).copied(),
                clips.rounding.get(&slot).copied(),
            )
        };
        // the refusals and roundings of slots past both lists' ends too
        let past = self.refused.keys().chain(other.refused.keys());
        let past = past
            .chain(self.rounding.keys())
            .chain(other.rounding.keys());
        let slots = past
            .map(|&slot| slot + 1)
            .fold(self.own.len().max(other.own.len()), usize::max);
        let slot = (0..slots).find(|&slot| at(self, slot) != at(other, slot))?;
        Some(format!(
            "clips at slot {slot}: kept (own, inner, frame, refused, rounding) {:?}, fresh {:?}",
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

#[path = "kept_clips_round.rs"]
mod round;
