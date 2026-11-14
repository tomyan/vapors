//! Which scroll containers' thumbs an update paints again
//! (`paint_partial.rs`). A thumb is made from its container's border
//! and padding boxes, its clip, its offset and range, its scrollbar
//! style and whether thumbs are shown (`thumb.rs`). A container painted
//! again or shifted has a new box or clip, and a fresh part holds no
//! thumb; one whose offset or range changed at all (placement compares
//! them exactly, `ChangedSlots::scroll_states`) has moved thumbs; the
//! rest keep theirs. A page of thousands of small scrollers therefore
//! paints O(changed) thumbs per update, not every one.

use crate::changed_slots::ChangedSlots;
use crate::kept_box::BoxFrom;
use crate::kept_order::KeptOrder;
use crate::paint::PaintInput;

/// [`thumbs_again`] for an update painting `order` with `input`, whose
/// last paint showed thumbs as `shown_last`, or `None` when nothing was
/// kept.
pub(crate) fn of_update(
    order: &KeptOrder,
    painted: (&[(usize, BoxFrom)], &[usize]),
    changed: &ChangedSlots,
    (shown_last, input): (Option<bool>, &PaintInput<'_>),
) -> Vec<usize> {
    let every = shown_last != Some(input.thumbs_shown);
    let scrolls = |slot| input.stackings.scrolls(slot);
    thumbs_again(&order.thumbs, painted, changed, every, &scrolls)
}

/// The scroll containers whose thumbs this update paints, each once:
/// every one of `thumbs` when nothing was kept or thumbs were shown or
/// hidden (`every`); else those painted `again` or `shifted`, and those
/// whose scroll state `changed`, that `scrolls`. An empty answer
/// allocates nothing. A debug build panics, naming the slot, when one
/// chosen is not among `thumbs`: `scrolls` and paint order's thumb
/// steps must name the same containers (`frames_check.rs`).
pub(crate) fn thumbs_again(
    thumbs: &[usize],
    (again, shifted): (&[(usize, BoxFrom)], &[usize]),
    changed: &ChangedSlots,
    every: bool,
    scrolls: &dyn Fn(usize) -> bool,
) -> Vec<usize> {
    if every {
        return thumbs.to_vec();
    }
    let again = again.iter().map(|(slot, _)| *slot);
    let named = again.chain(shifted.iter().copied());
    let named = named.chain(changed.scroll_states.iter().copied());
    let mut slots: Vec<usize> = named.filter(|&slot| scrolls(slot)).collect();
    slots.sort_unstable();
    slots.dedup();
    if let Some(slot) = slots
        .iter()
        .find(|slot| cfg!(debug_assertions) && !thumbs.contains(slot))
    {
        panic!(
            "slot {slot} scrolls but has no thumb step in paint order: the stackings and paint \
             order disagree on the scroll containers (paint_order.rs, frames_check.rs); \
             reproduce with `cargo xtask test`"
        );
    }
    slots
}

#[cfg(test)]
#[path = "thumbs_again_tests.rs"]
mod tests;
