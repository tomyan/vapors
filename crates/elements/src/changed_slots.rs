//! The slots an update changed since the last paint (IU3 3c-3b,
//! `docs/plans/iu3.md` "Changed slots"): rebuilt and freed, as the box
//! build reports them (`layout/rebuilt.rs`); moved, by comparing each
//! slot's rect with the last paint's, O(slots); and whether the tree's
//! shape changed. What paint keeps is patched from this one value.

use crate::layout::{BoxRect, Key, Rebuilt};
use crate::paint_order::Stackings;
use crate::painted::Painted;

/// What changed, by slot, since the last paint.
#[derive(Debug)]
pub(crate) struct ChangedSlots {
    /// Nothing is kept: every box was built, or there is no last paint.
    pub(crate) every: bool,
    /// The slots a built box took.
    pub(crate) rebuilt: Vec<usize>,
    /// The slots whose box is gone, with the key each held.
    pub(crate) freed: Vec<(usize, Key)>,
    /// The inline elements of the text runs rebuilt or freed, whose
    /// rects paint puts back only for those still in a run.
    pub(crate) spans: Vec<vapors_dom::NodeId>,
    /// The kept slots whose rect differs from the last paint's.
    pub(crate) moved: Vec<usize>,
    /// A slot was freed or newly taken, or a rebuilt box's children or
    /// `Stacking` differ. Every box has an outline step today, so no
    /// outline flag is compared until paint order lists outlines only
    /// for boxes that have one (3c-3c).
    #[expect(
        dead_code,
        reason = "3c-3c's kept paint order reads it; untested until then"
    )]
    pub(crate) shape_changed: bool,
}

impl ChangedSlots {
    /// What `rebuilt` and this update's `rects` and `stackings` change
    /// since `last`.
    pub(crate) fn of(
        rebuilt: Rebuilt,
        last: Option<&Painted>,
        (rects, stackings): (&[BoxRect], &Stackings),
    ) -> Self {
        let last = last.filter(|_| !rebuilt.every);
        let Some(last) = last else {
            return Self::every(rebuilt);
        };
        Self {
            every: false,
            moved: moved(&last.from.rects, rects, &rebuilt),
            shape_changed: rebuilt.shape_changed || restacked(&rebuilt, last, stackings),
            rebuilt: rebuilt.slots,
            freed: rebuilt.freed,
            spans: rebuilt.spans,
        }
    }

    /// Every slot of `rebuilt` changed.
    fn every(rebuilt: Rebuilt) -> Self {
        Self {
            every: true,
            rebuilt: rebuilt.slots,
            freed: rebuilt.freed,
            spans: rebuilt.spans,
            moved: Vec::new(),
            shape_changed: true,
        }
    }

    /// How many boxes were built.
    pub(crate) fn built(&self) -> usize {
        self.rebuilt.len()
    }
}

/// Whether a rebuilt slot's stacking differs from `last`'s.
fn restacked(rebuilt: &Rebuilt, last: &Painted, stackings: &Stackings) -> bool {
    rebuilt
        .slots
        .iter()
        .any(|&slot| last.stackings.at(slot) != stackings.at(slot))
}

/// The slots neither rebuilt nor freed whose rect in `now` differs
/// from `last`'s.
fn moved(last: &[BoxRect], now: &[BoxRect], rebuilt: &Rebuilt) -> Vec<usize> {
    let mut changed = vec![false; now.len().max(last.len())];
    let freed = rebuilt.freed.iter().map(|(slot, _)| slot);
    for &slot in rebuilt.slots.iter().chain(freed) {
        changed[slot] = true;
    }
    let differs = |slot: &usize| !changed[*slot] && last.get(*slot) != now.get(*slot);
    (0..now.len()).filter(differs).collect()
}
