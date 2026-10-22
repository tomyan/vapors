//! `calc()`, `min()`, `max()` and `clamp()`. Translated from sumi's
//! `runtime/css/calc_test.go`, in px instead of cells, plus the type
//! checking of CSS Values Level 4 section 10 which sumi did not have.

use vapors_css::value::{CalcKind, Context, parse_calc_text};

fn context(percent_base: Option<f64>) -> Context {
    Context {
        font_size: 16.0,
        root_font_size: 16.0,
        viewport_width: 800.0,
        viewport_height: 600.0,
        zero_advance: 8.0,
        percent_base,
    }
}

fn eval(expr: &str, base: Option<f64>) -> f64 {
    let calc = parse_calc_text(expr).unwrap_or_else(|e| panic!("{expr}: {e}"));
    calc.evaluate(&context(base))
        .unwrap_or_else(|e| panic!("{expr}: {e}"))
}

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
