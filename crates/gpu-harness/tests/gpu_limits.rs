//! GPU tier: the harness device has the adapter's own ceilings for the
//! three limits that decide how large an image can be, as the window
//! device does, so what passes here runs in a window.

use vapors_gpu_harness::gpu;

#[test]
fn gpu_device_limits_match_the_adapter_for_the_raised_fields() {
    // Given
    let gpu = gpu();
    let adapter = gpu.adapter.limits();

    // When
    let device = gpu.device.limits();

    // Then
    let name = gpu.adapter.get_info().name;
    assert_eq!(
        device.max_buffer_size, adapter.max_buffer_size,
        "max_buffer_size on {name}"
    );
    assert_eq!(
        device.max_texture_dimension_2d, adapter.max_texture_dimension_2d,
        "max_texture_dimension_2d on {name}"
    );
    assert_eq!(
        device.max_storage_buffer_binding_size, adapter.max_storage_buffer_binding_size,
        "max_storage_buffer_binding_size on {name}"
    );
}
