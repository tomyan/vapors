//! A rebuilt slot whose children differ from the box it replaced
//! (`rebuilt.rs`), as the order splice reads it (`order_splice.rs`): the
//! head and tail of children the two share, found by slice compares, and
//! the old children between them, so a change to a long child list
//! hands paint only the part that changed.

use super::boxes::LayoutBox;
use super::slots::Taken;

/// A slot whose box has other children than the box taken from it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Reshaped {
    pub(crate) slot: usize,
    /// The children at the start and at the end that are the same
    /// slots before and now.
    pub(crate) shared: Shared,
    /// The children the old box had between those.
    pub(crate) old_between: Vec<usize>,
}

/// The box at `slot` with the children it replaced, when they differ;
/// `None` when they are the same or the slot is newly taken. A kept key
/// keeps its slot, so equal children are the same boxes in the same
/// order.
///
/// # Panics
/// The slot's old children were moved out for a derived run (`put_edited`).
pub(super) fn reshaped(boxes: &[LayoutBox], slot: usize, taken: &Taken) -> Option<Reshaped> {
    let (_, old) = taken.boxes.get(&slot)?;
    assert!(
        !taken.moved_out.contains(&slot),
        "the owner in slot {slot} had its old children moved out, so comparing would read \
         them empty: its run is the one derived from its edits, read once from \
         `Taken::reshaped` (layout/rebuilt.rs `run_of`); reproduce with `cargo xtask test`"
    );
    let (old, new) = (&old.children, &boxes[slot].children);
    let shared = Shared::of(old, new);
    if shared.head == old.len() && old.len() == new.len() {
        return None;
    }
    let old_between = old[shared.head..old.len() - shared.tail].to_vec();
    Some(Reshaped {
        slot,
        shared,
        old_between,
    })
}

/// Panics, naming the slot, when `run`, derived from an owner's edits,
/// is not the run a compare of its old and new children finds (debug
/// builds keep the old children, `Taken::old_children`).
#[cfg(debug_assertions)]
pub(super) fn assert_derived(boxes: &[LayoutBox], run: &Reshaped, taken: &Taken) {
    let old = taken.old_children.get(&run.slot).unwrap_or_else(|| {
        panic!(
            "the edited owner in slot {} kept no old children to check",
            run.slot
        )
    });
    let new = &boxes[run.slot].children;
    let shared = Shared::of(old, new);
    let compared = Reshaped {
        slot: run.slot,
        shared,
        old_between: old[shared.head..old.len() - shared.tail].to_vec(),
    };
    assert_eq!(
        run, &compared,
        "the run of slot {}'s children its edits changed, derived from their places, differs \
         from a compare of its old and new children (layout/slots_edit_places.rs \
         `reshaped_by`); reproduce with `cargo xtask test`",
        run.slot
    );
}

/// How many children at the start and at the end the old and new lists
/// share, slot for slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shared {
    pub(crate) head: usize,
    pub(crate) tail: usize,
}

/// The slices compared at a time while the shared head and tail are
/// found: one comparison of 64 slots compiles to vector compares, where
/// one slot at a time costs a few instructions a slot.
const CHUNK: usize = 64;

impl Shared {
    /// The head and tail `old` and `new` share, not overlapping in
    /// either.
    pub(crate) fn of(old: &[usize], new: &[usize]) -> Self {
        let head = shared_len(old.iter(), new.iter(), |at| {
            old[at..at + CHUNK] == new[at..at + CHUNK]
        });
        let (old_rest, new_rest) = (&old[head..], &new[head..]);
        let (old_len, new_len) = (old_rest.len(), new_rest.len());
        let tail = shared_len(old_rest.iter().rev(), new_rest.iter().rev(), |back| {
            old_rest[old_len - back - CHUNK..old_len - back]
                == new_rest[new_len - back - CHUNK..new_len - back]
        });
        Self { head, tail }
    }
}

/// How many items `a` and `b` share from their start: whole chunks while
/// `chunk_equal(at)` holds for the chunk starting `at` items in, then one
/// item at a time.
fn shared_len<'s>(
    a: impl ExactSizeIterator<Item = &'s usize>,
    b: impl ExactSizeIterator<Item = &'s usize>,
    chunk_equal: impl Fn(usize) -> bool,
) -> usize {
    let len = a.len().min(b.len());
    let mut at = 0;
    while at + CHUNK <= len && chunk_equal(at) {
        at += CHUNK;
    }
    at + a
        .skip(at)
        .zip(b.skip(at))
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
#[path = "reshaped_tests.rs"]
mod tests;
