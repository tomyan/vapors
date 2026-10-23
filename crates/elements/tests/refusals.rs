//! What the tree does not implement yet is refused by update, naming
//! the element, the property or tag, and the slice that brings it.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::{Dom, Event};

#[test]
fn a_tag_from_a_later_slice_is_refused_naming_the_slice() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "span", "");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("<span>") && errors[0].contains("F2c S6"),
        "{errors:?}"
    );
}

#[test]
fn a_removed_element_is_no_longer_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let span = element(&mut doc, body, "span", "");

    // When
    doc.remove(span);

    // Then
    assert!(doc.update(viewport(100.0, 100.0)).is_ok());
}

#[test]
fn a_deferred_property_off_its_initial_value_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "card");
    doc.add_stylesheet("app.css", ".card { transform: translate(1px, 2px); }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert!(
        error.contains("<div class=\"card\">")
            && error.contains("transform")
            && error.contains("after F2c"),
        "{error}"
    );
}

#[test]
fn a_value_the_tree_does_not_lay_out_yet_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "shell");
    doc.add_stylesheet("app.css", ".shell { display: contents; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`display: contents`") && e.contains("not scheduled")),
        "{errors:?}"
    );
}

#[test]
fn a_family_vapors_does_not_bundle_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "p", "");
    doc.add_stylesheet("app.css", "p { font-family: Inter, sans-serif; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("font-family") && e.contains("does not bundle")),
        "{errors:?}"
    );
}

#[test]
fn a_stylesheet_that_does_not_parse_is_refused_naming_the_sheet_and_line() {
    // Given
    let mut doc = document();

    // When
    let error = doc
        .add_stylesheet("app.css", "div {\n  colour: red;\n}")
        .expect_err("refused");

    // Then
    assert!(error.to_string().starts_with("app.css:2:"), "{error}");
}

#[test]
fn every_refusal_in_one_update_is_reported_together() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "span", "");
    element(&mut doc, body, "div", "card");
    doc.add_stylesheet("app.css", ".card { opacity: 0.5; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 2, "{errors:?}");
}

#[test]
fn a_refused_update_drops_the_previous_result() {
    // Given
    let mut doc = document();
    doc.update(viewport(100.0, 100.0))
        .expect("an empty document updates");
    let body = doc.body();
    element(&mut doc, body, "span", "");

    // When
    let _ = doc.update(viewport(100.0, 100.0));

    // Then
    assert!(doc.painted().is_none());
}

#[test]
fn a_listener_for_an_event_the_tree_never_fires_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "panel");
    doc.listen(div, "click", Box::new(|_: &mut Event| {}));
    doc.listen(div, "mouseover", Box::new(|_: &mut Event| {}));

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("<div class=\"panel\">: onmouseover")
            && errors[0].contains("never fires"),
        "{errors:?}"
    );
}
