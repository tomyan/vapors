//! GPU tier: the quad pipeline fills each quad's pixels, in order, in
//! one draw.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{QuadPipeline, Target, clear};
use vapors_scene::{Color, Quad, Rect, Size};

const BACKGROUND: [u8; 4] = [40, 40, 40, 255];
const RED: [u8; 4] = [200, 30, 20, 255];
const BLUE: [u8; 4] = [20, 30, 200, 255];

fn pixel(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let start = (y * width + x) * 4;
    pixels[start..start + 4].try_into().expect("four bytes")
}

fn quad(rect: [f32; 4], rgba: [u8; 4]) -> Quad {
    Quad::fill(
        Rect::new(rect[0], rect[1], rect[2], rect[3]),
        Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]),
    )
}

fn draw(quads: &[Quad], width: u32, height: u32) -> Vec<u8> {
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        width,
        height,
        Color::from_rgba8(40, 40, 40, 255),
    );
    let mut pipeline = QuadPipeline::new(&gpu.device, target.format());
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let viewport = Size { width, height };
    pipeline.draw(
        &Target {
            device: &gpu.device,
            queue: &gpu.queue,
            view: &view,
            viewport,
        },
        quads,
    );
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

#[test]
fn gpu_each_quad_fills_its_pixels_and_a_later_quad_draws_over_an_earlier_one() {
    // Given: red over columns 1..5, blue over columns 3..7, both rows 1..3
    let quads = [
        quad([1.0, 1.0, 5.0, 3.0], RED),
        quad([3.0, 1.0, 7.0, 3.0], BLUE),
    ];

    // When
    let pixels = draw(&quads, 8, 4);

    // Then
    assert_eq!(pixel(&pixels, 8, 1, 1), RED);
    assert_eq!(pixel(&pixels, 8, 2, 2), RED);
    assert_eq!(pixel(&pixels, 8, 3, 1), BLUE, "the later quad is on top");
    assert_eq!(pixel(&pixels, 8, 6, 2), BLUE);
    assert_eq!(pixel(&pixels, 8, 0, 1), BACKGROUND, "left of both");
    assert_eq!(pixel(&pixels, 8, 7, 1), BACKGROUND, "right of both");
    assert_eq!(pixel(&pixels, 8, 3, 0), BACKGROUND, "above");
    assert_eq!(pixel(&pixels, 8, 3, 3), BACKGROUND, "below");
}

#[test]
fn gpu_more_quads_than_the_first_buffer_holds_are_all_drawn() {
    // Given: 100 one-pixel quads along a 100x1 row, more than the 64 the
    // first instance buffer holds
    let quads: Vec<Quad> = (0..100u8)
        .map(|x| quad([f32::from(x), 0.0, f32::from(x) + 1.0, 1.0], RED))
        .collect();

    // When
    let pixels = draw(&quads, 100, 1);

    // Then
    for x in [0, 63, 64, 99] {
        assert_eq!(pixel(&pixels, 100, x, 0), RED, "x = {x}");
    }
}
