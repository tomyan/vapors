//! The uncaptured-error state, what is said about it, and wgpu's handler
//! reaching it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

fn validation() -> Uncaptured {
    Uncaptured {
        kind: "Validation",
        message: "Dimension X value 16385 exceeds the limit of 16384".to_owned(),
    }
}

#[test]
fn on_the_web_the_sentence_names_the_error_and_says_reload() {
    // When
    let said = sentence(&validation(), Target::Web, "touche");

    // Then
    assert_eq!(
        said,
        "a GPU call failed with no error scope to catch it (Validation: Dimension X value 16385 \
         exceeds the limit of 16384); the frame it belonged to was not drawn. This is a bug in \
         touche or vapors, so report this message; fix: reload the page"
    );
}

#[test]
fn natively_the_fix_is_to_restart_the_app_by_name() {
    // When
    let said = sentence(&validation(), Target::Native, "touche");

    // Then
    assert!(said.ends_with("fix: restart touche"), "{said}");
}

#[test]
fn the_state_keeps_the_first_error_and_every_clone_sees_it() {
    // Given
    let state = UncapturedErrors::default();
    let seen_by_the_loop = state.clone();
    assert_eq!(seen_by_the_loop.get(), None);

    // When
    state.record(validation());
    state.record(Uncaptured {
        kind: "Internal",
        message: "later".to_owned(),
    });

    // Then
    assert_eq!(seen_by_the_loop.get(), Some(validation()));
}

#[test]
fn no_error_no_stop() {
    assert!(stop_for(None, Target::Native, "touche").is_none());
}

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
fn gpu_an_uncaptured_validation_error_is_recorded_wakes_the_loop_and_stops_it_by_name() {
    // Given: a device watched as the window's is
    let gpu = vapors_gpu_harness::gpu();
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
    let stop = stop_for(state.get(), Target::Native, "touche").expect("stops");
    assert!(
        matches!(&stop, PlatformError::UncapturedGpuError(said) if said.contains(&error.message)),
        "{stop}"
    );
}
