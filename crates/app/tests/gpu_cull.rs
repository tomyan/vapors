//! GPU tier: culling keeps an item whose box lies outside the target
//! but whose paint reaches into it: a shadow's blur, a text's
//! descenders. Each is drawn on a white 64 x 64 target and the edge row
//! it reaches is read back.

use vapors_app::{FrameCounts, ScenePainter};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

const SIZE: u32 = 64;

/// A 20 px box 2 px below the target, its 12 px blur reaching 18 px up.
const SHADOW_BELOW: &str = "html, body { margin: 0; background-color: #fff; } \
    div { position: absolute; left: 20px; top: 66px; width: 20px; height: 20px; \
    background-color: #00f; box-shadow: 0 0 12px 0 #000; }";

/// A 10 px line box ending 2 px above the target, its 32 px glyphs'
/// descenders reaching below it into the target.
const DESCENDERS_ABOVE: &str = "html, body { margin: 0; background-color: #fff; } \
    div { position: absolute; left: 2px; top: -12px; font-size: 32px; \
    line-height: 10px; color: #000; }";

/// `css` over one div holding `text`, drawn: the pixels and the counts.
fn drawn(css: &str, text: &str) -> (Vec<u8>, FrameCounts) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("cull.css", css).expect("parses");
    let (body, div) = (doc.body(), doc.create_element("div"));
    doc.append(body, div);
    let node = doc.create_text(text);
    doc.append(div, node);
    #[allow(clippy::cast_precision_loss)] // a small size
    let side = SIZE as f32;
    let viewport = Viewport {
        width: side,
        height: side,
        scale: 1.0,
    };
    let update = doc.update(viewport).expect("updates");
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let size = Size {
        width: SIZE,
        height: SIZE,
    };
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: size,
        scissor: None,
    };
    let mut painter =
        ScenePainter::new(&gpu.device, texture.format()).expect("Rgba8Unorm is drawn");
    painter.draw(&target, update).expect("paints");
    encoder.submit(&gpu.queue).expect("no draw refused");
    let counts = painter.frame_counts();
    (read_rgba8(&gpu.device, &gpu.queue, &texture), counts)
}

/// The darkest red channel in row `y`.
fn darkest_in_row(rgba: &[u8], y: u32) -> u8 {
    let row = (y * SIZE * 4) as usize..((y + 1) * SIZE * 4) as usize;
    rgba[row].chunks(4).map(|p| p[0]).min().unwrap_or(255)
}

#[test]
fn gpu_a_shadow_cast_from_below_the_target_darkens_its_bottom_row() {
    // When
    let (rgba, counts) = drawn(SHADOW_BELOW, "");

    // Then: the box is culled, its shadow drawn
    let darkest = darkest_in_row(&rgba, SIZE - 1);
    assert!(
        darkest < 200,
        "bottom row's darkest red {darkest}; {counts:?}"
    );
    assert_eq!(counts.shadows.draw_calls, 1, "{counts:?}");
    assert_eq!(counts.culled, 1, "{counts:?}");
}

#[test]
fn gpu_descenders_of_a_line_above_the_target_ink_its_top_row() {
    // When
    let (rgba, counts) = drawn(DESCENDERS_ABOVE, "gjpqy");

    // Then
    let darkest = darkest_in_row(&rgba, 0);
    assert!(darkest < 128, "top row's darkest red {darkest}; {counts:?}");
    assert_eq!(counts.texts.draw_calls, 1, "{counts:?}");
}
