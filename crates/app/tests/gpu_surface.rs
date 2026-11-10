//! GPU tier: what `ScenePainter` does with a `gpu-surface`
//! (docs/gpu-surface.md). Until the surface host (slice 1b) it refuses
//! the frame, naming the surface, rather than drawing it blank.

use vapors_app::{PaintError, ScenePainter};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::gpu;
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

const SIZE: u32 = 32;

/// A document with one 16 x 16 `gpu-surface` named "track", painted.
fn surface_document() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "gpu-surface { width: 16px; height: 16px; }")
        .expect("parses");
    let body = doc.body();
    let surface = doc.create_element("gpu-surface");
    doc.set_attribute(surface, "name", "track");
    doc.append(body, surface);
    #[allow(clippy::cast_precision_loss)] // a small size
    let viewport = Viewport {
        width: SIZE as f32,
        height: SIZE as f32,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    doc
}

#[test]
fn gpu_a_surface_the_painter_cannot_draw_refuses_the_frame_naming_it() {
    // Given
    let doc = surface_document();
    let gpu = gpu();
    let texture = clear(
        &gpu.device,
        &gpu.queue,
        SIZE,
        SIZE,
        Color::from_rgba8(0, 0, 0, 255),
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: Size {
            width: SIZE,
            height: SIZE,
        },
        scissor: None,
    };
    let mut painter = ScenePainter::new(&gpu.device, texture.format()).expect("Rgba8Unorm");

    // When
    let refused = painter
        .draw(&target, doc.painted().expect("updated"))
        .expect_err("refused");

    // Then
    assert!(matches!(refused, PaintError::Surface(ref name) if name == "track"));
    assert!(refused.to_string().contains("\"track\""), "{refused}");
}
