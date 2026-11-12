//! GPU tier: the image app draws its label strip across the top of the
//! frame with text on it, and leaves the pixels below the strip alone.

use vapors_canvas::{ImageApp, STRIP_COLOR, strip_height};
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

/// One frame of an empty app at `scale_factor`, read back.
fn frame_pixels(scale_factor: f64) -> Vec<u8> {
    let gpu = gpu();
    let mut app = ImageApp::new(BACKGROUND);
    let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor,
        now: vapors_platform::Instant::now(),
    });
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

/// Pixels in rows `0..height` from x 12 on that are not the strip colour.
fn inked_in_strip(pixels: &[u8], height: u32) -> usize {
    let strip = STRIP_COLOR.to_rgba8();
    (0..height)
        .flat_map(|y| (12..WIDTH).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(pixels, x, y) != strip)
        .count()
}

#[test]
fn gpu_the_label_strip_is_drawn_with_ink_on_it_and_nothing_below() {
    // Given / When: an empty app, one frame at 1x
    let pixels = frame_pixels(1.0);
    let height = strip_height(1.0);

    // Then
    let strip = STRIP_COLOR.to_rgba8();
    assert_eq!(pixel(&pixels, 0, 0), strip);
    assert_eq!(pixel(&pixels, WIDTH - 1, height - 1), strip);
    assert_eq!(
        pixel(&pixels, 0, height),
        BACKGROUND.to_rgba8(),
        "below the strip"
    );
    assert_eq!(pixel(&pixels, WIDTH / 2, HEIGHT / 2), BACKGROUND.to_rgba8());
    let inked = inked_in_strip(&pixels, height);
    assert!(inked > 50, "only {inked} inked pixels in the strip");
}

#[test]
fn gpu_at_2x_the_strip_is_twice_as_tall_and_the_text_has_more_ink() {
    // Given / When
    let at_1x = frame_pixels(1.0);
    let at_2x = frame_pixels(2.0);
    let height = strip_height(2.0);

    // Then
    let strip = STRIP_COLOR.to_rgba8();
    assert_eq!(height, 88);
    assert_eq!(pixel(&at_2x, WIDTH - 1, height - 1), strip);
    assert_eq!(
        pixel(&at_2x, 0, height),
        BACKGROUND.to_rgba8(),
        "below the strip"
    );
    let (ink_1x, ink_2x) = (inked_in_strip(&at_1x, 44), inked_in_strip(&at_2x, 88));
    assert!(ink_2x > ink_1x * 2, "1x {ink_1x} inked pixels, 2x {ink_2x}");
}
