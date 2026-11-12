//! A run is shaped once per text, styles, visibility and scale,
//! whichever node holds it: a page of equal labels shapes one, and
//! each width layout asks for is broken from that one shaping.

mod support;

use support::{document, element, text, viewport};
use vapors_elements::Document;

/// `count` flex rows, each holding `label` in a `b` in a span and a
/// distinct number in another span: two runs per row, the label's
/// with an inline element of its own row.
fn rows(count: usize, label: &str) -> Document {
    let mut doc = document();
    doc.add_stylesheet("rows.css", ".row { display: flex; gap: 8px; }")
        .expect("parses");
    let body = doc.body();
    for index in 0..count {
        let row = element(&mut doc, body, "div", "row");
        let span = element(&mut doc, row, "span", "");
        let bold = element(&mut doc, span, "b", "");
        text(&mut doc, bold, label);
        let number = element(&mut doc, row, "span", "");
        text(&mut doc, number, &format!("{index:05}"));
    }
    doc
}

#[test]
fn a_page_of_a_thousand_equal_labels_shapes_the_label_once() {
    // Given
    let mut doc = rows(1000, "Edit");

    // When
    doc.update(viewport(400.0, 600.0)).expect("updates");

    // Then: the label once, and each row's number once
    assert_eq!(doc.texts_shaped(), 1 + 1000);
}

#[test]
fn a_run_laid_out_at_several_widths_is_shaped_once() {
    // Given: a flex item, which Taffy measures at min- and max-content
    let mut doc = rows(1, "Row 1");

    // When
    doc.update(viewport(400.0, 600.0)).expect("updates");

    // Then
    assert_eq!(doc.texts_shaped(), 2);
    assert!(
        doc.text_layouts_built() > doc.texts_shaped(),
        "{}",
        doc.text_layouts_built()
    );
}
