//! What `overflow` cannot do yet (F2c S8, S9), refused naming it.

use crate::support;

use support::{document, element, refusals, text};
use vapors_dom::Dom;

#[test]
fn what_overflow_cannot_do_yet_is_refused_naming_it() {
    let cases = [
        ("overflow-x: clip; overflow-y: visible;", "overflow-y"),
        ("overflow: hidden; border-radius: 4px;", "border-radius"),
    ];
    for (css, named) in cases {
        // Given
        let mut doc = document();
        let body = doc.body();
        element(&mut doc, body, "div", "frame");
        doc.add_stylesheet("app.css", &format!(".frame {{ {css} }}"))
            .expect("parses");

        // When
        let errors = refusals(&mut doc);

        // Then
        assert!(
            errors
                .iter()
                .any(|e| e.contains("overflow") && e.contains(named)),
            "{css}: {errors:?}"
        );
    }
}

#[test]
fn overflow_on_an_inline_element_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "");
    let span = element(&mut doc, p, "span", "cut");
    text(&mut doc, span, "word");
    doc.add_stylesheet("app.css", ".cut { overflow: hidden; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("<span class=\"cut\">") && e.contains("overflow")),
        "{errors:?}"
    );
}

fn refused_with(css: &str) -> Vec<String> {
    let mut doc = document();
    doc.add_stylesheet("app.css", css).expect("parses");
    refusals(&mut doc)
}

#[test]
fn scrolling_the_viewport_is_refused() {
    for css in [
        "html { overflow: auto; }",
        "body { overflow: scroll; }",
        "html { overflow: visible; } body { overflow: auto; }",
    ] {
        // When
        let errors = refused_with(css);

        // Then
        assert!(
            errors
                .iter()
                .any(|e| e.contains("overflow") && e.contains("viewport")),
            "{css}: {errors:?}"
        );
    }
}

#[test]
fn a_body_scrolls_when_the_root_takes_the_viewport() {
    // Given: html's overflow is the viewport's, so body's is its own
    let mut doc = document();
    doc.add_stylesheet(
        "app.css",
        "html { overflow: hidden; } body { overflow: auto; }",
    )
    .expect("parses");

    // When
    let updated = doc.update(support::viewport(400.0, 300.0));

    // Then
    assert!(updated.is_ok(), "{:?}", updated.err());
}

#[test]
fn a_number_field_that_scrolls_is_refused_naming_its_declaration() {
    // Given: a number field given `overflow: auto` (a text field's
    // text scrolls, `field_scroll.rs`; a number field's does not yet)
    let mut doc = document();
    let body = doc.body();
    let field = element(&mut doc, body, "input", "field");
    doc.set_attribute(field, "type", "number");
    doc.add_stylesheet("app.css", ".field { width: 6em; overflow: auto; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.starts_with("app.css:1")
            && e.contains("overflow-x")
            && e.contains("number field")),
        "{errors:?}"
    );
}

#[test]
fn a_rounded_text_field_whose_padding_does_not_clear_its_corners_is_refused() {
    // Given: an 8 px radius over a 1 px border needs 7 px side padding
    let mut doc = document();
    let body = doc.body();
    let field = element(&mut doc, body, "input", "field");
    doc.set_attribute(field, "type", "text");
    let css = ".field { display: block; width: 6em; border-radius: 8px; padding: 0 6px; }";
    doc.add_stylesheet("app.css", css).expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("border-radius") && e.contains("side padding")),
        "{errors:?}"
    );
}

#[test]
fn a_rounded_text_field_that_does_not_scroll_is_refused_whatever_its_padding() {
    // Given: padding that clears the corners, but `overflow: hidden`,
    // whose clip is the square padding box
    let mut doc = document();
    let body = doc.body();
    let field = element(&mut doc, body, "input", "field");
    doc.set_attribute(field, "type", "text");
    let css = ".field { display: block; width: 6em; border-radius: 8px; padding: 0 7px; overflow: hidden; }";
    doc.add_stylesheet("app.css", css).expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.contains("border-radius")
            && e.contains("or the clip")
            && !e.contains("side padding")),
        "{errors:?}"
    );
}

#[test]
fn a_rounded_text_field_whose_padding_clears_its_corners_is_drawn() {
    // Given: 7 px side padding clears an 8 px radius over a 1 px border
    let mut doc = document();
    let body = doc.body();
    let field = element(&mut doc, body, "input", "field");
    doc.set_attribute(field, "type", "text");
    let css = ".field { display: block; width: 6em; border-radius: 8px; padding: 0 7px; }";
    doc.add_stylesheet("app.css", css).expect("parses");

    // Then
    doc.update(support::viewport(400.0, 300.0))
        .expect("updates");
}
