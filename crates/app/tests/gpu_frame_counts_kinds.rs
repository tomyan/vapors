//! GPU tier: `frame_counts` for the counter demo's idle frame, and an
//! icon's attribution to `icons` rather than `texts` (IU1 PR 3).

#[path = "support/surface.rs"]
mod support;

use support::{Rig, update};
use vapors_app::ScenePainter;
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_gpu_harness::gpu;
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

/// The atlas page, one coverage byte per pixel, uploaded whole.
const PAGE_BYTES: u64 = 1024 * 1024;

#[test]
fn gpu_the_counter_demos_idle_frame_matches_the_plan_row() {
    // Given: docs/plans/iu1.md, `counter` at 640x400 and 1x
    let demo = vapors_demos::find("counter").expect("the counter demo");
    let shown = vapors_demos::document(&demo);
    let mut document = shown.document.borrow_mut();
    document
        .update(vapors_elements::Viewport {
            width: 640.0,
            height: 400.0,
            scale: 1.0,
        })
        .expect("updates");
    let scene = document.painted().expect("updated");
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, 640, 400, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let size = Size {
        width: 640,
        height: 400,
    };
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: size,
        scissor: None,
    };
    let mut painter = ScenePainter::new(&gpu.device, texture.format()).expect("Rgba8Unorm");
    painter.draw(&target, scene).expect("paints");

    // When
    painter.draw(&target, scene).expect("paints");

    // Then
    let idle = painter.frame_counts().total();
    assert_eq!(idle.draw_calls, 3);
    assert_eq!(idle.buffer_bytes, 2800);
    assert_eq!(idle.texture_bytes, 0);
}

#[test]
fn gpu_an_icon_is_one_draw_call_and_an_atlas_page_filed_under_icons() {
    // Given
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let icon = doc.create_element("svg");
    for (name, value) in [
        ("viewBox", "0 0 16 16"),
        ("width", "16"),
        ("height", "16"),
        ("aria-hidden", "true"),
    ] {
        doc.set_attribute(icon, name, value);
    }
    doc.append(body, icon);
    let path = doc.create_element("path");
    for (name, value) in [
        ("fill", "none"),
        ("stroke", "currentColor"),
        ("d", "M0 8H16"),
    ] {
        doc.set_attribute(path, name, value);
    }
    doc.append(icon, path);
    doc.add_stylesheet("app.css", "svg { display: block; }")
        .expect("parses");
    update(&mut doc, 1.0);
    let mut rig = Rig::new();

    // When
    rig.draw(&doc).expect("draws");

    // Then
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.icons.draw_calls, 1);
    assert_eq!(counts.atlas_bytes(), PAGE_BYTES);
    assert_eq!(counts.icons.texture_bytes, PAGE_BYTES);
    assert_eq!(counts.texts.draw_calls, 0);
    assert_eq!(counts.total().draw_calls, 1);
}
