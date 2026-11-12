use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::painted::Painted;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A document with inputs as blocks, `.gone` hidden, and `tag` (with `kind` as its type,
/// when given) in the body, updated once; the element.
fn shown(tag: &str, kind: Option<&str>) -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "input { display: block; width: 80px; } .gone { display: none; }",
    )
    .expect("parses");
    let node = doc.create_element(tag);
    if let Some(kind) = kind {
        doc.set_attribute(node, "type", kind);
    }
    let body = doc.body();
    doc.append(body, node);
    doc.update(VIEWPORT).expect("updates");
    (doc, node)
}

/// `node` hidden, and the update after.
fn hidden(doc: &mut Document, node: NodeId) -> &Painted {
    doc.set_attribute(node, "class", "gone");
    doc.update(VIEWPORT).expect("updates")
}

#[test]
fn a_hidden_range_loses_its_track() {
    // Given
    let (mut doc, range) = shown("input", Some("range"));
    assert!(
        doc.painted
            .as_ref()
            .expect("painted")
            .ranges
            .contains_key(&range)
    );

    // When
    let painted = hidden(&mut doc, range);

    // Then
    assert!(!painted.ranges.contains_key(&range));
}

#[test]
fn a_hidden_text_field_loses_its_text_origin() {
    // Given
    let (mut doc, field) = shown("input", None);
    let painted = doc.painted.as_ref().expect("painted");
    assert!(painted.text_origins.contains_key(&field));

    // When
    let painted = hidden(&mut doc, field);

    // Then
    assert!(!painted.text_origins.contains_key(&field));
}

#[test]
fn a_hidden_inline_element_loses_its_border_box() {
    // Given: a span in a div's text run
    let (mut doc, div) = shown("div", None);
    let span = doc.create_element("span");
    let (before, inside) = (doc.create_text("a "), doc.create_text("span"));
    doc.append(div, before);
    doc.append(div, span);
    doc.append(span, inside);
    doc.update(VIEWPORT).expect("updates");
    assert!(
        doc.painted
            .as_ref()
            .expect("painted")
            .rects
            .contains_key(&span)
    );

    // When
    let painted = hidden(&mut doc, span);

    // Then
    assert!(!painted.rects.contains_key(&span));
}
