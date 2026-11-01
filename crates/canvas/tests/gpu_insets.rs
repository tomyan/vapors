//! GPU tier: an image view with a right inset (a side panel) draws the
//! fitted image inside the frame less the inset, at the size it fits
//! there, and leaves the panel's pixels the background. The frame keeps
//! its full size for drawing, so the image is not stretched across it.

use vapors_canvas::{ImageView, Insets, RgbaImage};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_platform::{Event, Frame};
use vapors_renderer::clear;
use vapors_scene::{Color, Size};

const WIDTH: u32 = 200;
const HEIGHT: u32 = 100;
const BACKGROUND: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// A 10x10 red image in a 200x100 frame with a 100 px panel on the right.
fn view() -> ImageView {
    let image = RgbaImage {
        size: Size {
            width: 10,
            height: 10,
        },
        rgba: [255, 0, 0, 255].repeat(100),
    };
    let mut view = ImageView::new(image, BACKGROUND);
    view.event(&Event::Resized {
        width: WIDTH,
        height: HEIGHT,
    });
    view.set_insets(Insets {
        right: 100,
        ..Insets::default()
    });
    view
}

#[test]
fn gpu_the_image_fits_the_frame_less_its_inset_and_the_panel_keeps_the_background() {
    // Given
    let gpu = gpu();
    let mut view = view();
    let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, Color::default());

    // When
    view.draw(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: vapors_platform::Instant::now(),
    })
    .expect("draws");

    // Then: a 100x100 square of red at the left; the panel blue
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let pixel = |x: u32, y: u32| pixels[((y * WIDTH + x) * 4) as usize..][..4].to_vec();
    assert_eq!(
        pixel(5, 50),
        vec![255, 0, 0, 255],
        "from the area's left edge"
    );
    assert_eq!(pixel(50, 50), vec![255, 0, 0, 255], "the image in the area");
    assert_eq!(
        pixel(98, 98),
        vec![255, 0, 0, 255],
        "fitted to the area's 100x100"
    );
    assert_eq!(pixel(150, 50), BACKGROUND.to_rgba8().to_vec(), "the panel");
}
