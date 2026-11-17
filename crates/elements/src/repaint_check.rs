//! The debug check after a box is painted again (`repaint.rs`,
//! flat-scene F1): under a whole translation, every item paint snaps is
//! whole device pixels in its frame, so it lands on whole pixels on the
//! page.

use vapors_scene::{Item, Rect, TransformClass};

use crate::kept_box::KeptBox;
use crate::kept_shift_items::{rect_whole_where_finite, snapped_kind, whole_where_finite};
use crate::paint::PaintInput;

/// Panics, naming the slot and the item, when the box in `slot`, its
/// border box snapped at `border_box`, paints a snapped item that is not
/// whole while its frame is a whole translation. Its own step's items
/// are checked whole; its outline ring by the box it is grown from.
pub(crate) fn assert_painted_whole(
    kept: &KeptBox,
    border_box: Rect,
    slot: usize,
    input: &PaintInput<'_>,
) {
    assert_snapped_whole(kept.own.scene.items().iter(), slot, input);
    for item in kept.outline().items() {
        assert_ring_on_box(item, border_box, slot, input);
    }
}

/// Panics, naming the slot and the item, when an item of the box in
/// `slot` that is snapped (`snapped_kind`) is not whole device pixels.
/// Text is placed unsnapped and its glyphs quantised by the painter; a
/// shape's geometry is in its transform, unsnapped as Chromium paints
/// SVG.
pub(crate) fn assert_snapped_whole<'i>(
    items: impl Iterator<Item = &'i Item>,
    slot: usize,
    input: &PaintInput<'_>,
) {
    if !snaps(input) {
        return;
    }
    for item in items.filter(|item| snapped_kind(item)) {
        assert!(
            whole_where_finite(item),
            "box slot {slot}, in a frame of class {:?}, paints {item:?}, not whole device \
             pixels: snapped items under a whole translation are whole (paint.rs \
             `snapped`); reproduce with `cargo xtask test`",
            input.class
        );
    }
}

/// How far apart a ring's four reaches may be, device pixels: each is
/// one float subtraction from the same `(offset + width) * scale`
/// (`paint_outline.rs` `ring_quad`), so they agree to a rounding error,
/// far under this.
const REACH_TOLERANCE: f32 = 1.0 / 64.0;

/// Panics, naming the slot, when the outline ring `item` of the box in
/// `slot` is not grown evenly from its whole border box `border_box`.
/// The ring's own edges are not asked to be whole: its reach,
/// `(offset + width) * scale`, is unsnapped, as at whole scales (the
/// review of #450, H1; whether Chromium snaps it at fractional scales is
/// unmeasured, as the corpus runs at whole scales).
fn assert_ring_on_box(item: &Item, border_box: Rect, slot: usize, input: &PaintInput<'_>) {
    if !snaps(input) {
        return;
    }
    let Item::Quad(ring) = item else {
        panic!("box slot {slot}'s outline paints {item:?}, not a ring quad (paint_outline.rs)")
    };
    assert!(
        rect_whole_where_finite(border_box),
        "box slot {slot}'s outline ring is grown from {border_box:?}, not whole device pixels \
         (paint.rs `snapped`); reproduce with `cargo xtask test`"
    );
    let [left, top] = [
        border_box.left - ring.rect.left,
        border_box.top - ring.rect.top,
    ];
    let [right, bottom] = [
        ring.rect.right - border_box.right,
        ring.rect.bottom - border_box.bottom,
    ];
    let reaches = [left, top, right, bottom];
    if reaches.iter().any(|reach| !reach.is_finite()) {
        return;
    }
    assert!(
        reaches
            .iter()
            .all(|reach| (reach - left).abs() <= REACH_TOLERANCE),
        "box slot {slot}'s outline ring {:?} is not grown evenly from its border box \
         {border_box:?} (paint_outline.rs `ring_quad`); reproduce with `cargo xtask test`",
        ring.rect
    );
}

fn snaps(input: &PaintInput<'_>) -> bool {
    input.class.is_some_and(TransformClass::snaps)
}
