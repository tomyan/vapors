//! An+B (`src/selector/nth.rs`). Translated from sumi `nth_test.go`.

mod support;

use support::selector;
use vapors_css::selector::{Nth, PseudoClass, parse_selector_list};

#[test]
fn parse_nth_child_argument() {
    // When
    let c = &selector("span:nth-child(2n+1)").compounds[0];

    // Then
    assert_eq!(
        c.pseudo_classes,
        [PseudoClass::NthChild(Nth { a: 2, b: 1 })]
    );
}

#[test]
fn parse_nth_keywords() {
    for (expr, a, b) in [
        ("odd", 2, 1),
        ("even", 2, 0),
        ("3", 0, 3),
        ("n", 1, 0),
        ("2n", 2, 0),
        ("2n+1", 2, 1),
        ("-n+3", -1, 3),
        ("3n-1", 3, -1),
        ("+2n+2", 2, 2),
        ("2n + 1", 2, 1),
        ("-2n - 3", -2, -3),
    ] {
        // When
        let nth = Nth::parse(expr).unwrap_or_else(|e| panic!("{expr}: {e}"));

        // Then
        assert_eq!(nth, Nth { a, b }, "{expr}");
    }
}

#[test]
fn nth_matches() {
    for (expr, index, want) in [
        ("odd", 1, true),
        ("odd", 2, false),
        ("even", 2, true),
        ("3", 3, true),
        ("3", 4, false),
        ("2n+1", 5, true),
        ("-n+3", 3, true),
        ("-n+3", 4, false),
        ("n", 7, true),
    ] {
        // When
        let nth = Nth::parse(expr).expect(expr);

        // Then
        assert_eq!(nth.matches(index), want, "{expr}.matches({index})");
    }
}

#[test]
fn parse_nth_invalid() {
    for expr in ["", "x", "n+n", "2m+1", "2 n"] {
        assert!(Nth::parse(expr).is_err(), "{expr:?} should be refused");
    }
}

#[test]
fn an_plus_b_whitespace_forms_from_the_specification() {
    // Given: review item 13.
    for (expr, a, b) in [
        ("2n +1", 2, 1),
        ("+n+1", 1, 1),
        ("2n+ 1", 2, 1),
        ("-n -2", -1, -2),
    ] {
        assert_eq!(
            Nth::parse(expr).unwrap_or_else(|e| panic!("{expr}: {e}")),
            Nth { a, b },
            "{expr}"
        );
    }
}

#[test]
fn nth_child_of_selector_is_refused_naming_of() {
    let err = parse_selector_list("li:nth-child(2n of .a)").expect_err("refused");
    assert!(err.message.contains("of"), "{err}");
}
