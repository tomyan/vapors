//! One kept item shifted (`kept_shift.rs`, and a scroll frame's shift in
//! `frames.rs`): a move of its rect, a shape's translation and bounds,
//! or a text item made from its rect as `paint_own.rs` makes it.

use vapors_scene::{Item, Rect, TextId};

use crate::layout::BoxRect;
use crate::slab::Slabs;

/// Whether every coordinate of `item` is a whole number.
pub(crate) fn whole_item(item: &Item) -> bool {
    edges_all(item, is_whole)
}

/// Whether every finite coordinate paint snaps in `item` is a whole
/// number, for the debug check after a repaint (`repaint.rs`, which says
/// why it skips the rest). A shadow's caster alone: its blurred rect is
/// the caster moved by the offset and grown by the spread, each times
/// the scale and unsnapped (`paint_own.rs` `shadow_item`), as the
/// corpus's Chromium comparisons pass it.
#[cfg(debug_assertions)]
pub(crate) fn whole_where_finite(item: &Item) -> bool {
    let edge = |v: f32| !v.is_finite() || is_whole(v);
    match *item {
        Item::Shadow(shadow) => rect_all(shadow.caster, edge),
        _ => edges_all(item, edge),
    }
}

/// Whether `edge` holds of every coordinate of `item`'s rects; false
/// for text and shapes.
fn edges_all(item: &Item, edge: impl Fn(f32) -> bool + Copy) -> bool {
    match *item {
        Item::Shadow(shadow) => rect_all(shadow.rect, edge) && rect_all(shadow.caster, edge),
        Item::Quad(quad) => rect_all(quad.rect, edge),
        Item::Image { rect, .. } | Item::Icon { rect, .. } | Item::Surface { rect, .. } => {
            rect_all(rect, edge)
        }
        // Painted again rather than shifted in place: a shape's transform
        // and bounds are not checked whole (no element paints one before
        // S4a, which decides). A scroll frame's shift moves it exactly.
        Item::Text { .. } | Item::Shape(_) => false,
    }
}

/// Whether `edge` holds of each of `rect`'s edges.
fn rect_all(rect: Rect, edge: impl Fn(f32) -> bool) -> bool {
    [rect.left, rect.top, rect.right, rect.bottom]
        .into_iter()
        .all(edge)
}

/// Whether `item` is of a kind paint snaps to the device pixel grid
/// under a whole translation: every kind `whole_item` tests but text
/// and shapes. For the debug check after a repaint (`repaint.rs`).
#[cfg(debug_assertions)]
pub(crate) fn snapped_kind(item: &Item) -> bool {
    !matches!(item, Item::Text { .. } | Item::Shape(_))
}

// Exact: a whole number has no fraction.
#[allow(clippy::float_cmp)]
fn is_whole(v: f32) -> bool {
    v.fract() == 0.0
}

/// `item` moved by `delta`.
pub(crate) fn shifted_item(item: Item, delta: [f32; 2]) -> Item {
    match item {
        Item::Shadow(mut shadow) => {
            shadow.rect = shadow.rect.offset(delta);
            shadow.caster = shadow.caster.offset(delta);
            Item::Shadow(shadow)
        }
        Item::Quad(mut quad) => {
            quad.rect = quad.rect.offset(delta);
            Item::Quad(quad)
        }
        Item::Image { image, rect } => Item::Image {
            image,
            rect: rect.offset(delta),
        },
        Item::Icon { icon, rect, colour } => Item::Icon {
            icon,
            rect: rect.offset(delta),
            colour,
        },
        Item::Surface { surface, rect } => Item::Surface {
            surface,
            rect: rect.offset(delta),
        },
        Item::Text { .. } => unreachable!("a text item is made from its rect (text_item)"),
        Item::Shape(mut shape) => {
            shape.transform[4] += delta[0];
            shape.transform[5] += delta[1];
            shape.bounds = shape.bounds.offset(delta);
            Item::Shape(shape)
        }
    }
}

/// The text item of run `text` at `rect`, as `paint_own.rs` makes it:
/// its origin the rect's top-left in device pixels, its ink the
/// layout's from there.
pub(crate) fn text_item(text: TextId, rect: BoxRect, slabs: &Slabs, scale: f32) -> Item {
    let layout = slabs.texts.get(text.0).unwrap_or_else(|| {
        panic!(
            "text {} named by a shifted part is not in the slab: its id was freed while \
             its part was kept (paint_partial.rs dropped, slab.rs); reproduce with \
             `cargo xtask test`",
            text.0
        )
    });
    let origin = [rect.x * scale, rect.y * scale];
    Item::Text {
        text,
        origin,
        ink: layout.ink().offset(origin),
    }
}

#[cfg(test)]
#[path = "kept_shift_items_tests.rs"]
mod tests;
