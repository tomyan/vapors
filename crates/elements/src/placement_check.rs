//! The debug build's check on a kept placement (`placement.rs`): every
//! box settled and placed afresh with `Offsets::settle` and
//! `layout::placed`, and compared slot by slot with what placement kept,
//! the offsets by element, and the moved slots with a comparison of
//! every held slot's rect and padding with the last paint's.

use rustc_hash::FxHashSet;

use crate::changed_slots::ChangedSlots;
use crate::layout::{BoxRect, Content, placed};
use crate::placement::{LaidOut, Placement};
use crate::scroll::Offsets;

/// What placement started from: the offsets, rects and paddings.
pub(crate) struct Before {
    scroll: Offsets,
    rects: Vec<BoxRect>,
    paddings: Vec<taffy::Rect<f32>>,
}

/// A fresh placement: each slot's rect, padding and offset.
struct Fresh {
    rects: Vec<BoxRect>,
    paddings: Vec<taffy::Rect<f32>>,
    offsets: Vec<[f32; 2]>,
}

impl Before {
    /// `scroll` and `kept` before placement.
    pub(crate) fn of(scroll: &Offsets, kept: &Placement) -> Self {
        Self {
            scroll: scroll.clone(),
            rects: kept.rects.clone(),
            paddings: kept.paddings.clone(),
        }
    }
}

/// Panics, naming the first differing slot, when `kept` and `scroll`
/// are not a fresh placement of `boxes` from `before`, or `moved` is not
/// every slot not in `changed` whose rect or padding changed.
pub(crate) fn assert_same(
    before: &Before,
    boxes: &LaidOut<'_>,
    (kept, scroll): (&Placement, &Offsets),
    (moved, changed): (&[usize], &ChangedSlots),
) {
    let mut fresh_scroll = before.scroll.clone();
    let offsets = fresh_scroll.settle(boxes.boxes, boxes.scrolls, boxes.scale);
    let (rects, paddings) = placed(boxes.boxes, &offsets);
    let fresh = Fresh {
        rects,
        paddings,
        offsets,
    };
    let held: Vec<usize> = (0..fresh.rects.len())
        .filter(|&slot| !matches!(boxes.boxes[slot].content, Content::Vacant))
        .collect();
    assert_slots_same(kept, &fresh, &held);
    if !scroll.same_offsets(&fresh_scroll) {
        fail("the offsets by element differ from a fresh settle");
    }
    if !changed.every {
        assert_moved_same(before, &fresh, &held, (moved, changed));
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

/// Fails on the first slot in one of `moved` and the `held` slots,
/// neither built again nor freed, whose rect or padding differs from
/// `before`'s, but not the other.
fn assert_moved_same(
    before: &Before,
    fresh: &Fresh,
    held: &[usize],
    (moved, changed): (&[usize], &ChangedSlots),
) {
    let freed = changed.freed.iter().map(|(slot, _)| *slot);
    let changed: FxHashSet<usize> = changed.rebuilt.iter().copied().chain(freed).collect();
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

fn fail(what: &str) -> ! {
    panic!(
        "the kept placement differs from a fresh one: {what}; a box whose layout, scroll \
         offset or containing block changed is not marked (crates/elements/src/placement.rs, \
         layout/written.rs), or the walk does not reach it (placement_walk.rs); reproduce \
         with `cargo xtask test`"
    )
}
