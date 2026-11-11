//! The page a device gets by default, and the pages it refuses.

use super::*;

#[test]
fn the_default_page_is_the_device_side_up_to_4096() {
    // Given
    let cases = [(16384, 4096), (8192, 4096), (4096, 4096), (2048, 2048)];

    for (max_texture_side, expected) in cases {
        // When
        let config = AtlasConfig::for_max_texture_side(max_texture_side);

        // Then
        assert_eq!(config.page_size, expected, "device side {max_texture_side}");
    }
}

#[test]
fn a_page_over_the_device_side_is_refused_naming_both() {
    // When
    let error = AtlasConfig { page_size: 32768 }
        .checked(16384)
        .expect_err("refused");

    // Then
    let message = error.to_string();
    assert!(
        message.contains("32768 px") && message.contains("16384 px"),
        "{message}"
    );
    assert!(message.contains("Fix:"), "{message}");
}

#[test]
fn a_page_whose_bytes_overflow_a_u32_is_refused_at_the_largest_page_plus_one() {
    // When
    let largest = AtlasConfig {
        page_size: LARGEST_PAGE,
    }
    .checked(u32::MAX);
    let over = AtlasConfig {
        page_size: LARGEST_PAGE + 1,
    }
    .checked(u32::MAX);

    // Then
    assert!(largest.is_ok(), "{largest:?}");
    let message = over.expect_err("overflows").to_string();
    assert!(
        message.contains("32768 px") && message.contains("32767 px"),
        "{message}"
    );
}

#[test]
fn a_page_of_zero_is_refused() {
    assert!(AtlasConfig { page_size: 0 }.checked(16384).is_err());
}

#[test]
fn a_page_the_device_holds_is_kept() {
    // Given
    let config = AtlasConfig::for_max_texture_side(16384);

    // When / Then
    assert_eq!(config.checked(16384), Ok(config));
}
