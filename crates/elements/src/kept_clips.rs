//! Each box's clip kept by slot (IU3 3c-3c, `docs/plans/iu3.md`): the
//! clip it paints under (its own) and the clip it gives its content
//! (its inner), made for the whole tree when the shape changes, else
//! redone for each changed slot and, below it, for each box whose clip
//! comes from a box whose inner clip came out different. A changed
//! slot below one whose inner came out equal is redone as its own
//! start. Starts go shallowest first, so a box is redone after the box
//! its clip comes from.

use rustc_hash::FxHashSet;

use crate::clip_tree::ClipTree;
use crate::layout::BoxRect;

/// What a box gives its content, from its own clip: `inner(slot, own)`.
pub(crate) type Inner<'a> = &'a dyn Fn(usize, Option<BoxRect>) -> Option<BoxRect>;

/// Each slot's own and inner clip, CSS pixels, `None` for none.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Clips {
    own: Vec<Option<BoxRect>>,
    inner: Vec<Option<BoxRect>>,
}

impl Clips {
    /// Every box's clips; how many boxes that redid.
    pub(crate) fn of(tree: &ClipTree, inner: Inner<'_>) -> (Self, usize) {
        let mut clips = Self {
            own: vec![None; tree.len()],
            inner: vec![None; tree.len()],
        };
        let redone = match tree.len() {
            0 => 0,
            _ => clips.descend(0, tree, inner, &mut Redo::Every),
        };
        (clips, redone)
    }

    /// Box `slot`'s own clip: the one it paints under.
    pub(crate) fn own(&self, slot: usize) -> Option<BoxRect> {
        self.own[slot]
    }

    /// Redoes the clips of the `changed` slots and below them where an
    /// inner clip changed; how many boxes that redid.
    pub(crate) fn redo_under(
        &mut self,
        mut changed: Vec<usize>,
        tree: &ClipTree,
        inner: Inner<'_>,
    ) -> usize {
        changed.sort_unstable_by_key(|&slot| tree.depth(slot));
        let mut done = Redo::Changed(FxHashSet::default());
        changed
            .into_iter()
            .map(|slot| self.descend(slot, tree, inner, &mut done))
            .sum()
    }

    /// Redoes `start` and the boxes below it that `redo` reaches; how
    /// many.
    fn descend(
        &mut self,
        start: usize,
        tree: &ClipTree,
        inner: Inner<'_>,
        redo: &mut Redo,
    ) -> usize {
        let mut open = vec![start];
        let mut redone = 0;
        while let Some(slot) = open.pop() {
            if !redo.first_visit(slot) {
                continue;
            }
            redone += 1;
            if self.redo(slot, tree, inner) || matches!(redo, Redo::Every) {
                open.extend_from_slice(tree.dependents(slot));
            }
        }
        redone
    }

    /// Box `slot`'s clips worked out again from its clip parent's
    /// inner; whether its inner changed.
    fn redo(&mut self, slot: usize, tree: &ClipTree, inner: Inner<'_>) -> bool {
        let own = tree.parent(slot).and_then(|parent| self.inner[parent]);
        let new = inner(slot, own);
        self.own[slot] = own;
        std::mem::replace(&mut self.inner[slot], new) != new
    }

    /// The first slot whose clips differ from `other`'s, with both.
    pub(crate) fn first_differing(&self, other: &Self) -> Option<String> {
        let at = |clips: &Self, slot: usize| {
            (clips.own.get(slot).copied(), clips.inner.get(slot).copied())
        };
        let slots = self.own.len().max(other.own.len());
        let slot = (0..slots).find(|&slot| at(self, slot) != at(other, slot))?;
        Some(format!(
            "clips at slot {slot}: kept (own, inner) {:?}, fresh {:?}",
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
