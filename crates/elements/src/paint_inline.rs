//! An inline run's elements: each fragment of an inline element (one
//! per line it is on) is its content area, the font's ascent and
//! descent around the line's baseline, as Chromium gives an inline
//! box. The fragments' union is the element's border box; each
//! fragment takes the pointer, and is filled with the element's
//! background behind the text.

use vapors_scene::{Item, Quad};
use vapors_text::{Layout, content_area};

use crate::layout::{BoxRect, InlineRun, InlineSpan};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// Records and paints `run`'s inline elements, laid out as `layout` in
/// the text box at `origin`. Call before the run's text is pushed.
pub(crate) fn paint_spans(
    run: &InlineRun,
    layout: &Layout,
    origin: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    for span in &run.spans {
        let fragments = fragments(span, run, layout, origin, input.scale);
        if let Some(union) = union(&fragments) {
            out.rects.insert(span.node, union);
        }
        let used = &input.used[&span.node];
        if !span.visible {
            continue;
        }
        for &fragment in &fragments {
            if used.takes_pointer {
                out.hits
                    .push((span.node, crate::clip::within(input.clip, fragment)));
                out.cursors.insert(span.node, used.cursor);
            }
            if used.background.a > 0.0 {
                out.scene.push(Item::Quad(Quad::fill(
                    snapped(fragment, input.scale),
                    used.background,
                )));
            }
        }
    }
}

/// The span's fragments in CSS pixels, document coordinates. Each is
/// placed as Chromium places an inline box: the line's half-leading
/// over the block's own font, then the block's ascent to the baseline,
/// less the span's ascent; every ascent and descent rounded to a device
/// pixel.
fn fragments(
    span: &InlineSpan,
    run: &InlineRun,
    layout: &Layout,
    origin: BoxRect,
    scale: f32,
) -> Vec<BoxRect> {
    let (ascent, descent) = metrics(&span.style.for_layout(), scale);
    let (block_ascent, block_descent) = metrics(&run.style.for_layout(), scale);
    let lines: Vec<(f32, f32)> = layout
        .lines()
        .map(|line| (line.top / scale, line.height / scale))
        .collect();
    // An element holding no text still has a box: an empty fragment
    // where it lies, as Chromium gives it.
    let found = if span.range.is_empty() {
        vec![layout.caret(span.range.start)]
    } else {
        layout.fragments(span.range.clone())
    };
    found
        .into_iter()
        .map(|fragment| {
            let (top, height) = lines[fragment.line];
            let baseline = top + (height - block_ascent - block_descent) / 2.0 + block_ascent;
            BoxRect {
                x: origin.x + fragment.x / scale,
                y: origin.y + baseline - ascent,
                width: fragment.width / scale,
                height: ascent + descent,
            }
        })
        .collect()
}

fn metrics(style: &vapors_text::Style<'_>, scale: f32) -> (f32, f32) {
    content_area(&style.family, style.size, scale).unwrap_or_else(|| {
        panic!(
            "{:?} is a bundled face, whose metrics are known",
            style.family
        )
    })
}

fn union(rects: &[BoxRect]) -> Option<BoxRect> {
    let first = *rects.first()?;
    let (mut left, mut top) = (first.x, first.y);
    let (mut right, mut bottom) = (first.x + first.width, first.y + first.height);
    for rect in &rects[1..] {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    }
    Some(BoxRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}
