//! A moved slot's kept paint shifted in place (IU3 3c-3e,
//! `docs/plans/iu3.md` "Moved boxes: option (a)"). A box that kept its
//! size and padding and, on each axis, either stayed (the same position
//! bit for bit) or moved by a whole number of device pixels from a
//! whole device pixel, has its items moved by that delta, its items'
//! clip set to its new clip and its own hit cut from its new rect and
//! clip, and its text items' origin and ink made from the new rect; all
//! as paint makes them, from whole numbers or the new values, so the
//! shifted part is, bit for bit, what painting again would make, which
//! a debug build checks (`kept_paint_check.rs`). The clip is read by
//! nothing in a part but its hits (`paint.rs` `clip_to`). A moved slot
//! has three states: kept (its `BoxFrom` unchanged), shifted, or
//! painted again. Painted again: a move on an axis whose positions are
//! not whole device pixels (a fractional scale or scroll), a size or
//! padding change, an item off whole pixels (a shadow with a fractional
//! offset), an item under another clip than the part's, a text item in
//! an element's part, or a part holding inline elements' hits and
//! rects, a range track or a text field's origin, which paint derives
//! from more than the rect. Never shifted on a guess.

use vapors_scene::{Item, Rect};

use crate::kept_box::{BoxFrom, KeptBox};
use crate::kept_shift_items::{shifted_item, text_item, whole_item};
use crate::layout::{BoxRect, Content};
use crate::paint::snapped;
use crate::painted::Painted;
use crate::slab::Slabs;

/// How a kept part moves to show its box at a new rect.
pub(crate) struct Shift {
    pub(crate) slot: usize,
    from: BoxFrom,
    /// The move in device pixels, whole.
    delta: [f32; 2],
    /// The new clip in device pixels, as paint sets it on each item.
    clip: Option<Rect>,
}

impl Shift {
    /// The shift of `part`, slot `slot` holding `content`, to show
    /// `from` at `scale`, when a shift shows it exactly; else `None`,
    /// and the slot is painted again.
    pub(crate) fn of(
        slot: usize,
        (part, content): (&KeptBox, &Content),
        from: BoxFrom,
        (scale, edges): (f32, impl Fn(BoxRect) -> [BoxRect; 2]),
    ) -> Option<Self> {
        let delta = whole_delta(part.from.rect, from.rect, scale)?;
        let (old, new) = (edges(part.from.rect), edges(from.rect));
        let shiftable = snaps_by(old, new, delta, scale)
            && part.from.padding == from.padding
            && part.own.positions_only_in_scene_and_hits()
            && hits_shiftable(part, content)
            && items_shiftable(part, content)
            && under_one_clip(part, part.from.clip.map(|clip| snapped(clip, scale)));
        let clip = from.clip.map(|clip| snapped(clip, scale));
        shiftable.then_some(Self {
            slot,
            from,
            delta,
            clip,
        })
    }

    /// Moves `kept`'s items and own hit, sets its clip, and notes what
    /// it now shows.
    pub(crate) fn apply_to(self, kept: &mut KeptBox, slabs: &Slabs, scale: f32) {
        let rect = self.from.rect;
        for scene in [&mut kept.own.scene, &mut kept.outline] {
            for item in scene.items_mut() {
                *item = match *item {
                    Item::Text { text, .. } => text_item(text, rect, slabs, scale),
                    other => shifted_item(other, self.delta),
                };
            }
            scene.clips_mut().fill(self.clip);
        }
        if let Some((_, hit)) = kept.own.hits.first_mut() {
            *hit = crate::clip::within(self.from.clip, rect);
        }
        kept.from = self.from;
    }
}

/// Shifts each of `shifts`' parts in `painted` at `scale`, counting
/// them.
pub(crate) fn apply(painted: &mut Painted, shifts: Vec<Shift>, scale: f32) {
    for shift in shifts {
        let kept = painted.kept_boxes[shift.slot].as_mut().unwrap_or_else(|| {
            panic!(
                "slot {} is shifted but holds no kept paint: paint_partial.rs read it from \
                 one; reproduce with `cargo xtask test`",
                shift.slot
            )
        });
        shift.apply_to(kept, &painted.slabs, scale);
        painted.boxes_shifted += 1;
    }
}

/// The move from `old` to `new` in device pixels, when `new` keeps
/// `old`'s size and each axis stayed or moved whole pixels.
// Exact: a size is kept bit for bit.
#[allow(clippy::float_cmp)]
fn whole_delta(old: BoxRect, new: BoxRect, scale: f32) -> Option<[f32; 2]> {
    let same_size = old.width == new.width && old.height == new.height;
    let dx = axis_delta(old.x, new.x, scale)?;
    let dy = axis_delta(old.y, new.y, scale)?;
    same_size.then_some([dx, dy])
}

/// The move on one axis in device pixels: none when the position is
/// the same bit for bit (adding none changes nothing), else whole when
/// both positions lie on whole device pixels.
// Exact: an unmoved axis is the same bits; a whole number has no fraction.
#[allow(clippy::float_cmp)]
fn axis_delta(old: f32, new: f32, scale: f32) -> Option<f32> {
    if old == new {
        return Some(0.0);
    }
    let (from, to) = (old * scale, new * scale);
    (from.fract() == 0.0 && to.fract() == 0.0).then_some(to - from)
}

/// Whether each of `new`'s boxes snaps (`paint.rs` `snapped`, which
/// rounds half away from zero) to its `old` box's snapped edges moved
/// by `delta`. Rounding is not translation-invariant at a tie that
/// crosses zero (`round(-1.5) + 3` is 1, `round(1.5)` is 2) nor where
/// a float sum rounds differently after a move, so a move whose edges
/// round differently is painted again.
fn snaps_by(old: [BoxRect; 2], new: [BoxRect; 2], delta: [f32; 2], scale: f32) -> bool {
    old.iter().zip(&new).all(|(old, new)| {
        let old = snapped(*old, scale);
        let moved = Rect::new(
            old.left + delta[0],
            old.top + delta[1],
            old.right + delta[0],
            old.bottom + delta[1],
        );
        snapped(*new, scale) == moved
    })
}

/// Whether `part`'s hits are made from its rect alone: an element's
/// one hit, its border box cut to its clip, or none; a text run's are
/// its inline elements' fragments, which are not.
fn hits_shiftable(part: &KeptBox, content: &Content) -> bool {
    match content {
        Content::Element(_) => part.own.hits.len() <= 1,
        Content::Text(_) | Content::Vacant => part.own.hits.is_empty(),
    }
}

/// Whether every item of `part` can be shifted exactly: a text item
/// only in a text run's part, whose origin paint takes from its rect;
/// any other item on whole device pixels.
fn items_shiftable(part: &KeptBox, content: &Content) -> bool {
    let mut items = part.own.scene.items().iter().chain(part.outline.items());
    items.all(|item| match item {
        Item::Text { .. } => matches!(content, Content::Text(_)),
        other => whole_item(other),
    })
}

/// Whether every item of `part` is under `clip`, the part's own, so
/// setting the new clip on every item is what paint does.
fn under_one_clip(part: &KeptBox, clip: Option<Rect>) -> bool {
    let mut clips = part.own.scene.clips().iter().chain(part.outline.clips());
    clips.all(|each| *each == clip)
}

#[cfg(test)]
#[path = "kept_shift_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "kept_shift_refusals_tests.rs"]
mod refusals_tests;

#[cfg(test)]
#[path = "kept_shift_ties_tests.rs"]
mod ties_tests;
