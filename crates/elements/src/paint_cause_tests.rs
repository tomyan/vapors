//! A box's own length past f32's range names the cause, by property;
//! an infinite max size limits nothing and is not one.

use taffy::{Dimension, LengthPercentageAuto, Size, Style};

use super::own_length;

#[test]
fn an_infinite_own_length_is_named_by_its_property() {
    // Given
    let mut style = Style::default();
    style.margin.bottom = LengthPercentageAuto::length(f32::INFINITY);

    // Then
    assert_eq!(own_length(&style), Some("margin-bottom"));
}

#[test]
fn an_infinite_max_size_or_finite_lengths_name_nothing() {
    // Given
    let style = Style {
        max_size: Size {
            width: LengthPercentageAuto::length(f32::INFINITY),
            height: LengthPercentageAuto::auto(),
        },
        size: Size {
            width: Dimension::length(10.0),
            height: Dimension::percent(0.5),
        },
        ..Style::default()
    };

    // Then
    assert_eq!(own_length(&style), None);
}
