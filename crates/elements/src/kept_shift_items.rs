//! One kept item shifted (`kept_shift.rs`): a whole move of its
//! rect, or a text item made from its rect as `paint_own.rs` makes it.

use vapors_scene::{Item, Rect, TextId};

use crate::layout::BoxRect;
use crate::slab::Slabs;

/// Whether every coordinate of `item` is a whole number.
pub(crate) fn whole_item(item: &Item) -> bool {
    match *item {
        Item::Shadow(shadow) => whole_rect(shadow.rect) && whole_rect(shadow.caster),
        Item::Quad(quad) => whole_rect(quad.rect),
        Item::Image { rect, .. } | Item::Icon { rect, .. } | Item::Surface { rect, .. } => {
            whole_rect(rect)
        }
        Item::Text { .. } => false,
    }
}

// Exact: a whole number has no fraction.
#[allow(clippy::float_cmp)]
fn whole_rect(rect: Rect) -> bool {
    [rect.left, rect.top, rect.right, rect.bottom]
        .iter()
        .all(|v| v.fract() == 0.0)
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
