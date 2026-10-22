//! GPU tier: the image app draws its label strip across the top of the
//! frame with text on it, and leaves the pixels below the strip alone.

use vapors_canvas::{ImageApp, STRIP_COLOR, STRIP_HEIGHT};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_platform::{App, Frame};
use vapors_renderer::clear;
use vapors_scene::Color;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 100;
const BACKGROUND: Color = Color {
    r: 40.0 / 255.0,
    g: 40.0 / 255.0,
    b: 40.0 / 255.0,
    a: 1.0,
};

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let start = ((y * WIDTH + x) * 4) as usize;
    pixels[start..start + 4].try_into().expect("four bytes")
}

#[test]
fn gpu_the_label_strip_is_drawn_with_ink_on_it_and_nothing_below() {
    // Given: an empty app and a frame to draw
    let gpu = gpu();
    let mut app = ImageApp::new(BACKGROUND);
    let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, Color::default());

    // When
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
    });
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);

    // Then
    let strip = STRIP_COLOR.to_rgba8();
    assert_eq!(pixel(&pixels, 0, 0), strip);
    assert_eq!(pixel(&pixels, WIDTH - 1, STRIP_HEIGHT - 1), strip);
    assert_eq!(
        pixel(&pixels, 0, STRIP_HEIGHT),
        BACKGROUND.to_rgba8(),
        "below the strip"
    );
    assert_eq!(pixel(&pixels, WIDTH / 2, HEIGHT / 2), BACKGROUND.to_rgba8());
    let inked = (0..STRIP_HEIGHT)
        .flat_map(|y| (12..WIDTH).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&pixels, x, y) != strip)
        .count();
    assert!(inked > 50, "only {inked} inked pixels in the strip");
}
