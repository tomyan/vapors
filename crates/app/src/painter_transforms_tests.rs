//! A frame's shift is whole device pixels before its entry is set.

use super::whole;

#[test]
fn a_whole_shift_is_kept() {
    assert_eq!(whole([3.0, -8.0]), [3.0, -8.0]);
}

#[test]
#[should_panic(expected = "a shift of [0.0, 37.5] device pixels is not whole")]
fn a_fractional_shift_is_refused() {
    let _ = whole([0.0, 37.5]);
}

#[test]
#[should_panic(expected = "is not whole")]
fn a_shift_that_is_not_a_number_is_refused() {
    let _ = whole([f32::NAN, 0.0]);
}

#[test]
#[should_panic(expected = "is not whole")]
fn an_infinite_shift_is_refused() {
    let _ = whole([0.0, f32::INFINITY]);
}
