//! An image's longest side is checked against the device's
//! `maxTextureDimension2D` once the document knows it, and against
//! WebGPU's guaranteed 8192 until then.

mod support;

use support::document;
use vapors_scene::{RgbaImage, Size};

fn row(width: u32) -> RgbaImage {
    RgbaImage {
        size: Size { width, height: 1 },
        rgba: [0, 0, 0, 255].repeat(width as usize),
    }
}

#[test]
fn before_the_device_limit_is_known_a_side_over_8192_is_refused_naming_why() {
    // Given
    let mut doc = document();

    // When
    let error = doc
        .register_image("wide.png", row(8193))
        .expect_err("refused");

    // Then
    assert!(
        error.contains("8192") && error.contains("set_max_image_side"),
        "{error}"
    );
}

#[test]
fn a_side_the_device_allows_is_registered_once_its_limit_is_known() {
    // Given
    let mut doc = document();
    doc.set_max_image_side(16384);

    // When
    let registered = doc.register_image("wide.png", row(10_000));

    // Then
    assert_eq!(registered, Ok(()));
}

#[test]
fn a_side_over_the_device_limit_is_refused_naming_the_limit() {
    // Given
    let mut doc = document();
    doc.set_max_image_side(16384);

    // When
    let error = doc
        .register_image("wider.png", row(16385))
        .expect_err("refused");

    // Then
    assert!(
        error.contains("wider.png") && error.contains("16384") && error.contains("device"),
        "{error}"
    );
}
