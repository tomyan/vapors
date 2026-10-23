//! A refusal names where its value was declared (sheet, line and
//! column), and an inherited value is refused once, at the element that
//! declared it, not at every descendant.

mod support;

use support::{document, element, refusals};

#[test]
fn a_refusal_names_the_sheet_line_and_column_of_the_declaration() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "card");
    doc.add_stylesheet("app.css", "div {}\n.card {\n  opacity: 0.5;\n}")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("app.css:3:3: <div class=\"card\">: `opacity: 0.5`"),
        "{errors:?}"
    );
}

#[test]
fn an_inherited_refused_value_is_refused_once_at_the_element_that_declared_it() {
    // Given: font-family declared on .outer, inherited by two descendants
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "outer");
    let inner = element(&mut doc, outer, "div", "inner");
    element(&mut doc, inner, "p", "");
    doc.add_stylesheet("app.css", ".outer { font-family: Inter; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("app.css:1:10: <div class=\"outer\">: `font-family: Inter`"),
        "{errors:?}"
    );
}

#[test]
fn an_inherited_deferred_property_is_refused_once_at_its_declarer() {
    // Given: font-feature-settings (deferred, inherited) on .outer
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "outer");
    element(&mut doc, outer, "div", "inner");
    doc.add_stylesheet("app.css", ".outer { font-feature-settings: \"liga\" 0; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("<div class=\"outer\">") && errors[0].contains("font-feature-settings"),
        "{errors:?}"
    );
}
