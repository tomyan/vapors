//! GPU tier: an image stretched over a rect, as an `img` is drawn,
//! inside the target's scissor.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{ImagePipeline, Target, clear, upload_rgba8};
use vapors_scene::{Color, Rect, Scissor, Size};

const RED: [u8; 4] = [211, 47, 47, 255];
const BLUE: [u8; 4] = [25, 118, 210, 255];
const GREY: [u8; 4] = [40, 40, 40, 255];

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let start = ((y * width + x) * 4) as usize;
    pixels[start..start + 4].try_into().expect("four bytes")
}

/// A 2 x 1 image, red then blue, stretched over `rect` on a 16 x 8 grey
/// target under `scissor`.
fn draw(rect: Rect, scissor: Option<Scissor>) -> Vec<u8> {
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        16,
        8,
        Color::from_rgba8(40, 40, 40, 255),
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let image = upload_rgba8(&gpu.device, &gpu.queue, 2, 1, &[RED, BLUE].concat());
    let pipeline = ImagePipeline::new(&gpu.device, target.format());
    let at = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: Size {
            width: 16,
            height: 8,
        },
        scissor,
    };
    pipeline.draw_rect(&at, &image, rect);
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

#[test]
fn gpu_an_image_stretches_over_its_rect_and_nowhere_else() {
    // When: 2 x 1 image pixels over 8 x 4 device pixels at (4, 2)
    let pixels = draw(Rect::new(4.0, 2.0, 12.0, 6.0), None);

    // Then: red on the left half, blue on the right, grey around
    assert_eq!(pixel(&pixels, 16, 5, 3), RED);
    assert_eq!(pixel(&pixels, 16, 10, 4), BLUE);
    assert_eq!(pixel(&pixels, 16, 2, 3), GREY);
    assert_eq!(pixel(&pixels, 16, 13, 3), GREY);
    assert_eq!(pixel(&pixels, 16, 6, 1), GREY);
}

#[test]
fn gpu_an_image_is_cut_to_the_scissor() {
    // Given: a scissor over the left 8 columns
    let scissor = Scissor {
        x: 0,
        y: 0,
        width: 8,
        height: 8,
    };

    // When
    let pixels = draw(Rect::new(4.0, 2.0, 12.0, 6.0), Some(scissor));

    // Then: the blue half is cut off
    assert_eq!(pixel(&pixels, 16, 5, 3), RED);
    assert_eq!(pixel(&pixels, 16, 10, 4), GREY);
}
