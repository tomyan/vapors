//! A `calc()` with a % term that does not evaluate at layout refuses the
//! update, naming the element and the property it was declared for.

mod support;

use support::{document, element, refusals};

#[test]
fn a_calc_that_does_not_evaluate_is_refused_naming_its_element_and_property() {
    // Given: a division by zero only seen once % has a basis
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "panel");
    doc.add_stylesheet("app.css", ".panel { width: calc(50% / (1 - 1)); }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("<div class=\"panel\">")
            && errors[0].contains("width")
            && errors[0].contains("division by zero"),
        "{errors:?}"
    );
}
