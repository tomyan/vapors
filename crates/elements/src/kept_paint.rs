//! The last update's paint, used again (IU3 3c-1, `docs/plans/iu3.md`).
//! An update that built no box, at the scale the kept paint was made
//! at, in which every box has the rect and the thumbs are shown or
//! hidden as when it was made, paints nothing and shows the kept paint;
//! any other update keeps what it can of it in place
//! (`paint_partial.rs`).
//! Everything else paint reads comes from a box's style or content, and
//! a change to either builds that box again (`layout/marks.rs`). In a
//! debug build the update paints again anyway and asserts the two
//! paints are the same (`kept_paint_check.rs`).

use crate::changed_slots::ChangedSlots;
use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, paint};
use crate::paint_partial::paint_partial;
use crate::painted::Painted;
use crate::placement::Placement;

/// What the kept paint was made from, besides the boxes.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct PaintedFrom {
    /// Each slot's border box, scroll offsets applied, and what placing
    /// it again needs (`placement.rs`).
    pub(crate) placement: Placement,
    pub(crate) thumbs_shown: bool,
    /// Device pixels per CSS pixel. A scale change builds every box,
    /// but with no box (html `display: none`) none is built.
    pub(crate) scale: f32,
}

impl PaintedFrom {
    /// Whether paint made from `self` shows what a paint made from `now`
    /// would, after an update that changed the `changed` slots: some
    /// kept, none built or moved, no scroll offset moved, and the same
    /// thumbs and scale.
    pub(crate) fn shows(&self, now: &Self, changed: &ChangedSlots) -> bool {
        let same = (self.thumbs_shown, self.scale) == (now.thumbs_shown, now.scale);
        let unmoved = changed.moved.is_empty() && changed.offsets_moved.is_empty();
        same && !changed.every && changed.built() == 0 && unmoved
    }
}

/// `kept`, the last update's paint, for this update, with no hit
/// changed; it holds the text layouts it shows, so the text cache keeps
/// them. A debug build paints
/// `boxes` again and panics, naming each part, when that paint differs
/// from `kept`.
pub(crate) fn keep(
    mut kept: Painted,
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
) -> Painted {
    if cfg!(debug_assertions) {
        let fresh = fresh(boxes, rects, input);
        crate::kept_paint_check::assert_same(&kept, &fresh);
    }
    kept.hits_changed = crate::kept_hits::HitsChanged::Known(Vec::new());
    kept
}

/// A paint of `boxes` at `rects`, keeping `last`'s parts in place where
/// they show what painting again would (`paint_partial.rs`). A debug
/// build paints every box again and panics, naming each part, when that
/// paint differs.
pub(crate) fn paint_all(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
    (last, changed): (Option<Painted>, &ChangedSlots),
) -> Painted {
    let painted = paint_partial(boxes, rects, input, last, changed);
    if cfg!(debug_assertions) {
        let fresh = fresh(boxes, rects, input);
        crate::kept_paint_check::assert_same(&painted, &fresh);
    }
    painted
}

/// A paint of every box, for the debug build's check; the text cache
/// is left as it was, so the check keeps no layout the update would not.
fn fresh(boxes: &[LayoutBox], rects: &[BoxRect], input: &mut PaintInput<'_>) -> Painted {
    let cache = input.cache.clone();
    let painted = paint(boxes, rects, input);
    *input.cache = cache;
    painted
}
