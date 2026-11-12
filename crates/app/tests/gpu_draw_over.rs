//! GPU tier: `ScenePainter::draw_over` paints a document over what the
//! target already holds. Where no element paints (a transparent body)
//! the pixels are left as they were; a translucent background and an
//! antialiased edge blend over them; a background on `html` or `body`
//! is refused. `draw` still clears to the canvas colour first, and a
//! target format the shaders do not write correctly is refused.

use vapors_app::{DrawOverError, NewPainterError, ScenePainter, UnsupportedFormat};
use vapors_dom::Dom;
use vapors_elements::{CanvasFrom, Document, Viewport};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;

const SIZE: u32 = 64;
const RED: [u8; 4] = [255, 0, 0, 255];

/// Two 20x20 cards on a transparent body: an opaque blue one at (10, 10)
/// with rounded corners, and a half-transparent blue one at (34, 34).
const CARDS: &str = "html, body { margin: 0; background-color: transparent; } \
    div { position: absolute; width: 20px; height: 20px; } \
    .solid { left: 10px; top: 10px; background-color: #0000ff; border-radius: 6px; } \
    .glass { left: 34px; top: 34px; background-color: rgba(0, 0, 255, 0.5); }";

/// `css`'s cards painted on a red target, by `draw_over` or `draw`.
fn painted(css: &str, over: bool) -> Result<Vec<u8>, DrawOverError> {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("cards.css", css).expect("parses");
    let body = doc.body();
    for class in ["solid", "glass"] {
        let card = doc.create_element("div");
        doc.set_attribute(card, "class", class);
        doc.append(body, card);
    }
    #[allow(clippy::cast_precision_loss)] // a small size
    let viewport = Viewport {
        width: SIZE as f32,
        height: SIZE as f32,
        scale: 1.0,
    };
    let update = doc.update(viewport).expect("updates");
    let gpu = gpu();
    let texture = clear(
        &gpu.device,
        &gpu.queue,
        SIZE,
        SIZE,
        Color::from_rgba8(255, 0, 0, 255),
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: Size {
            width: SIZE,
            height: SIZE,
        },
        scissor: None,
    };
    let mut painter =
        ScenePainter::new(&gpu.device, texture.format()).expect("Rgba8Unorm is drawn");
    if over {
        painter.draw_over(&target, update)?;
    } else {
        painter.draw(&target, update).expect("paints");
    }
    encoder.submit(&gpu.queue).expect("no draw refused");
    Ok(read_rgba8(&gpu.device, &gpu.queue, &texture))
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

fn on_a_card(x: u32, y: u32) -> bool {
    let within = |from: u32| (from..from + 20).contains(&x) && (from..from + 20).contains(&y);
    within(10) || within(34)
}

#[test]
fn gpu_drawing_over_leaves_every_pixel_off_the_cards_as_it_was() {
    // When
    let rgba = painted(CARDS, true).expect("draws");

    // Then
    let changed: Vec<(u32, u32)> = (0..SIZE)
        .flat_map(|y| (0..SIZE).map(move |x| (x, y)))
        .filter(|&(x, y)| !on_a_card(x, y) && pixel(&rgba, x, y) != RED)
        .collect();
    assert!(
        changed.is_empty(),
        "{} pixels off the cards changed, first {:?}",
        changed.len(),
        changed.first()
    );
    assert_eq!(pixel(&rgba, 20, 20), [0, 0, 255, 255]);
}

#[test]
fn gpu_a_half_transparent_background_blends_over_and_keeps_the_alpha() {
    // When
    let rgba = painted(CARDS, true).expect("draws");

    // Then: half blue over red, opaque
    let [r, g, b, a] = pixel(&rgba, 44, 44);
    assert!(
        r.abs_diff(128) <= 1 && g == 0 && b.abs_diff(127) <= 1 && a == 255,
        "{:?}",
        [r, g, b, a]
    );
}

#[test]
fn gpu_an_antialiased_corner_mixes_the_card_and_what_was_under_it() {
    // When
    let rgba = painted(CARDS, true).expect("draws");

    // Then: some pixel of the rounded corner is strictly between red and blue
    let mixed = (10..16)
        .flat_map(|y| (10..16).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let [r, _, b, a] = pixel(&rgba, x, y);
            (1..255).contains(&r) && (1..255).contains(&b) && a == 255
        });
    assert!(mixed.is_some(), "no blended pixel in the corner");
}

#[test]
fn gpu_draw_still_clears_to_the_white_canvas_first() {
    // When
    let rgba = painted(CARDS, false).expect("draws");

    // Then
    assert_eq!(pixel(&rgba, 5, 5), [255, 255, 255, 255]);
    assert_eq!(pixel(&rgba, 20, 20), [0, 0, 255, 255]);
}

#[test]
fn gpu_drawing_over_a_document_whose_body_sets_a_background_is_refused_naming_body() {
    // Given
    let css = format!("{CARDS} body {{ background-color: #1c1c1c; }}");

    // When
    let refused = painted(&css, true).expect_err("a canvas colour");

    // Then
    assert!(
        matches!(refused, DrawOverError::CanvasBackground(CanvasFrom::Body)),
        "{refused:?}"
    );
    assert!(
        refused
            .to_string()
            .starts_with("the document's body sets a background"),
        "{refused}"
    );
}

#[test]
fn gpu_a_target_format_the_shaders_do_not_write_correctly_is_refused_naming_it() {
    // Given
    let gpu = gpu();

    // Then
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba16Float,
    ] {
        let refused = ScenePainter::new(&gpu.device, format).err();
        assert_eq!(
            refused,
            Some(NewPainterError::Format(UnsupportedFormat(format)))
        );
    }
    assert!(ScenePainter::new(&gpu.device, wgpu::TextureFormat::Bgra8Unorm).is_ok());
}
