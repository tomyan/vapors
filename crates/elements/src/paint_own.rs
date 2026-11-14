//! A box's own painting: an element's shadows, background and border
//! and its place among the hits, or a text run (`paint.rs`).

use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Quad, Rect, Shadow, TextId};

use crate::corners;
use crate::layout::{BoxRect, Content, InlineRun, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painting::Painting;
use crate::style::BoxShadow;

/// One box's own painting: an element's shadows, background and
/// border, or a run's text; its children are painted in their turn.
pub(crate) fn paint_own(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
    out: &mut Painting,
) {
    match &boxes[index].content {
        Content::Element(node) => paint_element(*node, rects[index], input, out),
        Content::Text(run) => paint_text(run, rects[index], input, out),
        Content::Vacant => unreachable!("a vacant slot is no box's child"),
    }
}

/// The element's shadows, then its background and border, unless it is
/// hidden; a background that became the canvas's is not painted again.
/// Also its place among the elements the pointer can hit.
fn paint_element(node: NodeId, rect: BoxRect, input: &PaintInput<'_>, out: &mut Painting) {
    let used = &input.used[&node];
    if used.visible && used.takes_pointer {
        out.hits.push((node, crate::clip::within(input.clip, rect)));
        out.cursors.insert(node, used.cursor);
    }
    if !used.visible {
        return;
    }
    out.visible.insert(node);
    let border_box = snapped(rect, input.scale);
    let decoration = &used.decoration;
    let radii = corners::fitted(decoration.radii.map(|r| r * input.scale), border_box);
    for shadow in decoration.shadows.iter().rev() {
        let item = shadow_item(shadow, border_box, radii, input.scale);
        out.scene.push(Item::Shadow(item));
    }
    let background = if input.propagated == Some(node) {
        Color::default()
    } else {
        used.background
    };
    let border = decoration.border.map(|w| w * input.scale);
    if background.a > 0.0 || border.iter().any(|w| *w > 0.0) {
        let quad = Quad {
            rect: border_box,
            background,
            border,
            border_colors: decoration.border_colors,
            radii,
        };
        out.scene.push(Item::Quad(quad));
    }
    #[cfg(feature = "test-support")]
    crate::test_items::paint(node, out);
}

/// A CSS outer shadow as the scene draws it: the border box moved by
/// the offset and grown by the spread, corners grown with it.
fn shadow_item(shadow: &BoxShadow, caster: Rect, radii: [f32; 4], scale: f32) -> Shadow {
    let [dx, dy] = shadow.offset.map(|v| v * scale);
    let spread = shadow.spread * scale;
    Shadow {
        rect: Rect::new(
            caster.left + dx - spread,
            caster.top + dy - spread,
            caster.right + dx + spread,
            caster.bottom + dy + spread,
        ),
        radii: radii.map(|r| corners::spread(r, spread)),
        blur: shadow.blur * scale,
        color: shadow.color,
        caster,
        caster_radii: radii,
    }
}

fn paint_text(run: &InlineRun, rect: BoxRect, input: &mut PaintInput<'_>, out: &mut Painting) {
    let layout = input
        .cache
        .layout(input.fonts, input.scale, run, Some(rect.width));
    crate::paint_inline::paint_spans(run, &layout, rect, input, out);
    let origin = [rect.x * input.scale, rect.y * input.scale];
    let ink = layout.ink().offset(origin);
    let text = TextId(out.slabs.texts.insert(layout));
    out.scene.push(Item::Text { text, origin, ink });
}
