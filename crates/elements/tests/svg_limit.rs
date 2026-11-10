//! The largest an svg icon is drawn (docs/icons.md, The subset): an
//! svg whose width or height times the viewport scale is over 256
//! device px is refused before layout, naming the element, both sizes
//! and the fix.

mod support;

use support::svg_page::{assert_refused, icon};
use support::{refusals_at, viewport};
use vapors_elements::Viewport;

fn scaled(scale: f32) -> Viewport {
    Viewport {
        scale,
        ..viewport(400.0, 300.0)
    }
}

#[test]
fn an_svg_over_256_device_px_is_refused_naming_both_sizes_and_the_fix() {
    // Given: 130 CSS px at 2x is 260 device px
    let (mut doc, _) = icon("", &[("width", "130"), ("height", "130")], &[]);

    // When
    let errors = refusals_at(&mut doc, scaled(2.0));

    // Then
    let says = [
        "<svg class=\"icon\">",
        "130 x 130 CSS px",
        "260 x 260 device px",
        "256 px",
        "draw it smaller, or as an image",
    ];
    assert!(
        errors.iter().any(|e| says.iter().all(|s| e.contains(s))),
        "{says:?} in {errors:#?}"
    );
}

#[test]
fn an_svg_of_exactly_256_device_px_is_accepted() {
    // Given
    let (mut doc, _) = icon("", &[("width", "128"), ("height", "128")], &[]);

    // When
    let updated = doc.update(scaled(2.0));

    // Then
    assert!(updated.is_ok(), "{:?}", updated.err());
}

#[test]
fn touches_icons_are_accepted() {
    for (size, scale) in [("16", 1.0), ("16", 3.0), ("48", 3.0)] {
        // Given
        let viewbox = format!("0 0 {size} {size}");
        let attributes = [("width", size), ("height", size), ("viewBox", &viewbox)];
        let (mut doc, _) = icon("", &attributes, &[]);

        // When
        let updated = doc.update(scaled(scale));

        // Then
        assert!(updated.is_ok(), "{size} at {scale}x: {:?}", updated.err());
    }
}

#[test]
fn a_scale_change_that_lifts_an_accepted_svg_over_the_limit_is_refused_on_the_next_update() {
    // Given: 100 CSS px is 100 device px at 1x, accepted
    let (mut doc, _) = icon(
        "",
        &[
            ("width", "100"),
            ("height", "100"),
            ("viewBox", "0 0 100 100"),
        ],
        &[],
    );
    doc.update(scaled(1.0)).expect("accepted at 1x");

    // When: the viewport moves to 3x, 300 device px
    let errors = refusals_at(&mut doc, scaled(3.0));

    // Then
    assert!(
        errors.iter().any(|e| e.contains("300 x 300 device px")),
        "{errors:#?}"
    );
}

#[test]
fn a_wide_svg_is_refused_on_its_longer_side() {
    // Given: 300 x 10 at 1x
    let (mut doc, _) = icon(
        "",
        &[("width", "300"), ("height", "10"), ("viewBox", "0 0 30 1")],
        &[],
    );

    // When / Then
    assert_refused(&mut doc, &["300 x 10 CSS px", "draw it smaller"]);
}
