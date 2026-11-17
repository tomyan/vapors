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
    pub(crate) thumb_alpha: u8,
    /// Device pixels per CSS pixel. A scale change builds every box,
    /// but with no box (html `display: none`) none is built.
    pub(crate) scale: f32,
}

impl PaintedFrom {
    /// Whether paint made from `self` shows what a paint made from `now`
    /// would, after an update that changed the `changed` slots: some
    /// kept, none built or moved, no scroll offset or framed row's start
    /// moved (keeping skips the lane bases, `paint_frames.rs`), and the
    /// same thumbs and scale.
    pub(crate) fn shows(&self, now: &Self, changed: &ChangedSlots) -> bool {
        let same = (self.thumb_alpha, self.scale) == (now.thumb_alpha, now.scale);
        let unmoved = changed.moved.is_empty() && changed.offsets_moved.is_empty();
        let unmoved = unmoved && changed.started.is_empty();
        let unmoved = unmoved && changed.scroll_states.is_empty();
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
    kept.frames_worked = 0;
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
        // Relaxed: a paint the update refuses is not shown, and its NaN
        // coordinates equal nothing, themselves included, so it is not
        // compared. Retired when paint stops drawing a box it cannot
        // place, rather than recording it and painting on so the
        // frames, entries and thumbs stay whole (paint_refusals.rs).
        if input.unresolvable.is_empty() {
            crate::kept_paint_check::assert_same(&painted, &fresh);
        }
    }
    painted
}

/// A paint of every box, for the debug build's check; the text cache is
/// left as it was, so the check keeps no layout the update would not.
/// The boxes it could not place must be the update's: a box kept or
/// shifted was placed by an update accepted before, so only the boxes
/// painted again can differ; a difference panics naming both lists.
fn fresh(boxes: &[LayoutBox], rects: &[BoxRect], input: &mut PaintInput<'_>) -> Painted {
    let cache = input.cache.clone();
    let partial = std::mem::take(&mut input.unresolvable);
    let partial_chains = std::mem::take(&mut input.long_chains);
    let painted = paint(boxes, rects, input);
    let mut chains = std::mem::replace(&mut input.long_chains, partial_chains);
    chains.sort_unstable();
    let mut kept_chains = input.long_chains.clone();
    kept_chains.sort_unstable();
    assert_eq!(
        chains, kept_chains,
        "the boxes whose clip chain a fresh paint finds past MAX_CLIP_CHAIN differ from the \
         update's (fresh first): a kept box's chain changed unseen (clip_chain.rs, \
         kept_paint.rs); reproduce with `cargo xtask test`"
    );
    let slots = |list: &[crate::paint_refusals::NotPlaced]| {
        let mut slots: Vec<usize> = list.iter().map(|n| n.slot).collect();
        slots.sort_unstable();
        slots.dedup();
        slots
    };
    assert_eq!(
        slots(&input.unresolvable),
        slots(&partial),
        "the boxes a fresh paint could not place differ from the update's (fresh first): a \
         kept or shifted box carries a coordinate that is not finite (kept_paint.rs, \
         paint_refusals.rs); reproduce with `cargo xtask test`"
    );
    (*input.cache, input.unresolvable) = (cache, partial);
    painted
}

#[cfg(test)]
#[path = "kept_paint_tests.rs"]
mod tests;
