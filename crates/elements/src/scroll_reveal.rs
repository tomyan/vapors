//! A text field's scroll offset follows its caret (entry 44), as
//! Chromium 157's inner editor does: when the caret moves or the
//! field's laid-out content width changes, the field scrolls on x just
//! far enough that the caret's 1 px bar is in the content box; when the
//! field loses focus it scrolls back to its start. A wheel scroll in
//! between is kept until one of those changes. The range on x leaves
//! room for the caret after the text ([`field_reach`]).
//!
//! Not yet: a selection's moving end is not revealed (Chromium reveals
//! it); the offset is kept while a selection shows.

use crate::layout::{BoxRect, LayoutBox};

/// The caret's width, CSS px: `text_paint.rs` draws it 1 px wide.
pub(crate) const CARET_WIDTH: f32 = 1.0;

/// What a field's offset was last revealed for: `None` unfocused;
/// focused, its caret's x from the text's origin and its content
/// width, as bits, or `None` while a selection shows.
pub(crate) type Revealed = Option<Option<[u32; 2]>>;

/// `at`, the offset of the scroll container in box `index`, moved to
/// show a text field's caret when the caret or its content width
/// differs from `last`, or to its start when it lost focus; with what
/// it is now revealed for. Any other box's comes back unmoved.
pub(crate) fn revealed(
    boxes: &[LayoutBox],
    index: usize,
    at: [f32; 2],
    last: Revealed,
) -> ([f32; 2], Revealed) {
    if !scrolls_text(&boxes[index]) {
        return (at, None);
    }
    let now = caret(boxes, index);
    if now == last {
        return (at, last);
    }
    let x = match now {
        None => 0.0,
        Some(None) => at[0],
        Some(Some([x, width])) => shown(at[0], f32::from_bits(x), f32::from_bits(width)),
    };
    ([x, at[1]], now)
}

/// The nearest offset to `at` that shows a caret at `x` in a content
/// box `width` wide.
fn shown(at: f32, x: f32, width: f32) -> f32 {
    at.max(x + CARET_WIDTH - width).min(x)
}

/// The field's caret now, as [`Revealed`] holds it.
fn caret(boxes: &[LayoutBox], index: usize) -> Revealed {
    let marks = run(boxes, index).text_marks.as_ref()?;
    let width = content_width(&boxes[index]);
    Some(
        marks
            .caret
            .map(|caret| [caret[0].to_bits(), width.to_bits()]),
    )
}

/// Whether box `layout_box` is a text field whose text scrolls on x
/// (the user-agent sheet's `overflow-x: auto`): cut off at its content
/// box (`clip.rs`), its range its text's (`scroll.rs`), its offset
/// following its caret. A number field's does not (refused,
/// `style/overflow.rs`).
pub(crate) fn scrolls_text(layout_box: &LayoutBox) -> bool {
    layout_box.text_field && layout_box.style.overflow.x == taffy::Overflow::Scroll
}

/// The run of the field in box `index`: its text, its first child.
///
/// # Panics
/// When it has none: every field's text is its run, a blank one a
/// no-break space (`layout/field_box.rs` `mark_run`).
fn run(boxes: &[LayoutBox], index: usize) -> &LayoutBox {
    let run = boxes[index].children.first().unwrap_or_else(|| {
        panic!("field box {index} has no run: every field's text is its run (layout/field_box.rs `mark_run`); reproduce with `cargo xtask test`")
    });
    let run = &boxes[*run];
    assert!(
        run.run_of_field.is_some(),
        "field box {index}'s first child is not its run: every field's text is its first child (layout/field_box.rs `mark_run`); reproduce with `cargo xtask test`"
    );
    run
}

/// The width of a field's content box, CSS px (`clip::content_box`).
fn content_width(field: &LayoutBox) -> f32 {
    let size = field.layout.size;
    let border_box = BoxRect {
        x: 0.0,
        y: 0.0,
        width: size.width,
        height: size.height,
    };
    crate::clip::content_box(field, border_box).width
}

/// How far the text field in box `index` scrolls on x: its text's
/// width, its run's overflow as laid out (the run itself is as wide as
/// the content box), and the caret's after it, past its content box;
/// 0 when they fit.
pub(crate) fn field_reach(boxes: &[LayoutBox], index: usize) -> f32 {
    let text = run(boxes, index).layout.scrollable_overflow_rect.right;
    (text + CARET_WIDTH - content_width(&boxes[index])).max(0.0)
}
