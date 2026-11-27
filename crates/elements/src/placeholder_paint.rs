//! Painting an empty field's placeholder (`docs/placeholder.md`): one
//! line of text in the field's font and Chromium's placeholder grey,
//! from the left of the content box, on the line of the field's run,
//! which layout centres in it as Chromium's `LayoutTextControlSingleLine`
//! centres its inner editor (`layout/field_box.rs`), and cut to the
//! content box. It is no hit, no mark and no part of the editor:
//! the pointer and the selection never reach it.

use vapors_scene::{Item, TextId};

use crate::layout::{BoxRect, InlineRun, LayoutBox};
use crate::paint::{PaintInput, clip_to};
use crate::painting::Painting;

/// The top of field `index`'s run, its first child: the blank line a
/// placeholder shows on.
///
/// # Panics
/// When the field has no run (`layout/field_box.rs` `mark_run`).
fn run_top(boxes: &[LayoutBox], rects: &[BoxRect], index: usize) -> f32 {
    let run = *boxes[index]
        .children
        .first()
        .unwrap_or_else(|| panic!("field box {index} has no run: a blank field's run is a no-break space (layout/boxes.rs)"));
    rects[run].y
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
    clip_to(
        Some(crate::clip::within(before, content)),
        input,
        &mut out.scene,
    );
    out.scene.push(Item::Text { text, origin, ink });
    clip_to(before, input, &mut out.scene);
}
