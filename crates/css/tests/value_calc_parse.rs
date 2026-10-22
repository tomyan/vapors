//! `calc()` parsing and type checks (`src/value/calc_parse.rs`). Translated from sumi `calc_test.go` where it refused.

mod support;

use vapors_css::value::{CalcKind, parse_calc_text};

#[test]
fn calc_invalid_is_refused() {
    for expr in [
        "calc(",
        "calc(1px +)",
        "calc(x)",
        "blah(1px)",
        "calc(1px+2px)",
        "clamp(1px, 2px)",
    ] {
        assert!(parse_calc_text(expr).is_err(), "{expr:?} should be refused");
    }
}

#[test]
fn calc_division_by_zero_is_refused() {
    assert!(parse_calc_text("calc(1px / 0)").is_err());
}

#[test]
fn calc_type_mismatches_are_refused() {
    for expr in [
        "calc(10px + 5)",
        "calc(10px * 2px)",
        "calc(10 / 2px)",
        "min(1px, 2)",
    ] {
        let err = parse_calc_text(expr).expect_err(expr);
        assert!(
            err.message.contains("type") || err.message.contains("number"),
            "{expr}: {err}"
        );
    }
}

#[test]
fn calc_kinds_follow_the_operands() {
    for (expr, kind) in [
        ("calc(1 + 2)", CalcKind::Number),
        ("calc(1px + 2px)", CalcKind::Length),
        ("calc(10% + 2px)", CalcKind::LengthPercentage),
        ("calc(10% * 2)", CalcKind::Percentage),
    ] {
        assert_eq!(parse_calc_text(expr).expect(expr).kind(), kind, "{expr}");
    }
}

#[test]
fn product_of_two_lengths_anywhere_in_the_chain_is_refused() {
    // Given: the review found `calc(2 * 4px * 3px)` passed and panicked.
    for expr in [
        "calc(2 * 4px * 3px)",
        "calc(1px * 1px)",
        "calc(1px / 1px)",
        "calc(2 * 3px / 1px)",
    ] {
        let err = parse_calc_text(expr).expect_err(expr);
        assert!(
            err.message.contains("length") || err.message.contains("number"),
            "{expr}: {err}"
        );
        assert_eq!(
            (err.position.line, err.position.column > 0),
            (1, true),
            "{expr}"
        );
    }
}

#[test]
fn division_by_a_literal_zero_number_is_refused_too() {
    // sumi's calc_test case.
    assert!(parse_calc_text("calc(1 / 0)").is_err());
}
