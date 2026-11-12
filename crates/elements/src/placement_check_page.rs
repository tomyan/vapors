//! The placement check's comparison on the page (`placement_check.rs`):
//! each kept rect, in its scroll frame, snapped to device pixels as
//! paint snaps it and shifted by the rounded offsets of the scroll
//! containers above it, against the snapped rect of a fresh placement
//! with every offset applied. This is the rounding argument of
//! `docs/plans/iu4.md` section 3 checked on every update.

use vapors_scene::Rect;

use crate::layout::BoxRect;
use crate::paint::snapped;
use crate::placement::{LaidOut, NO_SLOT, Placement};

/// Fails on the first `held` slot whose kept rect, on the page, does
/// not snap to the device pixels of `on_page`, its fresh rect there.
pub(crate) fn assert_on_page_same(
    (kept, boxes): (&Placement, &LaidOut<'_>),
    on_page: &[BoxRect],
    held: &[usize],
) {
    let scale = boxes.scale;
    let shown = |slot: usize| snapped(kept.rects[slot], scale).offset(shift(kept, boxes, slot));
    let fresh = |slot: usize| snapped(on_page[slot], scale);
    if let Some(&slot) = held.iter().find(|&&slot| shown(slot) != fresh(slot)) {
        let (shown, fresh): (Rect, Rect) = (shown(slot), fresh(slot));
        crate::placement_check::fail(&format!(
            "slot {slot} shows at {shown:?} on the page, fresh {fresh:?}"
        ));
    }
}

/// The shift of slot `slot`'s frame, device pixels: the negated
/// offsets of the scroll containers up its chain, each rounded.
fn shift(kept: &Placement, boxes: &LaidOut<'_>, slot: usize) -> [f32; 2] {
    let mut shift = [0.0; 2];
    let mut up = kept.up[slot];
    while up != NO_SLOT {
        if (boxes.scrolls)(up) {
            let offset = kept.offsets[up];
            shift = [0, 1].map(|axis| shift[axis] - (offset[axis] * boxes.scale).round());
        }
        up = kept.up[up];
    }
    shift
}
