//! A scroll container's overlay thumbs (F2c S9): drawn over its content
//! at the far edge of its padding box, taking no space, sized by how
//! much of the content shows and placed by the offset. They take no
//! pointer: dragging a thumb is not built.

use vapors_scene::{Item, Quad};

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// Space between a thumb and the padding box's edges, CSS px.
pub(crate) const INSET: f32 = 2.0;
/// The shortest a thumb is drawn, CSS px, so a long list's stays
/// visible.
pub(crate) const MIN_LENGTH: f32 = 24.0;

/// The vertical thumb then the horizontal one, for the axes that can
/// scroll, in CSS px: `padding` is the container's padding box, `at` its
/// offset, `max` its range, `width` the thumb's thickness (0 for none).
pub(crate) fn thumbs(padding: BoxRect, at: [f32; 2], max: [f32; 2], width: f32) -> Vec<BoxRect> {
    if width <= 0.0 {
        return Vec::new();
    }
    let corner = |axis: usize| if max[axis] > 0.0 { width + INSET } else { 0.0 };
    let vertical = (max[1] > 0.0).then(|| {
        let (y, height) = along(padding.y, padding.height, corner(0), at[1], max[1]);
        rect(padding.x + padding.width - INSET - width, y, width, height)
    });
    let horizontal = (max[0] > 0.0).then(|| {
        let (x, length) = along(padding.x, padding.width, corner(1), at[0], max[0]);
        rect(x, padding.y + padding.height - INSET - width, length, width)
    });
    vertical.into_iter().chain(horizontal).collect()
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> BoxRect {
    BoxRect {
        x,
        y,
        width,
        height,
    }
}

/// Paints the thumbs of the scroll container `layout_box` (border box
/// `border_box`), unless thumbs are hidden now.
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let Content::Element(node) = layout_box.content else {
        return;
    };
    let Some((at, max)) = input.scroll.state(node).filter(|_| input.thumbs_shown) else {
        return;
    };
    let style = input.used[&node].scrollbar;
    let padding = crate::clip::padding_box(layout_box, border_box);
    for thumb in thumbs(padding, at, max, style.width) {
        let mut quad = Quad::fill(snapped(thumb, input.scale), style.thumb);
        quad.radii = [style.width * input.scale / 2.0; 4];
        out.scene.push(Item::Quad(quad));
    }
}

/// A thumb's start and length along a padding box edge from `edge`,
/// `shown` long, leaving `corner` at the end for the other thumb.
fn along(edge: f32, shown: f32, corner: f32, at: f32, max: f32) -> (f32, f32) {
    let track = (shown - 2.0 * INSET - corner).max(0.0);
    let length = (track * shown / (shown + max)).max(MIN_LENGTH).min(track);
    (edge + INSET + (track - length) * at / max, length)
}

#[cfg(test)]
#[path = "thumb_tests.rs"]
mod tests;
