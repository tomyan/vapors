//! GPU tier: `App::start` on an `ElementsApp` does, before the first
//! frame, the work that frame would otherwise do once (one-scene F0):
//! it builds every pipeline, mounts the surface renderers and uploads
//! the glyph atlas's new page (sizes: `gpu_app_start_sizes.rs`).

#[path = "support/surface.rs"]
mod support;

use support::start_rig::{rig, start, start_for};
use support::{RED, app_frame, pixel};
use vapors_app::StartState;
use vapors_renderer::pipelines::built;

#[test]
fn gpu_a_started_app_builds_no_pipeline_in_its_first_frame() {
    // Given
    let (mut app, _, gpu) = rig();
    start(&mut app, &gpu).expect("starts");
    let before = built();

    // When
    let (_, rgba) = app_frame(&mut app, &gpu);

    // Then
    assert!(
        built().since(before).is_none(),
        "{:?}",
        built().since(before)
    );
    assert_eq!(pixel(&rgba, 4, 4), RED);
    assert!(app.failure().get().is_none());
}

#[test]
fn gpu_start_mounts_each_surface_renderer_before_the_first_frame() {
    // Given
    let (mut app, log, gpu) = rig();

    // When
    start(&mut app, &gpu).expect("starts");

    // Then
    assert_eq!(log.borrow().mounts, 1);
    assert!(log.borrow().frames.is_empty(), "start rendered a frame");
}

#[test]
fn gpu_a_started_app_uploads_only_its_glyphs_in_its_first_frame() {
    // Given: a new 1024 px page is 1 MiB of coverage bytes
    let (mut app, _, gpu) = rig();
    start(&mut app, &gpu).expect("starts");

    // When
    app_frame(&mut app, &gpu);

    // Then
    let texture_bytes = app.frame_counts().texts.texture_bytes;
    assert!(
        (1..64 * 1024).contains(&texture_bytes),
        "the first frame wrote {texture_bytes} atlas bytes"
    );
}

#[test]
fn gpu_a_frame_in_another_format_than_the_start_is_refused_naming_both() {
    // Given
    let (mut app, _, gpu) = rig();
    start_for(&mut app, &gpu, wgpu::TextureFormat::Bgra8Unorm).expect("starts");

    // When
    app_frame(&mut app, &gpu);

    // Then
    let failure = app.failure().get().unwrap_or_default();
    assert!(
        failure.contains("Rgba8Unorm")
            && failure.contains("Bgra8Unorm")
            && failure.contains("fix: give App::start the format"),
        "{failure}"
    );
}

#[test]
fn gpu_a_second_start_is_refused_naming_the_fix() {
    // Given
    let (mut app, _, gpu) = rig();
    start(&mut app, &gpu).expect("starts");

    // When
    let again = start(&mut app, &gpu);

    // Then
    let error = again.expect_err("started twice");
    assert!(
        error.contains("after the app's painter was made") && error.contains("fix:"),
        "{error}"
    );
}

#[test]
fn gpu_an_app_framed_without_a_start_says_its_painter_was_made_in_the_frame() {
    // Given
    let (mut app, _, gpu) = rig();

    // When
    app_frame(&mut app, &gpu);

    // Then
    assert_eq!(app.start_state(), StartState::MadeInFirstFrame);
}

#[test]
fn gpu_a_started_app_says_it_was_started() {
    // Given
    let (mut app, _, gpu) = rig();

    // When
    start(&mut app, &gpu).expect("starts");
    app_frame(&mut app, &gpu);

    // Then
    assert_eq!(app.start_state(), StartState::Started);
}
