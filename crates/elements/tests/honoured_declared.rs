//! A deferred property is refused where it is declared, and nowhere
//! else: `check_honoured` (`style/honoured.rs`) compares with the
//! initial value only the longhands a declaration reached
//! (`ComputedStyle::declared`), so an element with no declarations is
//! checked without reading its values.

mod support;

use support::{document, element, refusals, viewport};

const DEFERRED: &str = "is not implemented by vapors-elements yet";

#[test]
fn a_deferred_property_declared_on_an_element_is_refused_naming_it() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "spinner");
    doc.add_stylesheet("t.css", ".spinner { animation-name: spin; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then: one refusal, at the spinner, for animation-name
    let deferred: Vec<&String> = errors.iter().filter(|e| e.contains(DEFERRED)).collect();
    assert_eq!(deferred.len(), 1, "{errors:?}");
    assert!(
        deferred[0].contains("animation-name") && deferred[0].contains("spinner"),
        "{}",
        deferred[0]
    );
}

#[test]
fn an_element_under_one_with_a_deferred_declaration_is_not_refused_for_it() {
    // Given: the declaration on the parent, a child with none
    let mut doc = document();
    let body = doc.body();
    let parent = element(&mut doc, body, "div", "spinner");
    element(&mut doc, parent, "span", "inside");
    doc.add_stylesheet("t.css", ".spinner { animation-name: spin; }")
        .expect("parses");

    // When
    let errors = refusals(&mut doc);

    // Then: the parent's refusal only
    let deferred: Vec<&String> = errors.iter().filter(|e| e.contains(DEFERRED)).collect();
    assert_eq!(deferred.len(), 1, "{errors:?}");
    assert!(!deferred[0].contains("inside"), "{}", deferred[0]);
}

#[test]
fn a_page_with_no_deferred_declarations_updates() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "plain");
    element(&mut doc, div, "span", "");
    doc.add_stylesheet("t.css", ".plain { color: red; width: 10px; }")
        .expect("parses");

    // Then
    let updated = doc.update(viewport(100.0, 100.0));
    assert!(updated.is_ok(), "{:?}", updated.err());
}
