//! GPU tier: text in a framed lane's rows, drawn from kept bytes through
//! the transform table (`docs/plans/flat-scene-f2.md`): after a row is put
//! in at the top, each row's text is where a painter that kept nothing
//! draws it.

use crate::support_surface as support;

use support::{Rig, document, update};
use vapors_dom::Dom;
use vapors_elements::Document;

/// Rows 8 CSS px tall, each a label in 7 px text: four in the 32 px
/// target.
const CSS: &str = ".row { height: 8px; font-size: 7px; line-height: 8px; color: #000; }";

/// 40 rows, each labelled with its number, updated at scale 1.
fn labelled_rows() -> Document {
    let rows: Vec<_> = (0..40).map(|_| (None, "div", "row")).collect();
    let mut doc = document(CSS, &rows);
    let body = doc.body();
    for (at, row) in doc.children(body).into_iter().enumerate() {
        let label = doc.create_text(&format!("Row {at}"));
        doc.append(row, label);
    }
    update(&mut doc, 1.0);
    doc
}

#[test]
fn gpu_text_in_rows_put_after_an_insert_is_where_a_fresh_painter_draws_it() {
    // Given: drawn once
    let mut doc = labelled_rows();
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: a labelled row put in at the top, drawn from the kept bytes
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    let label = doc.create_text("New");
    doc.append(row, label);
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
    update(&mut doc, 1.0);
    let kept = rig.draw(&doc).expect("paints");

    // Then: the lane is framed, and the pixels are a fresh painter's
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    let fresh = Rig::new().draw(&doc).expect("paints");
    assert!(kept.iter().any(|&v| v < 128), "the text drew nothing");
    assert!(kept == fresh, "kept text in rows differs from a fresh draw");
}
