//! The slots an update changed since the last paint (IU3 3c-3b,
//! `docs/plans/iu3.md` "Changed slots"): rebuilt and freed, as the box
//! build reports them (`layout/rebuilt.rs`); moved, as placement
//! reports them (`placement.rs`, IU3 3b); and whether the tree's shape
//! changed. What paint keeps is patched from this one value.

use crate::layout::{Key, Rebuilt};
use crate::painted::Painted;
use crate::stacking::Patched;

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
    /// paint's, as placement reports them; none until it runs.
    pub(crate) moved: Vec<usize>,
    /// A slot was freed or newly taken, or a rebuilt box's children or
    /// `Stacking` differ: paint order, containing blocks and chains are
    /// spliced (`order_splice.rs`) or walked again (`kept_order.rs`).
    /// `Stacking` holds what decides containing blocks, so a box made
    /// absolute or a containing block with its children unchanged is
    /// caught. Every box has an outline step, so whether a box has an
    /// outline is not compared.
    pub(crate) shape_changed: bool,
    /// The rebuilt slots whose children differ from the box each
    /// replaced, with the children it had: what the order splice reads.
    pub(crate) reshaped: Vec<(usize, Vec<usize>)>,
    /// What patching the stackings found: a rebuilt slot's stacking
    /// changed, or a freed slot held one other than the default.
    pub(crate) stackings: Patched,
}

impl ChangedSlots {
    /// What `rebuilt` changes since `last`, moved slots aside, and what
    /// patching the `stackings` found (`Stackings::patched`).
    pub(crate) fn of(rebuilt: Rebuilt, last: Option<&Painted>, stackings: Patched) -> Self {
        if rebuilt.every || last.is_none() {
            return Self::every(rebuilt);
        }
        Self {
            every: false,
            moved: Vec::new(),
            shape_changed: rebuilt.shape_changed || stackings.restacked,
            rebuilt: rebuilt.slots,
            freed: rebuilt.freed,
            reshaped: rebuilt.reshaped,
            stackings,
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
            reshaped: Vec::new(),
            stackings: Patched {
                restacked: true,
                freed_stacked: true,
            },
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
