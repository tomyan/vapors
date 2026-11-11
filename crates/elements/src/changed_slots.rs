//! The slots an update changed since the last paint (IU3 3c-3b,
//! `docs/plans/iu3.md` "Changed slots"): rebuilt and freed, as the box
//! build reports them (`layout/rebuilt.rs`); moved, by comparing each
//! slot's rect and padding with the last paint's, O(slots); and whether
//! the tree's shape changed. What paint keeps is patched from this one value.

use crate::kept_paint::PaintedFrom;
use crate::layout::{Key, Rebuilt};
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
    /// The kept slots whose rect or padding differs from the last
    /// paint's.
    pub(crate) moved: Vec<usize>,
    /// A slot was freed or newly taken, or a rebuilt box's children or
    /// `Stacking` differ: paint order, containing blocks and chains are
    /// walked again (`kept_order.rs`). `Stacking` holds what decides
    /// containing blocks, so a box made absolute or a containing block
    /// with its children unchanged is caught. Every box has an outline step, so whether a
    /// box has an outline is not compared.
    pub(crate) shape_changed: bool,
}

impl ChangedSlots {
    /// What `rebuilt` and this update's rects, paddings and `stackings`
    /// change since `last`.
    pub(crate) fn of(
        rebuilt: Rebuilt,
        last: Option<&Painted>,
        (now, stackings): (&PaintedFrom, &Stackings),
    ) -> Self {
        let last = last.filter(|_| !rebuilt.every);
        let Some(last) = last else {
            return Self::every(rebuilt);
        };
        Self {
            every: false,
            moved: moved(&last.from, now, &rebuilt),
            shape_changed: rebuilt.shape_changed || restacked(&rebuilt, last, stackings),
            rebuilt: rebuilt.slots,
            freed: rebuilt.freed,
        }
    }

    /// Every slot of `rebuilt` changed.
    fn every(rebuilt: Rebuilt) -> Self {
        Self {
            every: true,
            rebuilt: rebuilt.slots,
            freed: rebuilt.freed,
            moved: Vec::new(),
            shape_changed: true,
        }
    }

    /// Nothing kept, for a paint of every box afresh.
    pub(crate) fn all() -> Self {
        Self::every(Rebuilt::every(&[]))
    }

    /// How many boxes were built.
    pub(crate) fn built(&self) -> usize {
        self.rebuilt.len()
    }
}

/// Whether a rebuilt slot's stacking, containing block claims or
/// flow differ from `last`'s. A kept box's containing block changes
/// only through a rebuilt box: itself, or an ancestor whose mark
/// rebuilt the subtree.
fn restacked(rebuilt: &Rebuilt, last: &Painted, stackings: &Stackings) -> bool {
    rebuilt
        .slots
        .iter()
        .any(|&slot| last.stackings.at(slot) != stackings.at(slot))
}

/// The slots neither rebuilt nor freed whose rect or padding in `now`
/// differs from `last`'s.
fn moved(last: &PaintedFrom, now: &PaintedFrom, rebuilt: &Rebuilt) -> Vec<usize> {
    let len = now.rects.len();
    let mut changed = vec![false; len.max(last.rects.len())];
    let freed = rebuilt.freed.iter().map(|(slot, _)| slot);
    for &slot in rebuilt.slots.iter().chain(freed) {
        changed[slot] = true;
    }
    let differs = |slot: &usize| {
        let padding = last.paddings.get(*slot) != now.paddings.get(*slot);
        !changed[*slot] && (last.rects.get(*slot) != now.rects.get(*slot) || padding)
    };
    (0..len).filter(differs).collect()
}
