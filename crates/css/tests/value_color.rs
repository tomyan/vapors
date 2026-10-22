//! Colour conversions (`src/value/color.rs`): Oklab round trips.

mod support;

use vapors_css::value::Color;
use vapors_scene::Color as Rgba;

#[test]
fn oklab_round_trips_srgb() {
    // Given
    let c = Rgba::from_rgba8(200, 30, 90, 128);

    // When
    let back = Color::from_oklab(Color::to_oklab(c));

    // Then
    assert_eq!(back.to_rgba8(), c.to_rgba8());
}

#[test]
fn oklab_lightness_of_white_is_one() {
    let [l, a, b, _] = Color::to_oklab(Rgba::from_rgba8(255, 255, 255, 255));
    assert!(
        (l - 1.0).abs() < 0.001 && a.abs() < 0.001 && b.abs() < 0.001,
        "{l} {a} {b}"
    );
}
