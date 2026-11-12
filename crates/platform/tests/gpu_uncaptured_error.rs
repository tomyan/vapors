//! GPU tier: wgpu's uncaptured-error handler reaches vapors' state and
//! wakes the loop, which then stops naming the error (`gpu_watch.rs`
//! unit tests). A texture over the device's side, made outside any
//! error scope, is the atlas-page-over-the-limit path of
//! review-fable-painter H2.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vapors_gpu_harness::gpu;
use vapors_platform::UncapturedErrors;

/// A texture one pixel wider than the device allows, made with no error
/// scope: the atlas-page-over-the-limit path the review names.
fn make_too_wide_texture(device: &wgpu::Device) {
    let width = device.limits().max_texture_dimension_2d + 1;
    drop(device.create_texture(&wgpu::TextureDescriptor {
        label: Some("too wide on purpose"),
        size: wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }));
}

#[test]
fn gpu_an_uncaptured_validation_error_is_recorded_and_wakes_the_loop() {
    // Given: a device watched as the window's is
    let gpu = gpu();
    let state = UncapturedErrors::default();
    let woken = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&woken);
    state.watch(&gpu.device, move || flag.store(true, Ordering::SeqCst));

    // When
    make_too_wide_texture(&gpu.device);

    // Then
    let error = state.get().expect("the handler recorded the error");
    assert_eq!(error.kind, "Validation", "{error:?}");
    assert!(woken.load(Ordering::SeqCst), "the waker was called");
}
