//! Inline elements (F2c S6): their text joins the block's one paragraph,
//! a point over one hits it, its background is painted behind the
//! text, and what is not laid out yet is refused.

mod support;

use support::{document, element, refusals, text, viewport};
use vapors_elements::Document;
use vapors_scene::{Color, Item};

/// `<p>Say <b>hello</b>   there</p>` with `css`, laid out 400 px wide.
fn paragraph(css: &str) -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "");
    text(&mut doc, p, "Say ");
    let bold = element(&mut doc, p, "b", "");
    text(&mut doc, bold, "hello");
    text(&mut doc, p, "   there");
    doc.add_stylesheet(
        "app.css",
        &format!("body {{ margin: 0; }} p {{ margin: 0; }} {css}"),
    )
    .expect("parses");
    (doc, bold)
}

#[test]
fn an_inline_elements_text_is_part_of_one_paragraph_with_spaces_collapsed() {
    // Given
    let (mut doc, _bold) = paragraph("");

    // When
    let painted = doc.update(viewport(400.0, 100.0)).expect("updates");

    // Then: one text run holding all of it
    let texts: Vec<&str> = painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(painted.text(*text).text()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["Say hello there"]);
}

#[test]
fn a_point_over_an_inline_elements_text_hits_it_and_beside_it_hits_the_block() {
    // Given
    let (mut doc, bold) = paragraph("");
    let painted = doc.update(viewport(400.0, 100.0)).expect("updates");
    let rect = painted.border_box(bold).expect("the b has a box");

    // When
    let inside = painted.hit([rect.x + rect.width / 2.0, rect.y + rect.height / 2.0]);
    let before = painted.hit([rect.x - 2.0, rect.y + rect.height / 2.0]);

    // Then
    assert_eq!(inside, Some(bold));
    assert_ne!(before, Some(bold));
}

#[test]
fn an_inline_elements_background_is_painted_behind_its_text() {
    // Given
    let (mut doc, bold) = paragraph("b { background-color: #ffff00; }");

    // When
    let painted = doc.update(viewport(400.0, 100.0)).expect("updates");

    // Then: a yellow quad over the b's box, then the text
    let rect = painted.border_box(bold).expect("the b has a box");
    let items = painted.scene().items();
    let quad = items
        .iter()
        .position(|item| matches!(item, Item::Quad(q) if q.background == Color::from_rgba8(255, 255, 0, 255)))
        .expect("a yellow quad");
    let text = items
        .iter()
        .position(|item| matches!(item, Item::Text { .. }))
        .expect("the text");
    assert!(quad < text, "{items:?}");
    let Item::Quad(q) = &items[quad] else {
        unreachable!()
    };
    assert_eq!(
        (q.rect.left, q.rect.right),
        (rect.x.round(), (rect.x + rect.width).round())
    );
}

#[test]
fn a_block_inside_an_inline_element_is_refused_naming_it() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let span = element(&mut doc, body, "span", "");
    element(&mut doc, span, "div", "inner");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("<div class=\"inner\">") && e.contains("inside an inline element")),
        "{errors:?}"
    );
}

#[test]
fn padding_on_an_inline_element_is_refused_naming_the_side() {
    // Given
    let (mut doc, _bold) = paragraph("b { padding-left: 4px; }");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.contains("<b>")
            && e.contains("padding-left")
            && e.contains("inline element")),
        "{errors:?}"
    );
}
