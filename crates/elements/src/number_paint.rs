//! A number field's selection and caret (3-E3c), painted in the
//! field's own step, before its text: a selection highlight under the
//! whole text when it is selected, else a 1 px caret after the text
//! in the text's colour, while the field has focus. The box, border
//! and text are the element's own.

use vapors_scene::{Color, Item, Quad};

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// The selection's colour: the focus ring's blue at a third.
const SELECTION: Color = Color {
    r: 0.0,
    g: 96.0 / 255.0,
    b: 223.0 / 255.0,
    a: 0.3,
};

/// Paints the selection or caret of box `index` when it is a focused
/// number field; `rects` are every box's rect, for its text child's.
pub(crate) fn paint(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
    out: &mut Painted,
) {
    let Content::Element(node) = boxes[index].content else {
        return;
    };
    let Some(focus) = boxes[index].number_focus else {
        return;
    };
    let used = &input.used[&node];
    if !used.visible {
        return;
    }
    let (text_rect, text_width) = text_extent(boxes, rects, index, input);
    let rect = if focus.selected {
        BoxRect {
            width: text_width,
            ..text_rect
        }
    } else {
        BoxRect {
            x: text_rect.x + text_width,
            width: 1.0,
            ..text_rect
        }
    };
    let color = if focus.selected {
        SELECTION
    } else {
        used.text.color
    };
    if rect.height > 0.0 {
        out.scene
            .push(Item::Quad(Quad::fill(snapped(rect, input.scale), color)));
    }
}

/// The field's text box and the width of the text in it (0 for a
/// blank field, whose run is a no-break space).
fn text_extent(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
) -> (BoxRect, f32) {
    let Some(child) = boxes[index].children.first().copied() else {
        return (crate::clip::content_box(&boxes[index], rects[index]), 0.0);
    };
    let rect = rects[child];
    let Content::Text(run) = &boxes[child].content else {
        return (rect, 0.0);
    };
    if run.text == "\u{a0}" {
        return (rect, 0.0);
    }
    let layout = input
        .cache
        .layout(input.fonts, input.scale, run, Some(rect.width));
    (rect, layout.width() / input.scale)
}
