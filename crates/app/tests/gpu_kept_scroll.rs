//! GPU tier: a scroll keeps the batches still in view (IU4 4c,
//! `docs/plans/iu4.md` section 3): their bytes, kept in the scroll
//! frame's coordinates, are drawn moved by the frame's new shift; only
//! the batches entering at the view's edge are written; and the pixels
//! are those of the scrolled page drawn by a painter that kept nothing.

#[path = "support/surface.rs"]
mod support;

use support::{RED, Rig, WHITE, document, pixel, update};
use vapors_app::FrameCounts;
use vapors_elements::Document;

/// A list 24 px tall of 8 px rows, each a shadow and a quad: a batch
/// each, so a row's batches are its own.
const CSS: &str = ".list { overflow: auto; width: 24px; height: 24px; } \
    .row { height: 8px; background-color: #f00; box-shadow: 0 0 1px 0 #000; }";

/// The list of `CSS` with ten rows, updated.
fn list() -> Document {
    let rows = [(Some(0), "div", "row"); 10];
    let mut doc = document(CSS, &[[(None, "div", "list")].as_slice(), &rows].concat());
    update(&mut doc, 1.0);
    doc
}

/// `doc`'s list scrolled down by `by` CSS pixels, updated.
fn scrolled(doc: &mut Document, by: f32) {
    let list = doc.children(doc.body())[0];
    assert!(doc.scroll_node_by(list, [0.0, by]), "the list scrolls");
    update(doc, 1.0);
}

/// `doc` drawn by `rig`: the pixels and the frame's counts.
fn drawn(rig: &mut Rig, doc: &Document) -> (Vec<u8>, FrameCounts) {
    let pixels = rig.draw(doc).expect("paints");
    (pixels, rig.painter.frame_counts())
}

#[test]
fn gpu_a_scroll_writes_only_the_batches_entering_the_view() {
    // Given: drawn once
    let mut doc = list();
    let mut rig = Rig::new();
    let (_, first) = drawn(&mut rig, &doc);
    assert_eq!((first.batches_written, first.batches_kept), (10, 0));

    // When: scrolled one row down and drawn again
    scrolled(&mut doc, 8.0);
    let (pixels, counts) = drawn(&mut rig, &doc);

    // Then: rows 0 to 3 showed, row 3 touching the list's bottom edge;
    // scrolled, row 0 touches its top edge and row 4 comes in at the
    // bottom. Row 4's shadow (112 bytes) and quad (128) are written,
    // with the new shift's uniform value for each pipeline (16 each);
    // the white quad on the page, rows 0 to 3 and the thumb are drawn as
    // kept. Below the list (y 28) the rows are clipped away
    let afresh = Rig::new().draw(&doc).expect("paints");
    assert!(pixels == afresh, "the kept batches drew other pixels");
    assert_eq!([pixel(&pixels, 4, 4), pixel(&pixels, 4, 28)], [RED, WHITE]);
    assert_eq!((counts.batches_written, counts.batches_kept), (2, 10));
    let bytes = (counts.shadows.buffer_bytes, counts.quads.buffer_bytes);
    assert_eq!(bytes, (112 + 16, 128 + 16), "{counts:?}");
}
