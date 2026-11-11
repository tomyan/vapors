//! The last update's paint, used again (IU3 3c-1, `docs/plans/iu3.md`).
//! An update that built no box, at the scale the kept paint was made
//! at, in which every box has the rect and the thumbs are shown or
//! hidden as when it was made, paints nothing and shows the kept paint.
//! Everything else paint reads comes from a box's style or content, and
//! a change to either builds that box again (`layout/marks.rs`). In a
//! debug build the update paints again anyway and asserts the two
//! paints are the same (`kept_paint_check.rs`).

use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, paint};
use crate::painted::Painted;

/// What the kept paint was made from, besides the boxes.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct PaintedFrom {
    /// Each slot's border box, scroll offsets applied (`placed`).
    pub(crate) rects: Vec<BoxRect>,
    pub(crate) thumbs_shown: bool,
    /// Device pixels per CSS pixel. A scale change builds every box,
    /// but with no box (html `display: none`) none is built.
    pub(crate) scale: f32,
}

impl PaintedFrom {
    /// Whether paint made from `self` shows what a paint made from `now`
    /// would, after an update that built `built` boxes.
    pub(crate) fn shows(&self, now: &Self, built: usize) -> bool {
        built == 0 && self == now
    }
}

/// `kept`, the last update's paint, for this update: the text layouts
/// it uses are kept for the next. A debug build paints `boxes` again
/// and panics, naming each part, when that paint differs from `kept`.
pub(crate) fn keep(
    kept: Painted,
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    scrolls: &dyn Fn(usize) -> bool,
) -> Painted {
    input.cache.keep_previous();
    if cfg!(debug_assertions) {
        let fresh = paint_all(boxes, rects, input, scrolls);
        crate::kept_paint_check::assert_same(&kept, &fresh);
    }
    kept
}

/// A new paint of `boxes` at `rects`, with its scroll chains.
pub(crate) fn paint_all(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    scrolls: &dyn Fn(usize) -> bool,
) -> Painted {
    let mut painted = paint(boxes, rects, input);
    painted.scroll_chains = crate::scroll_chains::Chains::of(boxes, scrolls);
    painted
}
