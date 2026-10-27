//! A checkbox's mark (3-E3b): when checked, a rounded square in
//! `--vapors-mark-color` inside the content box, inset by a quarter of
//! the smaller side. The box itself (border, background, radius) is the
//! element's own, from the user-agent sheet or the app's.

use vapors_scene::{Item, Quad};

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// The mark's inset from each edge, as a fraction of the content box's
/// smaller side.
const INSET: f32 = 0.25;

/// Paints the mark of `layout_box` when it is a checked checkbox, over
/// its content box (border box `border_box`).
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let (Some(true), &Content::Element(node)) = (layout_box.checked, &layout_box.content) else {
        return;
    };
    let used = &input.used[&node];
    if !used.visible {
        return;
    }
    let content = crate::clip::content_box(layout_box, border_box);
    let inset = content.width.min(content.height) * INSET;
    let mark = BoxRect {
        x: content.x + inset,
        y: content.y + inset,
        width: (content.width - 2.0 * inset).max(0.0),
        height: (content.height - 2.0 * inset).max(0.0),
    };
    if mark.width <= 0.0 || mark.height <= 0.0 || used.parts.mark_color.a <= 0.0 {
        return;
    }
    let mut quad = Quad::fill(snapped(mark, input.scale), used.parts.mark_color);
    quad.radii = [input.scale; 4];
    out.scene.push(Item::Quad(quad));
}
