//! Pseudo-classes (`src/selector/pseudo.rs`): parsing and refusals.

mod support;

use support::{refusal, selector};
use vapors_css::selector::{PseudoClass, parse_selector_list};

#[test]
fn parse_structural_pseudo_separated_from_state() {
    // When
    let c = &selector("span:first-child:hover").compounds[0];

    // Then
    assert_eq!(
        c.pseudo_classes,
        [PseudoClass::FirstChild, PseudoClass::Hover]
    );
}

#[test]
fn unknown_pseudo_class_is_refused_naming_it() {
    // Given: sumi let `:future-thing` parse and never match.
    let err = refusal("span:future-thing { color: red; }");

    // Then
    assert_eq!(err.position.column, 5);
    assert!(err.message.contains("future-thing"), "{err}");
}

#[test]
fn pseudo_elements_are_refused_as_unsupported() {
    for text in [".item::before", "a::after", "li:before"] {
        let err = refusal(&format!("{text} {{ color: cyan; }}"));
        assert!(err.message.contains("pseudo-element"), "{text}: {err}");
    }
}

#[test]
fn has_nested_at_any_depth_is_refused() {
    // Given: review item 14.
    for text in [
        ":has(:is(:has(a)))",
        ":has(.a :not(:has(b)))",
        ":has(:is(.x, :where(:has(c))))",
    ] {
        let err = parse_selector_list(text).expect_err(text);
        assert!(
            err.message.contains(":has() inside :has()"),
            "{text}: {err}"
        );
    }
}
