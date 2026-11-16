//! Where each step's entry is in the extents index (`frame_extents.rs`),
//! by slot and step kind (`drawn_steps::kind`), not by its place in paint
//! order, which an insert would move (`docs/plans/paint-order-labels.md`).

use super::{At, Extents};
use crate::drawn_steps::kind;
use crate::paint_order::Step;

impl Extents {
    /// Where `step`'s entry is, if it draws.
    pub(super) fn at_of(&self, step: Step) -> Option<At> {
        self.at.get(step.index())?[kind(step)]
    }

    /// Where `step`'s entry is noted, or a panic naming it: the index is
    /// made for every slot of paint order (`of`, `patch`).
    pub(super) fn at_mut(&mut self, step: Step) -> &mut Option<At> {
        let slots = self.at.len();
        let at = self.at.get_mut(step.index()).unwrap_or_else(|| {
            panic!(
                "{step:?} is past the {slots} slots the extents index was made for: a slot was \
                 put in without paint order walked or spliced (paint_partial.rs \
                 drawn_updated); reproduce with `cargo xtask test`"
            )
        });
        &mut at[kind(step)]
    }
}
