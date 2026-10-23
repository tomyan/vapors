//! `:global(list)`: for a component's scoped style, the part of a
//! selector left unscoped (vapors-elements does the scoping). The
//! engine itself matches and weighs it as `:is(list)`.

mod support;

use support::{matches_chain, refusal, selector};
use vapors_css::selector::PseudoClass;

#[test]
fn global_parses_its_selector_list() {
    // When
    let compound = &selector(":global(body, .x)").compounds[0];

    // Then
    assert!(
        matches!(&compound.pseudo_classes[..], [PseudoClass::Global(list)] if list.len() == 2),
        "{compound:?}"
    );
}

#[test]
fn global_matches_as_is_does() {
    // Then
    assert!(matches_chain(":global(.x) p", &["div.x", "p"]));
    assert!(!matches_chain(":global(.y) p", &["div.x", "p"]));
}

#[test]
fn global_weighs_its_most_specific_argument() {
    // Then
    assert_eq!(
        selector(":global(#a, .b)").specificity(),
        selector("#a").specificity()
    );
}

#[test]
fn global_without_an_argument_is_refused() {
    // When
    let error = refusal(":global { color: red; }");

    // Then
    assert!(
        error.message.contains(":global needs an argument"),
        "{error}"
    );
}
