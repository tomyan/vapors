//! Positioned elements (F2c S8) beyond what the layout corpus compares
//! with Chromium: what is refused.

mod support;

use support::{document, element, refusals, text};

#[test]
fn a_positioned_inline_element_is_refused_naming_it() {
    // Given: an inline element has no box of its own to place
    let mut doc = document();
    let body = doc.body();
    let p = element(&mut doc, body, "p", "");
    let span = element(&mut doc, p, "span", "badge");
    text(&mut doc, span, "new");
    doc.add_stylesheet("app.css", ".badge { position: relative; top: -4px; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("<span class=\"badge\">") && e.contains("position")),
        "{errors:?}"
    );
}

#[test]
fn sticky_is_refused_naming_the_scrolling_slice() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "bar");
    doc.add_stylesheet("app.css", ".bar { position: sticky; top: 0; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("sticky") && e.contains("F2c S9")),
        "{errors:?}"
    );
}
