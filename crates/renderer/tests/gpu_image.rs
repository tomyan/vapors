//! GPU tier: the image pipeline places a texture by its transform with
//! nearest sampling, so every target pixel is exactly one source pixel.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{ImagePipeline, clear, upload_rgba8};
use vapors_scene::{Color, Size, Transform};

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const BACKGROUND: [u8; 4] = [40, 40, 40, 255];

#[test]
fn gpu_a_two_by_two_image_at_scale_two_fills_quadrants_exactly() {
    // Given: a 2x2 image (red, green / blue, white) and a 4x4 target
    let gpu = gpu();
    let source = [RED, GREEN, BLUE, WHITE].concat();
    let texture = upload_rgba8(&gpu.device, &gpu.queue, 2, 2, &source);
    let viewport = Size {
        width: 4,
        height: 4,
    };
    let target = clear(
        &gpu.device,
        &gpu.queue,
        4,
        4,
        Color::from_rgba8(40, 40, 40, 255),
    );
    let pipeline = ImagePipeline::new(&gpu.device, target.format());

    // When: drawn at scale 2 from the origin
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let transform = Transform {
        scale: 2.0,
        translate: [0.0, 0.0],
    };
    pipeline.draw(
        &gpu.device,
        &gpu.queue,
        &view,
        viewport,
        &texture,
        transform,
    );
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &target);

    // Then: each quadrant is one source pixel
    assert_eq!(pixel(&pixels, 4, 0, 0), RED);
    assert_eq!(pixel(&pixels, 4, 1, 1), RED);
    assert_eq!(pixel(&pixels, 4, 2, 0), GREEN);
    assert_eq!(pixel(&pixels, 4, 0, 3), BLUE);
    assert_eq!(pixel(&pixels, 4, 3, 3), WHITE);
}

#[test]
fn gpu_pixels_outside_the_image_keep_the_background() {
    // Given
    let gpu = gpu();
    let texture = upload_rgba8(&gpu.device, &gpu.queue, 1, 1, &RED);
    let viewport = Size {
        width: 4,
        height: 4,
    };
    let target = clear(
        &gpu.device,
        &gpu.queue,
        4,
        4,
        Color::from_rgba8(40, 40, 40, 255),
    );
    let pipeline = ImagePipeline::new(&gpu.device, target.format());

    // When: the single pixel is placed at (2,2) at scale 1
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let transform = Transform {
        scale: 1.0,
        translate: [2.0, 2.0],
    };
    pipeline.draw(
        &gpu.device,
        &gpu.queue,
        &view,
        viewport,
        &texture,
        transform,
    );
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &target);

    // Then
    assert_eq!(pixel(&pixels, 4, 2, 2), RED);
    assert_eq!(pixel(&pixels, 4, 0, 0), BACKGROUND);
    assert_eq!(pixel(&pixels, 4, 3, 3), BACKGROUND);
}

fn pixel(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let start = (y * width + x) * 4;
    [
        pixels[start],
        pixels[start + 1],
        pixels[start + 2],
        pixels[start + 3],
    ]
}
