//! An element's outline (3-E2b), painted at its `Step::Outline`: last
//! within its stacking context (CSS 2 Appendix E), so a later stacking
//! context above it covers it, as in Chromium. The ring lies outside
//! the border box at the outline's offset, under the clip the element
//! paints under (its ancestors', not its own overflow), its corners
//! following the border's.

use vapors_scene::{Color, Item, Quad, Rect};

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;
use crate::style::outline::Outline;

/// The outline of the element `layout_box` lays out at `rect`, if it is
/// visible and draws one.
pub(crate) fn paint_outline(
    layout_box: &LayoutBox,
    rect: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let Content::Element(node) = layout_box.content else {
        return;
    };
    let used = &input.used[&node];
    let Some(outline) = used.decoration.outline.filter(|_| used.visible) else {
        return;
    };
    if let Some(ring) = ring_quad(
        snapped(rect, input.scale),
        &outline,
        used.decoration.radii,
        input.scale,
    ) {
        out.scene.push(Item::Quad(ring));
    }
}

/// The ring around `border_box` (device pixels), or `None` when a
/// negative offset pulls it inside out.
fn ring_quad(border_box: Rect, outline: &Outline, radii: [f32; 4], scale: f32) -> Option<Quad> {
    let reach = (outline.offset + outline.width) * scale;
    let rect = Rect::new(
        border_box.left - reach,
        border_box.top - reach,
        border_box.right + reach,
        border_box.bottom + reach,
    );
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    Some(Quad {
        rect,
        background: Color::default(),
        border: [outline.width * scale; 4],
        border_colors: [outline.color; 4],
        radii: ring_radii(radii, reach, scale),
    })
}

/// The ring's outer corners: the border's grown by the ring's reach,
/// none below zero (a negative offset shrinks them, as Chromium clamps
/// them).
fn ring_radii(radii: [f32; 4], reach: f32, scale: f32) -> [f32; 4] {
    radii.map(|r| {
        if r > 0.0 {
            (r * scale + reach).max(0.0)
        } else {
            0.0
        }
    })
}
