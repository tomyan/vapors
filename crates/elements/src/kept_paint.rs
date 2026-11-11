//! The last update's paint, used again (IU3 3c-1, `docs/plans/iu3.md`).
//! An update that built no box, at the scale the kept paint was made
//! at, in which every box has the rect and the thumbs are shown or
//! hidden as when it was made, paints nothing and shows the kept paint;
//! any other update copies what it can of it (`paint_partial.rs`).
//! Everything else paint reads comes from a box's style or content, and
//! a change to either builds that box again (`layout/marks.rs`). In a
//! debug build the update paints again anyway and asserts the two
//! paints are the same (`kept_paint_check.rs`).

use crate::kept_box::KeptBox;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, paint};
use crate::paint_partial::paint_partial;
use crate::painted::Painted;
use crate::scroll_chains::Chains;

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

/// `kept`, the last update's paint, for this update; it holds the text
/// layouts it shows, so the text cache keeps them. A debug build paints
/// `boxes` again and panics, naming each part, when that paint differs
/// from `kept`.
pub(crate) fn keep(
    kept: Painted,
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    scrolls: &dyn Fn(usize) -> bool,
) -> Painted {
    if cfg!(debug_assertions) {
        crate::kept_paint_check::assert_same(&kept, &fresh(boxes, rects, input, scrolls));
    }
    kept
}

/// A paint of `boxes` at `rects`, with its scroll chains, copying the
/// kept paint in `last` where it shows what painting again would
/// (`paint_partial.rs`). A debug build paints every box again and
/// panics, naming each part, when that paint differs.
pub(crate) fn paint_all(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    scrolls: &dyn Fn(usize) -> bool,
    last: Vec<Option<KeptBox>>,
) -> Painted {
    let mut painted = paint_partial(boxes, rects, input, last);
    painted.scroll_chains = Chains::of(boxes, scrolls);
    if cfg!(debug_assertions) {
        crate::kept_paint_check::assert_same(&painted, &fresh(boxes, rects, input, scrolls));
    }
    painted
}

/// A paint of every box, for the debug build's check; the text cache is
/// left as it was, so the check keeps no layout the update would not.
fn fresh(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    scrolls: &dyn Fn(usize) -> bool,
) -> Painted {
    let cache = input.cache.clone();
    let mut painted = paint(boxes, rects, input);
    painted.scroll_chains = Chains::of(boxes, scrolls);
    *input.cache = cache;
    painted
}
