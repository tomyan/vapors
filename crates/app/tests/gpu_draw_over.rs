//! GPU tier: `ScenePainter::draw_over` paints a document over what the
//! target already holds. Where no element paints (a transparent body),
//! the target's pixels are left as they were; where an element paints,
//! it shows. `draw` still clears to the canvas colour first.

use vapors_app::ScenePainter;
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

const SIZE: u32 = 64;

/// A 20x20 blue card at (10, 10) on a transparent body.
fn card() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "card.css",
        "html, body { margin: 0; background-color: transparent; } \
         div { position: absolute; left: 10px; top: 10px; width: 20px; height: 20px; background-color: #0000ff; }",
    )
    .expect("parses");
    let body = doc.body();
    let card = doc.create_element("div");
    doc.append(body, card);
    doc
}

/// A red target with the card painted on it, by `draw_over` or `draw`.
fn painted(over: bool) -> Vec<u8> {
    let mut doc = card();
    #[allow(clippy::cast_precision_loss)] // a small size
    let viewport = Viewport {
        width: SIZE as f32,
        height: SIZE as f32,
        scale: 1.0,
    };
    let update = doc.update(viewport).expect("updates");
    let gpu = gpu();
    let red = Color::from_rgba8(255, 0, 0, 255);
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, red);
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
    let mut painter = ScenePainter::new(&gpu.device, texture.format());
    if over {
        painter.draw_over(&target, update).expect("paints");
    } else {
        painter.draw(&target, update).expect("paints");
    }
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * SIZE + x) * 4) as usize;
    [
        rgba[offset],
        rgba[offset + 1],
        rgba[offset + 2],
        rgba[offset + 3],
    ]
}

#[test]
fn gpu_drawing_over_leaves_every_pixel_outside_the_elements_as_it_was() {
    // When
    let rgba = painted(true);

    // Then: the card is blue; every pixel off it is still the red
    assert_eq!(pixel(&rgba, 20, 20), [0, 0, 255, 255]);
    let mut changed = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let on_card = (10..30).contains(&x) && (10..30).contains(&y);
            if !on_card && pixel(&rgba, x, y) != [255, 0, 0, 255] {
                changed.push((x, y, pixel(&rgba, x, y)));
            }
        }
    }
    assert!(
        changed.is_empty(),
        "{} pixels off the card changed, first {:?}",
        changed.len(),
        changed.first()
    );
}

#[test]
fn gpu_draw_still_clears_to_the_canvas_colour_first() {
    // When
    let rgba = painted(false);

    // Then: off the card is no longer the red
    assert_ne!(pixel(&rgba, 50, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(&rgba, 20, 20), [0, 0, 255, 255]);
}
