//! What `overflow` cannot do yet (F2c S8), refused naming it.

mod support;

use support::{document, element, refusals, text};

#[test]
fn what_overflow_cannot_do_yet_is_refused_naming_it() {
    let cases = [
        ("overflow: scroll;", "F2c S9"),
        ("overflow: auto;", "F2c S9"),
        ("overflow-x: hidden; overflow-y: visible;", "overflow-y"),
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
