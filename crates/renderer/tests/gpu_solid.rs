//! GPU tier: the solid pipeline fills exactly the requested rectangle.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{SolidPipeline, clear};
use vapors_scene::{Color, Size};

const BACKGROUND: [u8; 4] = [40, 40, 40, 255];
const STRIP: [u8; 4] = [23, 27, 31, 255];

fn pixel(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let start = (y * width + x) * 4;
    pixels[start..start + 4].try_into().expect("four bytes")
}

#[test]
fn gpu_a_rectangle_fills_its_pixels_and_no_others() {
    // Given: an 8x6 target and a strip across the top 2 rows, 1 px in from the left
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        8,
        6,
        Color::from_rgba8(40, 40, 40, 255),
    );
    let pipeline = SolidPipeline::new(&gpu.device, target.format());
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // When
    pipeline.fill(
        &gpu.device,
        &gpu.queue,
        &view,
        Size {
            width: 8,
            height: 6,
        },
        [1.0, 0.0, 7.0, 2.0],
        Color::from_rgba8(23, 27, 31, 255),
    );
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &target);

    // Then
    assert_eq!(pixel(&pixels, 8, 1, 0), STRIP);
    assert_eq!(pixel(&pixels, 8, 6, 1), STRIP);
    assert_eq!(pixel(&pixels, 8, 0, 0), BACKGROUND, "left of the strip");
    assert_eq!(pixel(&pixels, 8, 7, 1), BACKGROUND, "right of the strip");
    assert_eq!(pixel(&pixels, 8, 1, 2), BACKGROUND, "below the strip");
    assert_eq!(pixel(&pixels, 8, 4, 5), BACKGROUND);
}

#[test]
fn gpu_a_translucent_rectangle_blends_over_the_background() {
    // Given
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        2,
        2,
        Color::from_rgba8(0, 0, 0, 255),
    );
    let pipeline = SolidPipeline::new(&gpu.device, target.format());
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // When: half-opaque white over black
    pipeline.fill(
        &gpu.device,
        &gpu.queue,
        &view,
        Size {
            width: 2,
            height: 2,
        },
        [0.0, 0.0, 2.0, 2.0],
        Color::from_rgba8(255, 255, 255, 128),
    );
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &target);

    // Then: 128/255 of the way to white, within a rounding step
    let [r, g, b, _] = pixel(&pixels, 2, 1, 1);
    for channel in [r, g, b] {
        assert!((127..=129).contains(&channel), "{r} {g} {b}");
    }
}
