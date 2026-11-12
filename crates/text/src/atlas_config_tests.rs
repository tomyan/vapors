//! The pages a device gets by default, and the pages it refuses.

use super::*;

#[test]
fn the_default_page_starts_at_1024_and_grows_to_the_device_side_up_to_4096() {
    // Given
    let cases = [
        (16384, 1024, 4096),
        (8192, 1024, 4096),
        (4096, 1024, 4096),
        (2048, 1024, 2048),
        (512, 512, 512),
    ];

    for (max_texture_side, first, largest) in cases {
        // When
        let config = AtlasConfig::for_max_texture_side(max_texture_side);

        // Then
        let expected = AtlasConfig {
            page_size: first,
            max_page_size: largest,
        };
        assert_eq!(config, expected, "device side {max_texture_side}");
    }
}

#[test]
fn a_largest_page_over_the_device_side_is_refused_naming_both() {
    // When
    let error = AtlasConfig::fixed(32768)
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
fn a_largest_page_whose_bytes_overflow_a_u32_is_refused_at_the_largest_page_plus_one() {
    // When
    let largest = AtlasConfig::fixed(LARGEST_PAGE).checked(u32::MAX);
    let over = AtlasConfig {
        page_size: 1024,
        max_page_size: LARGEST_PAGE + 1,
    }
    .checked(u32::MAX);

    // Then
    assert!(largest.is_ok(), "{largest:?}");
    let message = over.expect_err("overflows").to_string();
    assert!(
        message.contains("65536 px") && message.contains("65535 px"),
        "{message}"
    );
}

#[test]
fn a_page_of_zero_is_refused() {
    assert!(AtlasConfig::fixed(0).checked(16384).is_err());
}

#[test]
fn a_first_page_over_the_largest_is_refused() {
    // Given
    let config = AtlasConfig {
        page_size: 2048,
        max_page_size: 1024,
    };

    // When / Then
    assert!(config.checked(16384).is_err());
}

#[test]
fn the_default_for_a_device_is_kept() {
    for side in [1, 2048, 16384, 32768, 65536, u32::MAX] {
        // Given
        let config = AtlasConfig::for_max_texture_side(side);

        // When / Then
        assert_eq!(config.checked(side), Ok(config), "device side {side}");
    }
}
