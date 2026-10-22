//! Specificity (`src/selector/specificity.rs`). Translated from sumi `selector_test.go` and `logical_test.go`.

mod support;

use support::selector;
use vapors_css::selector::Specificity;

#[test]
fn specificity_ordering() {
    for (lower, higher) in [
        ("span", ".title"),
        (".title", "#main"),
        (".a", ".a.b"),
        ("span", "div span"),
        ("*", "span"),
    ] {
        // When / Then
        assert!(
            selector(lower).specificity() < selector(higher).specificity(),
            "specificity({lower}) should be < specificity({higher})"
        );
    }
}

#[test]
fn specificity_is_the_id_class_type_triple() {
    // When / Then
    assert_eq!(
        selector("div#a.b.c[d]:hover").specificity(),
        Specificity {
            ids: 1,
            classes: 4,
            types: 1
        }
    );
}

#[test]
fn attribute_selector_specificity_counts_as_class() {
    assert!(selector("div").specificity() < selector("[focusable]").specificity());
}

#[test]
fn structural_pseudo_counts_in_specificity() {
    assert!(selector("span").specificity() < selector("span:first-child").specificity());
}

#[test]
fn where_has_zero_specificity_and_is_keeps_its_argument() {
    assert_eq!(
        selector(":where(.a #b)").specificity(),
        Specificity::default()
    );
    assert_eq!(
        selector(":is(.a, #b)").specificity(),
        Specificity {
            ids: 1,
            classes: 0,
            types: 0
        }
    );
    assert_eq!(
        selector("span:not(.a.b, span)").specificity(),
        Specificity {
            ids: 0,
            classes: 2,
            types: 1
        }
    );
    assert_eq!(
        selector("div:has(> .x)").specificity(),
        Specificity {
            ids: 0,
            classes: 1,
            types: 1
        }
    );
}
