//! A line mixing font sizes or line heights is not laid out as Chromium
//! lays it out yet (its baseline is the maximum over every inline box),
//! so an inline element whose metrics differ from its block's is
//! refused, naming it; one that differs only in weight or face is laid
//! out.

mod support;

use support::{document, element, refusals, text, viewport};

fn paragraph(css: &str) -> vapors_elements::Document {
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "");
    text(&mut doc, p, "Small and ");
    let big = element(&mut doc, p, "span", "big");
    text(&mut doc, big, "big");
    doc.add_stylesheet("app.css", &format!("p {{ font-size: 16px; }} {css}"))
        .expect("parses");
    doc
}

#[test]
fn a_larger_span_in_a_paragraph_is_refused_naming_it() {
    // Given
    let mut doc = paragraph(".big { font-size: 32px; }");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors.iter().any(|e| e.contains("<span class=\"big\">")
            && e.contains("font-size")
            && e.contains("32px")),
        "{errors:?}"
    );
}

#[test]
fn a_span_with_its_own_line_height_is_refused_naming_it() {
    // Given
    let mut doc = paragraph(".big { line-height: 40px; }");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("<span class=\"big\">") && e.contains("line-height")),
        "{errors:?}"
    );
}

#[test]
fn a_span_differing_only_in_weight_and_face_is_laid_out() {
    // Given: Geist Mono has Geist's vertical metrics
    let mut doc = paragraph(".big { font-weight: bold; font-family: monospace; }");

    // When
    let updated = doc.update(viewport(400.0, 100.0));

    // Then
    assert!(updated.is_ok(), "{:?}", updated.err());
}
