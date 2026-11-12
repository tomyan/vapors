//! The debug build's check on a kept placement (`placement.rs`): every
//! box settled and placed afresh with `Offsets::settle` and
//! `layout::placed`, and compared slot by slot with what placement kept,
//! the offsets by element, and the moved slots with a comparison of
//! every held slot's rect and padding with the last paint's. The chain
//! the next update's walk follows (`up`, `containing`) is compared too,
//! so a broken chain fails on the update that broke it. Rects are kept
//! in scroll-frame coordinates (IU4), so they are compared with a fresh
//! placement at no scroll offset, and each slot's rect on the page,
//! snapped as paint snaps it and shifted by its frames' rounded offsets
//! (`frames.rs`), with the snapped rect of today's placement with the
//! offsets applied: what the page shows.

use rustc_hash::FxHashSet;

use crate::changed_slots::ChangedSlots;
use crate::layout::{BoxRect, Content, LayoutBox, containing_blocks, placed};
use crate::placement::{LaidOut, NO_SLOT, Placed, Placement};
use crate::scroll::Offsets;

/// What placement started from: the offsets, rects, paddings and each
/// slot's applied offset.
pub(crate) struct Before {
    scroll: Offsets,
    rects: Vec<BoxRect>,
    paddings: Vec<taffy::Rect<f32>>,
    offsets: Vec<[f32; 2]>,
}

/// A fresh placement: each slot's rect in its frame, padding, offset,
/// and rect on the page.
struct Fresh {
    rects: Vec<BoxRect>,
    paddings: Vec<taffy::Rect<f32>>,
    offsets: Vec<[f32; 2]>,
    on_page: Vec<BoxRect>,
}

impl Before {
    /// `scroll` and `kept` before placement.
    pub(crate) fn of(scroll: &Offsets, kept: &Placement) -> Self {
        Self {
            scroll: scroll.clone(),
            rects: kept.rects.clone(),
            paddings: kept.paddings.clone(),
            offsets: kept.offsets.clone(),
        }
    }
}

impl Fresh {
    /// `boxes` settled from `before`'s offsets and placed afresh, and
    /// the offsets by element after.
    fn of(before: &Before, boxes: &LaidOut<'_>) -> (Self, Offsets) {
        let mut scroll = before.scroll.clone();
        let offsets = scroll.settle(boxes.boxes, boxes.scrolls, boxes.scale);
        let (rects, paddings) = placed(boxes.boxes, &vec![[0.0; 2]; offsets.len()]);
        let (on_page, _) = placed(boxes.boxes, &offsets);
        let fresh = Self {
            rects,
            paddings,
            offsets,
            on_page,
        };
        (fresh, scroll)
    }
}

/// The slots of `boxes` that hold a box.
fn held(boxes: &[LayoutBox]) -> Vec<usize> {
    (0..boxes.len())
        .filter(|&slot| !matches!(boxes[slot].content, Content::Vacant))
        .collect()
}

/// Panics, naming the first differing slot, when `kept` and `scroll`
/// are not a fresh placement of `boxes` from `before`, or `placed`'s
/// moved and scrolled slots are not every slot not in `changed` whose
/// rect or padding changed.
pub(crate) fn assert_same(
    before: &Before,
    boxes: &LaidOut<'_>,
    (kept, scroll): (&Placement, &Offsets),
    (placed, changed): (&Placed, &ChangedSlots),
) {
    let (fresh, fresh_scroll) = Fresh::of(before, boxes);
    let held = held(boxes.boxes);
    assert_slots_same(kept, &fresh, &held);
    assert_chain_same(kept, boxes.boxes, &held);
    crate::placement_check_page::assert_on_page_same((kept, boxes), &fresh.on_page, &held);
    if !scroll.same_offsets(&fresh_scroll) {
        fail("the offsets by element differ from a fresh settle");
    }
    if !changed.every {
        assert_moved_same(before, &fresh, &held, (&placed.moved, changed));
        assert_offsets_moved_same(before, &fresh, &held, (&placed.offsets_moved, changed));
    }
}

/// Fails on the first `held` slot whose rect, padding or offset in
/// `kept` is not `fresh`'s.
fn assert_slots_same(kept: &Placement, fresh: &Fresh, held: &[usize]) {
    let of_kept = |slot: usize| (kept.rects[slot], kept.paddings[slot], kept.offsets[slot]);
    let of_fresh = |slot: usize| (fresh.rects[slot], fresh.paddings[slot], fresh.offsets[slot]);
    if let Some(&slot) = held.iter().find(|&&slot| of_kept(slot) != of_fresh(slot)) {
        let (kept, fresh) = (of_kept(slot), of_fresh(slot));
        fail(&format!("slot {slot} kept {kept:?}, fresh {fresh:?}"));
    }
}

/// Fails on the first `held` slot whose containing block or the slot
/// it is placed in differs in `kept` from a fresh walk of `boxes`.
fn assert_chain_same(kept: &Placement, boxes: &[LayoutBox], held: &[usize]) {
    let up = crate::chain::by_containing_block(boxes, NO_SLOT, &|slot, _| slot);
    let containing = containing_blocks(boxes);
    let of_kept = |slot: usize| (kept.up[slot], kept.containing[slot]);
    let of_fresh = |slot: usize| (up[slot], containing[slot]);
    if let Some(&slot) = held.iter().find(|&&slot| of_kept(slot) != of_fresh(slot)) {
        let (kept, fresh) = (of_kept(slot), of_fresh(slot));
        fail(&format!(
            "slot {slot} kept (up, containing) {kept:?}, fresh {fresh:?}"
        ));
    }
}

/// Fails on the first slot in one of `moved` and the `held` slots,
/// neither built again nor freed, whose rect or padding differs from
/// `before`'s, but not the other.
fn assert_moved_same(
    before: &Before,
    fresh: &Fresh,
    held: &[usize],
    (moved, changed): (&[usize], &ChangedSlots),
) {
    let changed = built_or_freed(changed);
    let differs = |slot: &&usize| {
        let slot = **slot;
        let was = (before.rects.get(slot), before.paddings.get(slot));
        !changed.contains(&slot) && was != (fresh.rects.get(slot), fresh.paddings.get(slot))
    };
    let expected: Vec<usize> = held.iter().filter(differs).copied().collect();
    let mut reported = moved.to_vec();
    reported.sort_unstable();
    if let Some(slot) = first_differing(&expected, &reported) {
        fail(&format!(
            "slot {slot}: moved {expected:?}, reported {reported:?}"
        ));
    }
}

/// The first slot in one of `a` and `b`, both sorted, but not the other.
fn first_differing(a: &[usize], b: &[usize]) -> Option<usize> {
    let pairs = a.iter().zip(b).find(|(a, b)| a != b);
    pairs
        .map(|(a, b)| *a.min(b))
        .or_else(|| a.get(b.len()).or_else(|| b.get(a.len())).copied())
}

pub(crate) fn fail(what: &str) -> ! {
    panic!(
        "the kept placement differs from a fresh one: {what}; a box whose layout, scroll \
         offset or containing block changed is not marked (crates/elements/src/placement.rs, \
         layout/written.rs), or the walk does not reach it (placement_walk.rs); reproduce \
         with `cargo xtask test`"
    )
}

/// Fails on the first `held` slot, neither built again nor freed, in
/// one of `offsets_moved` and the slots whose offset as applied
/// differs from `before`'s, but not the other: a scroll is reported to
/// paint by this alone, as it moves no slot (IU4).
// Exact: an offset kept is the same bits.
#[allow(clippy::float_cmp)]
fn assert_offsets_moved_same(
    before: &Before,
    fresh: &Fresh,
    held: &[usize],
    (offsets_moved, changed): (&[usize], &ChangedSlots),
) {
    let changed = built_or_freed(changed);
    let was = |slot: usize| before.offsets.get(slot).copied().unwrap_or_default();
    let differs = |slot: &&usize| !changed.contains(*slot) && was(**slot) != fresh.offsets[**slot];
    let expected: Vec<usize> = held.iter().filter(differs).copied().collect();
    let mut reported: Vec<usize> = offsets_moved
        .iter()
        .filter(|slot| !changed.contains(*slot))
        .copied()
        .collect();
    reported.sort_unstable();
    if let Some(slot) = first_differing(&expected, &reported) {
        fail(&format!(
            "slot {slot}: offsets moved {expected:?}, reported {reported:?}"
        ));
    }
}

/// The slots `changed` built again or freed.
fn built_or_freed(changed: &ChangedSlots) -> FxHashSet<usize> {
    let freed = changed.freed.iter().map(|(slot, _)| *slot);
    changed.rebuilt.iter().copied().chain(freed).collect()
}
