//! GPU tier: `App::start` on an `ElementsApp` does, before the first
//! frame, the work that frame would otherwise do once (one-scene F0):
//! it builds every pipeline, mounts the surface renderers, uploads the
//! glyph atlas's new page, lays the document out at the surface's size
//! and sizes each surface's backing.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{RED, SIZE, Solid, app_frame, document, pixel};
use vapors_app::{ElementsApp, StartState, SurfaceSize};
use vapors_dom::Dom;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Start};
use vapors_renderer::pipelines::built;

const CSS: &str = ".track { width: 16px; height: 16px; } p { font-size: 8px; margin: 0; }";

/// An app showing a 16 x 16 surface named `track` over a line of text,
/// its renderer's log, and its GPU.
fn rig() -> (ElementsApp, Rc<RefCell<support::Log>>, Gpu) {
    let mut doc = document(CSS, &[(None, "gpu-surface", "track"), (None, "p", "")]);
    let line = doc.create_text("Lap 1");
    let paragraph = doc.children(doc.body())[1];
    doc.append(paragraph, line);
    let mut app = ElementsApp::new(Rc::new(RefCell::new(doc)));
    let (solid, log) = Solid::new(RED);
    app.mount("track", Box::new(solid)).expect("mounts");
    (app, log, gpu())
}

/// `app`'s start on `gpu`, at the test target's size and format.
fn start(app: &mut ElementsApp, gpu: &Gpu) -> Result<(), String> {
    start_for(app, gpu, wgpu::TextureFormat::Rgba8Unorm)
}

/// `app`'s start on `gpu` for textures of `format`.
fn start_for(app: &mut ElementsApp, gpu: &Gpu, format: wgpu::TextureFormat) -> Result<(), String> {
    app.start(&Start {
        format,
        ..start_at(gpu)
    })
}

/// A start on `gpu` for the test target: its size and format.
fn start_at(gpu: &Gpu) -> Start<'_> {
    Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width: SIZE,
        height: SIZE,
        scale_factor: 1.0,
    }
}

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

#[test]
fn gpu_start_lays_the_document_out_and_sizes_each_surface_before_the_first_frame() {
    // Given
    let (mut app, log, gpu) = rig();
    start(&mut app, &gpu).expect("starts");
    let at_start = log.borrow().resizes.clone();

    // When
    app_frame(&mut app, &gpu);

    // Then: the first frame reuses the backing sized at start
    let side = SurfaceSize {
        width: 16,
        height: 16,
        scale: 1.0,
    };
    assert_eq!(at_start, vec![side]);
    assert_eq!(log.borrow().resizes, vec![side]);
    assert_eq!(log.borrow().frames, vec![side]);
}

#[test]
fn gpu_a_start_with_no_surface_size_is_refused_naming_the_fix() {
    // Given
    let (mut app, _, gpu) = rig();

    // When
    let refused = app.start(&Start {
        width: 0,
        ..start_at(&gpu)
    });

    // Then
    let error = refused.expect_err("a 0-pixel surface has no layout");
    assert!(error.contains("0x32") && error.contains("fix:"), "{error}");
}
