//! The placement check's comparison on the page (`placement_check.rs`):
//! each kept rect, in its scroll frame, snapped to device pixels as
//! paint snaps it and shifted by its frame's whole device-pixel shift
//! up placement's chain ([`chain_shift`]), against the fresh rect in
//! its frame, snapped and shifted by the whole device pixels a fresh
//! placement with every offset applied puts between that rect and its
//! rect on the page. Both sides add a whole shift to the same snapped
//! edges, so the rounding argument of `docs/plans/iu4.md` section 3
//! holds exactly; the page rect's CSS pixels (an applied offset such
//! as 1/1.5 is no f32 number) only give the shift, rounded.

use vapors_scene::Rect;

use crate::layout::{BoxRect, Lanes};
use crate::paint::snapped;
use crate::placement::{LaidOut, NO_SLOT, Placement};

/// A fresh placement's rects: in each slot's frame, and on the page.
pub(crate) struct FreshRects<'a> {
    pub(crate) in_frame: &'a [BoxRect],
    pub(crate) on_page: &'a [BoxRect],
}

/// Fails on the first `held` slot whose kept rect, on the page, does
/// not snap to the device pixels of its fresh rect there.
pub(crate) fn assert_on_page_same(
    (kept, boxes): (&Placement, &LaidOut<'_>),
    fresh: &FreshRects<'_>,
    held: &[usize],
) {
    let scale = boxes.scale;
    let shown = |slot: usize| {
        let [x, y] = chain_shift(kept, boxes.scrolls, scale, kept.up[slot]);
        let lane = lane_offset(kept, boxes.lanes, slot) * scale;
        snapped(kept.rects[slot], scale).offset([x, y + lane])
    };
    let fresh = |slot: usize| fresh_on_page(fresh, scale, slot);
    if let Some(&slot) = held.iter().find(|&&slot| shown(slot) != fresh(slot)) {
        let (shown, fresh): (Rect, Rect) = (shown(slot), fresh(slot));
        crate::placement_check::fail(&format!(
            "slot {slot} shows at {shown:?} on the page, fresh {fresh:?}"
        ));
    }
}

/// How far down the lane frames at and above slot `slot` put it, CSS
/// pixels: for each framed lane child on placement's chain, its
/// container's position in its own frame and its offset in the lane
/// (`docs/plans/splice-layout.md`, section 11). Each term is whole
/// device pixels, as framing requires, so the sum is exact.
#[allow(clippy::cast_possible_truncation)] // CSS px offsets, well within f32
pub(crate) fn lane_offset(kept: &Placement, lanes: &Lanes, slot: usize) -> f32 {
    let mut offset = 0.0;
    let mut at = slot;
    while at != NO_SLOT {
        let container = kept.up[at];
        if kept.framed[at] {
            let start = lanes.frame_offset(container, at).unwrap_or(0.0);
            offset += f64::from(kept.rects[container].y) + start;
        }
        at = container;
    }
    offset as f32
}

/// Slot `slot`'s fresh rect in its frame, snapped, shifted by the
/// whole device pixels between it and its fresh rect on the page. An
/// infinite edge (refused by the a11y tree, `a11y_tree.rs` `finite`)
/// gives no difference, and no shift moves it.
fn fresh_on_page(fresh: &FreshRects<'_>, scale: f32, slot: usize) -> Rect {
    let (in_frame, on_page) = (fresh.in_frame[slot], fresh.on_page[slot]);
    let moved = [on_page.x - in_frame.x, on_page.y - in_frame.y];
    let whole = |css: f32| {
        if css.is_finite() {
            (css * scale).round()
        } else {
            0.0
        }
    };
    snapped(in_frame, scale).offset(moved.map(whole))
}

/// Where the content of the frame `first` is in sits on the page:
/// device pixels, whole, the sum of the frame shifts
/// (`scroll::frame_shift`) of `first` and each slot up `kept`'s chain
/// from it that scrolls. Paint's frames are checked against it
/// (`frames_check.rs`).
pub(crate) fn chain_shift(
    kept: &Placement,
    scrolls: &dyn Fn(usize) -> bool,
    scale: f32,
    first: usize,
) -> [f32; 2] {
    let mut shift = [0.0; 2];
    let mut up = first;
    while up != NO_SLOT {
        if scrolls(up) {
            let own = crate::scroll::frame_shift(kept.offsets[up], scale);
            shift = [shift[0] + own[0], shift[1] + own[1]];
        }
        up = kept.up[up];
    }
    shift
}
