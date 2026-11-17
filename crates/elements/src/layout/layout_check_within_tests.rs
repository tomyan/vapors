//! Two layout values agree when they are the same, or both finite and
//! within the summing tolerance; an infinity against a finite value
//! never does.

use super::within;

#[test]
fn values_within_the_tolerance_or_equal_infinities_agree() {
    assert!(within(100.0, 100.0005, 10));
    assert!(within(f32::INFINITY, f32::INFINITY, 10));
}

#[test]
fn an_infinity_against_a_finite_value_disagrees() {
    for (a, b) in [
        (f32::INFINITY, 100.0),
        (100.0, f32::NEG_INFINITY),
        (f32::INFINITY, f32::NEG_INFINITY),
    ] {
        assert!(!within(a, b, 10), "{a} against {b}");
    }
}

#[test]
fn a_nan_or_values_past_the_tolerance_disagree() {
    assert!(!within(f32::NAN, f32::NAN, 10));
    assert!(!within(100.0, 101.0, 10));
}
