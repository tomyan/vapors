//! Painting an empty field's placeholder (`docs/placeholder.md`): one
//! line of text in the field's font and Chromium's placeholder grey,
//! from the left of the content box, centred in it as Chromium's
//! `LayoutTextControlSingleLine` centres its inner editor, and cut to
//! the content box. It is no hit, no mark and no part of the editor:
//! the pointer and the selection never reach it.

use vapors_scene::{Item, TextId};

use crate::layout::{BoxRect, InlineRun, LayoutBox};
use crate::paint::{PaintInput, clip_to};
use crate::painting::Painting;

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
            color: crate::placeholder::COLOR,
            ..used.text
        },
        visible: true,
        spans: Vec::new(),
    };
    let layout = input.cache.layout(input.fonts, input.scale, &run, None);
    let content = crate::clip::content_box(&boxes[index], rects[index]);
    let spare = content.height - layout.height() / input.scale;
    let origin = [
        content.x * input.scale,
        (content.y + spare / 2.0) * input.scale,
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
