use super::applied_limit;
use crate::{ArenaFull, Holding};

#[test]
fn the_lower_of_the_two_buffer_limits_applies_and_is_named() {
    // Given
    let storage_lower = wgpu::Limits {
        max_buffer_size: 1 << 30,
        max_storage_buffer_binding_size: 1 << 27,
        ..wgpu::Limits::default()
    };
    let buffer_lower = wgpu::Limits {
        max_buffer_size: 1 << 26,
        max_storage_buffer_binding_size: 1 << 27,
        ..wgpu::Limits::default()
    };

    // When, then
    assert_eq!(
        applied_limit(&storage_lower),
        (1 << 27, "max_storage_buffer_binding_size")
    );
    assert_eq!(applied_limit(&buffer_lower), (1 << 26, "max_buffer_size"));
}

#[test]
fn a_full_table_names_the_limit_that_applied_and_a_fix_for_entries() {
    // Given
    let full = ArenaFull {
        pipeline: "painter transforms",
        size: 1 << 28,
        limit: 1 << 27,
        holding: Holding::TransformEntries {
            limit: "max_storage_buffer_binding_size",
        },
    };

    // When
    let message = full.to_string();

    // Then
    assert!(
        message.contains("over the device's max_storage_buffer_binding_size of 134217728 bytes"),
        "{message}"
    );
    assert!(
        message.contains("draw fewer scroll frames and lane batches"),
        "{message}"
    );
}
