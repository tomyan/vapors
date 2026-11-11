//! A text field's caret and selection (E6a), painted in the field's
//! own step, before its text: the selection's rects under the text in
//! the ring's blue at a third, a 1 px underline under composing text,
//! and the caret as a 1 px bar in the text's colour, while the field
//! has focus; from the editor's
//! geometry, which agrees with the text's layout (one Parley, one
//! font). The text's origin is kept for the pointer, which maps a
//! press to a caret by it (`text_input`).

use vapors_scene::{Color, Item, Quad};

use crate::layout::{BoxRect, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// The selection's colour: the focus ring's blue at a third.
const SELECTION: Color = Color {
    r: 0.0,
    g: 96.0 / 255.0,
    b: 223.0 / 255.0,
    a: 0.3,
};

/// Paints the marks of box `index` when it is a focused text field,
/// and keeps every text field's text origin; `rects` are every box's
/// rect, for the field's text child.
pub(crate) fn paint(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let Some(node) = boxes[index].element_in(index) else {
        return;
    };
    if !boxes[index].text_field {
        return;
    }
    let origin = text_origin(boxes, rects, index);
    out.text_origins.insert(node, [origin.x, origin.y]);
    let Some(marks) = &boxes[index].text_marks else {
        return;
    };
    let used = &input.used[&node];
    if !used.visible {
        return;
    }
    for rect in &marks.selection {
        push(out, origin, *rect, SELECTION, input.scale);
    }
    for rect in &marks.composing {
        let line = [rect[0], rect[1] + rect[3] - 1.0, rect[2], 1.0];
        push(out, origin, line, used.text.color, input.scale);
    }
    if let Some(caret) = marks.caret {
        push(out, origin, caret, used.text.color, input.scale);
    }
}

/// A mark `[x, y, width, height]` from the text's origin, as a quad.
fn push(out: &mut Painted, origin: BoxRect, mark: [f32; 4], color: Color, scale: f32) {
    let rect = BoxRect {
        x: origin.x + mark[0],
        y: origin.y + mark[1],
        width: mark[2].max(1.0),
        height: mark[3],
    };
    if rect.height <= 0.0 {
        return;
    }
    out.scene
        .push(Item::Quad(Quad::fill(snapped(rect, scale), color)));
}

/// Where the field's text starts: its text child's box, else its
/// content box.
pub(crate) fn text_origin(boxes: &[LayoutBox], rects: &[BoxRect], index: usize) -> BoxRect {
    boxes[index].children.first().map_or_else(
        || crate::clip::content_box(&boxes[index], rects[index]),
        |child| rects[*child],
    )
}
