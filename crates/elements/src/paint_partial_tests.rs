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

/// A document of `count` scroll containers, each holding a taller
/// block of text, updated once; the first container's text node.
fn scrollers(count: usize) -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        ".scroller { overflow: auto; height: 10px; } .tall { height: 40px; }",
    )
    .expect("parses");
    let body = doc.body();
    let texts: Vec<NodeId> = (0..count)
        .map(|_| {
            let (scroller, tall) = (doc.create_element("div"), doc.create_element("div"));
            doc.set_attribute(scroller, "class", "scroller");
            doc.set_attribute(tall, "class", "tall");
            let text = doc.create_text("12345");
            doc.append(body, scroller);
            doc.append(scroller, tall);
            doc.append(tall, text);
            text
        })
        .collect();
    doc.update(VIEWPORT).expect("updates");
    (doc, texts[0])
}

/// The scene items an edit of `text` to a text of the same length built.
fn items_built_by_edit(doc: &mut Document, text: NodeId) -> usize {
    doc.set_text(text, "54321");
    doc.update(VIEWPORT).expect("updates").items_built
}

#[test]
fn an_edit_inside_a_scroll_container_paints_no_other_containers_thumbs() {
    // Given
    let (mut one, one_text) = scrollers(1);
    let (mut many, many_text) = scrollers(5);

    // When
    let built_with_one = items_built_by_edit(&mut one, one_text);
    let built_with_many = items_built_by_edit(&mut many, many_text);

    // Then
    assert_eq!(built_with_many, built_with_one);
}

#[test]
fn an_edit_inside_a_scroll_container_works_out_no_scroll_frame() {
    // Given
    let (mut doc, text) = scrollers(5);

    // When
    doc.set_text(text, "54321");
    let painted = doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(painted.frames_worked, 0);
/// A 104 px scroll container at the top of the page whose content is
/// 1,000 px taller, and a text after it, updated once: its thumb is 24
/// px long on a 100 px track, so it moves 0.076 px per px scrolled.
fn tall_scroller() -> (Document, NodeId, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .scroller { overflow: auto; height: 104px; } \
         .tall { height: 1104px; }",
    )
    .expect("parses");
    let body = doc.body();
    let [scroller, tall, after] = ["div"; 3].map(|tag| doc.create_element(tag));
    doc.set_attribute(scroller, "class", "scroller");
    doc.set_attribute(tall, "class", "tall");
    let text = doc.create_text("12345");
    doc.append(body, scroller);
    doc.append(scroller, tall);
    doc.append(body, after);
    doc.append(after, text);
    doc.update(VIEWPORT).expect("updates");
    (doc, scroller, text)
}

#[test]
fn a_sub_pixel_scroll_that_moves_only_the_thumb_paints_the_thumb() {
    // Given: at 6.55 px the offset applies as 7 and the thumb's top is
    // 2.498, drawn at 2
    let (mut doc, scroller, text) = tall_scroller();
    assert!(doc.scroll_to(scroller, [0.0, 6.55]));
    doc.update(VIEWPORT).expect("updates");

    // When: at 6.6 px the offset still applies as 7, the thumb's top is
    // 2.502, drawn at 3; a debug build compares each paint with a fresh one
    assert!(doc.scroll_to(scroller, [0.0, 6.6]));
    let built = doc.update(VIEWPORT).expect("updates").items_built;
    doc.set_text(text, "54321");
    doc.update(VIEWPORT).expect("updates");

    // Then: the thumb, its one item, was painted again
    assert_eq!(built, 1);
}
