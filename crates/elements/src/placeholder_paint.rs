//! Painting an empty field's placeholder (`docs/placeholder.md`): one
//! line of text in the field's font and Chromium's placeholder grey,
//! from the left of the content box, on the line of the field's run,
//! which layout centres in it as Chromium's `LayoutTextControlSingleLine`
//! centres its inner editor (`layout/field_box.rs`), and cut to the
//! content box. It is no hit, no mark and no part of the editor:
//! the pointer and the selection never reach it.

use crate::clip_rounded::Met;
use vapors_scene::{Item, TextId};

use crate::layout::{BoxRect, InlineRun, LayoutBox};
use crate::paint::{PaintInput, clip_to};
use crate::painting::Painting;

/// The top of field `index`'s run (`scroll_reveal::run_index`): the
/// blank line a placeholder shows on.
fn run_top(boxes: &[LayoutBox], rects: &[BoxRect], index: usize) -> f32 {
    rects[crate::scroll_reveal::run_index(boxes, index)].y
}

/// Paints box `index`'s placeholder, when it shows one and is visible.
pub(crate) fn paint(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
    out: &mut Painting,
) {
    let Some(text) = &boxes[index].placeholder else {
        return;
    };
    let node = boxes[index]
        .element_in(index)
        .expect("a field is an element");
    let used = &input.used[&node];
    if !used.visible {
        return;
    }
    let run = InlineRun {
        text: text.clone(),
        style: crate::style::TextStyle {
            color: used.placeholder,
            ..used.text
        },
        visible: true,
        spans: Vec::new(),
    };
    let layout = input.cache.layout(input.fonts, input.scale, &run, None);
    let content = crate::clip::content_box(&boxes[index], rects[index]);
    let origin = [
        content.x * input.scale,
        run_top(boxes, rects, index) * input.scale,
    ];
    let ink = layout.ink().offset(origin);
    let text = TextId(out.slabs.texts.insert(layout));
    let before = input.clip;
    // A content box that overlaps a rounded clip without being held by
    // it is refused after paint (`clip_refusals.rs` `field_text_overlaps`,
    // `FieldText`), so the square cut stands only until then.
    let radii_of = |slot| crate::clip::radii(boxes, rects, slot, input);
    let square = crate::clip::BoxClip::square(content);
    let (Met::Clip(clip) | Met::Overlap(clip)) =
        crate::clip_rounded::meet(before, square, &radii_of);
    clip_to(Some(clip), (boxes, rects), input, &mut out.scene);
    out.scene.push(Item::Text { text, origin, ink });
    clip_to(before, (boxes, rects), input, &mut out.scene);
}
