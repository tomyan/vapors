//! GPU tier: the transform table's writes for a framed lane's rows
//! (`docs/plans/flat-scene-f2.md`): an unchanged frame writes none, and a
//! row put in at the top rewrites the lane batch's row range, the rows in
//! view and no more, with the frame's entry as it was.

use crate::support_surface as support;

use support::{Rig, document, update};
use vapors_dom::Dom;
use vapors_elements::Document;

/// Rows 2 CSS px tall: 4 device px each, eight in the 32 px target.
const CSS: &str = ".row { height: 2px; background-color: #f00; } .blue { background-color: #00f; }";

/// Device pixels per CSS pixel.
const SCALE: f32 = 2.0;

/// 40 rows, red and blue in turn, updated.
fn rows() -> Document {
    let rows: Vec<_> = (0..40)
        .map(|at| (None, "div", if at % 2 == 0 { "row" } else { "row blue" }))
        .collect();
    let mut doc = document(CSS, &rows);
    update(&mut doc, SCALE);
    doc
}

/// The table's writes of `doc` drawn by `rig`: entries and bytes.
fn written(rig: &mut Rig, doc: &Document) -> (u64, u64) {
    rig.draw(doc).expect("paints");
    let counts = rig.painter.frame_counts();
    (counts.transform_writes, counts.transform_bytes)
}

#[test]
fn gpu_an_unchanged_frame_of_a_framed_lane_writes_no_row_entry() {
    // Given: drawn once
    let doc = rows();
    let mut rig = Rig::new();
    written(&mut rig, &doc);

    // When
    let idle = written(&mut rig, &doc);

    // Then
    assert!(doc.update_counts().lanes_framed > 0, "the lane is framed");
    assert_eq!(idle, (0, 0));
}

#[test]
fn gpu_a_row_put_in_at_the_top_rewrites_the_rows_in_view_only() {
    // Given: drawn once
    let mut doc = rows();
    let mut rig = Rig::new();
    written(&mut rig, &doc);

    // When: a row put in at the top
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row blue");
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
    update(&mut doc, SCALE);
    let inserted = written(&mut rig, &doc);

    // Then: the eight rows in the 32 px target each at a new offset (the
    // new row, then seven moved down one row), 64 bytes an entry; the
    // lane's frame entry as it was; the 32 rows below the view not written
    assert_eq!(inserted, (8, 8 * 64));
}
