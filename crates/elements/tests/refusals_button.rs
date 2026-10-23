//! What a button refuses until later slices: `disabled="false"`, which
//! disables in HTML, and inline layout outside a flex container.

mod support;

use support::{document, element, refusals};
use vapors_dom::Dom;

#[test]
fn a_disabled_attribute_of_false_is_refused_since_it_disables() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let button = element(&mut doc, body, "button", "");
    doc.set_attribute(button, "disabled", "false");
    doc.add_stylesheet("app.css", "body { display: flex; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("disabled=\"false\"> is disabled"),
        "{errors:?}"
    );
}

#[test]
fn a_button_outside_a_flex_container_is_refused_until_inline_layout() {
    // Given: body is a block container, so the button stays inline-block
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "button", "");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`display: inline-block`") && e.contains("F2c S6")),
        "{errors:?}"
    );
}

#[test]
fn disabled_false_on_an_element_where_disabled_does_nothing_is_not_refused() {
    // Given: HTML ignores disabled on a div
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    doc.set_attribute(div, "disabled", "false");

    // When
    let updated = doc.update(support::viewport(100.0, 100.0));

    // Then
    assert!(updated.is_ok());
}
