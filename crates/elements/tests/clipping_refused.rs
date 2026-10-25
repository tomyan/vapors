//! What `overflow` cannot do yet (F2c S8, S9), refused naming it.

mod support;

use support::{document, element, refusals, text};

#[test]
fn what_overflow_cannot_do_yet_is_refused_naming_it() {
    let cases = [
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
