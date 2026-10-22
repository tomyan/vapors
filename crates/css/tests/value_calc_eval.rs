//! `calc()` evaluation (`src/value/calc_eval.rs`). Translated from sumi `calc_test.go`, in px.

mod support;

use support::value_calc::{context, eval};
use vapors_css::value::parse_calc_text;

#[test]
fn calc_arithmetic() {
    for (expr, want) in [
        ("calc(10px + 5px)", 15.0),
        ("calc(10px - 4px)", 6.0),
        ("calc(3px * 4)", 12.0),
        ("calc(10px / 2)", 5.0),
        ("calc(2px + 3px * 4)", 14.0),
        ("calc((2px + 3px) * 4)", 20.0),
        ("calc(10 + 5)", 15.0),
        ("calc(1em + 2px)", 18.0),
        ("calc(1rem + 50vw + 1ch)", 16.0 + 400.0 + 8.0),
    ] {
        assert_eq!(eval(expr, None), want, "{expr}");
    }
}

#[test]
fn calc_percent_against_base() {
    assert_eq!(eval("calc(100% - 10px)", Some(80.0)), 70.0);
    assert_eq!(eval("calc(50% + 2px)", Some(40.0)), 22.0);
}

#[test]
fn calc_percent_without_base_is_unresolvable() {
    // When
    let calc = parse_calc_text("calc(50% - 1px)").expect("parses");
    let err = calc.evaluate(&context(None)).unwrap_err();

    // Then
    assert!(err.to_string().contains('%'), "{err}");
}

#[test]
fn min_max_clamp() {
    for (expr, base, want) in [
        ("min(10px, 20px)", None, 10.0),
        ("max(10px, 20px, 15px)", None, 20.0),
        ("clamp(10px, 5px, 30px)", None, 10.0),
        ("clamp(10px, 50px, 30px)", None, 30.0),
        ("clamp(10px, 20px, 30px)", None, 20.0),
        ("min(50%, 30px)", Some(80.0), 30.0),
        ("max(50%, 30px)", Some(80.0), 40.0),
    ] {
        assert_eq!(eval(expr, base), want, "{expr}");
    }
}

#[test]
fn nested_calc_functions() {
    assert_eq!(eval("calc(min(10px, 20px) + max(1px, 2px))", None), 12.0);
}

#[test]
fn clamp_with_min_above_max_returns_min() {
    assert_eq!(eval("clamp(30px, 20px, 10px)", None), 30.0);
}
