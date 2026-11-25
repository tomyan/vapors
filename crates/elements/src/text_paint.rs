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
use crate::paint::PaintInput;
use crate::painting::Painting;

/// The selection's colour: the focus ring's blue at a third.
const SELECTION: Color = Color {
    r: 0.0,
    g: 96.0 / 255.0,
    b: 223.0 / 255.0,
    a: 0.3,
};

/// Paints the marks of box `index` when it is a focused field's run,
/// and keeps every field's text origin, the run's box; `rects` are
/// every box's rect.
pub(crate) fn paint(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &PaintInput<'_>,
    out: &mut Painting,
) {
    let Some(node) = boxes[index].field_text else {
        return;
    };
    let origin = rects[index];
    out.text_origins.insert(node, [origin.x, origin.y]);
    let Some(marks) = &boxes[index].text_marks else {
        return;
    };
    let used = &input.used[&node];
    if !used.visible {
        return;
    }
    for rect in &marks.selection {
        push(out, origin, *rect, SELECTION, input);
    }
    for rect in &marks.composing {
        let line = [rect[0], rect[1] + rect[3] - 1.0, rect[2], 1.0];
        push(out, origin, line, used.text.color, input);
    }
    if let Some(caret) = marks.caret {
        push(out, origin, caret, used.text.color, input);
    }
}

/// A mark `[x, y, width, height]` from the text's origin, as a quad.
fn push(out: &mut Painting, origin: BoxRect, mark: [f32; 4], color: Color, input: &PaintInput<'_>) {
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
        .push(Item::Quad(Quad::fill(input.snapped(rect), color)));
}
