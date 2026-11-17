//! The a11y tree's own refusal of bounds that are not finite, by role
//! and name, not written as null. Paint refuses such a box first (issue
//! #449, `tests/paint_finite.rs`), so an update never reaches this; it
//! stays as the tree's guard for bounds from any other source.

use super::finite;
use crate::{AxError, BoxRect};

#[test]
fn bounds_that_are_not_finite_are_refused_by_role_and_name() {
    // Given
    let bounds = BoxRect {
        x: 26.0,
        y: 24.0,
        width: f32::INFINITY,
        height: 24.0,
    };

    // When
    let refused = finite(bounds, "button", "Pin");

    // Then
    let Err(AxError::NotFinite { role, name, bounds }) = refused else {
        panic!("not_finite expected, got {refused:?}");
    };
    assert_eq!((role.as_str(), name.as_str()), ("button", "Pin"));
    assert!(bounds.contains("width inf"), "{bounds}");
}

#[test]
fn finite_bounds_pass() {
    // Given
    let bounds = BoxRect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };

    // Then
    assert_eq!(finite(bounds, "button", "Pin"), Ok(()));
}
