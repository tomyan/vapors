//! `@supports` (`src/cascade/supports.rs`). Translated from sumi `resolve_styles_test.go`; the value is checked too.

mod support;

use support::{computed, css, refusal};

#[test]
fn supports_checks_property_and_value() {
    // Given: sumi checked the property name only; vapors parses the value.
    let (tree, leaf) = support::chain(&["span.x"]);
    let src = "@supports (display: grid) { .x { color: red; } }
        @supports (display: table-cell) { .x { color: blue; } }
        @supports (border-title: x) { .x { color: green; } }
        @supports (--custom: anything) { .x { opacity: 0.5; } }
        @supports not (display: grid) { .x { width: 1px; } }";
    let style = computed(src, &tree, leaf);
    assert_eq!(css(&style, "color"), "rgb(255 0 0)");
    assert_eq!(css(&style, "opacity"), "0.5");
    assert_eq!(css(&style, "width"), "auto");
}

#[test]
fn supports_with_and_or_and_malformed_refused() {
    let (tree, leaf) = support::chain(&["span.x"]);
    let src = "@supports (display: grid) and (color: red) { .x { color: red; } } @supports (display: grid) or (nope: 1) { .x { opacity: 0.5; } }";
    let style = computed(src, &tree, leaf);
    assert_eq!(css(&style, "color"), "rgb(255 0 0)");
    assert_eq!(css(&style, "opacity"), "0.5");
    assert!(
        refusal("@supports display: grid { }")
            .message
            .contains("@supports")
    );
}

#[test]
fn supports_selector_function_is_refused_naming_it() {
    let err = refusal("@supports selector(:has(a)) { .x { color: red; } }");
    assert!(err.message.contains("selector()"), "{err}");
}
