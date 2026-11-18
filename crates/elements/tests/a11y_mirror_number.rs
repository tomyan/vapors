//! A number field's record carries its range from `min` and `max`, as
//! Chromium maps them to `aria-valuemin` and `aria-valuemax`; a field
//! without both carries none, and the mirror leaves the bounds out
//! (ARIA gives a spinbutton no default range).

use crate::support;

use support::mirror::{node_of, step, taken_model};
use vapors_dom::Dom;
use vapors_elements::AxRange;

/// The range in the record of the number field Radius after `bounds`
/// are set on it (and a new name).
fn radius_range(bounds: &[(&str, &str)]) -> Option<AxRange> {
    let (doc, mut model) = taken_model();
    let radius = node_of(&doc, "spinbutton", "Radius");
    for (name, value) in bounds {
        doc.borrow_mut().set_attribute(radius, name, value);
    }
    // a new name, so the field's record is in the take whatever else
    doc.borrow_mut().set_attribute(radius, "aria-label", "Size");
    let update = step(&doc, &mut model).expect("a change");
    let record = update.nodes.iter().find(|node| node.node == radius);
    record.expect("Radius changed").range
}

#[test]
fn a_number_field_with_min_and_max_carries_them_as_its_range() {
    // When
    let range = radius_range(&[("min", "-5"), ("max", "5")]);

    // Then
    assert_eq!(
        range,
        Some(AxRange {
            min: -5.0,
            max: 5.0
        })
    );
}

#[test]
fn a_number_field_without_both_bounds_carries_no_range() {
    // Then
    assert_eq!(radius_range(&[]), None, "neither");
    assert_eq!(radius_range(&[("min", "0")]), None, "min alone");
}
