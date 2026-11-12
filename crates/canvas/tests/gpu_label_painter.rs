//! GPU tier: the label strip is drawn after the image, so it stays on
//! top of a painter that fills the whole viewport, which owns it
//! (`ImagePainter`'s contract: no clip, overlays after).

use vapors_canvas::{ImageApp, ImagePainter, STRIP_COLOR, strip_height};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_platform::{App, Event, Frame, Platform};
use vapors_renderer::{clear, clear_view};
use vapors_scene::{Color, Size, Transform};

const WIDTH: u32 = 200;
const HEIGHT: u32 = 100;
const FILL: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// Fills the whole target, as a painter may.
struct Flood;

impl ImagePainter for Flood {
    fn paint(
        &mut self,
        frame: &Frame<'_>,
        target: &wgpu::TextureView,
        _: Size,
        _: Transform,
    ) -> Result<(), String> {
        clear_view(frame.device, frame.queue, target, FILL);
        Ok(())
    }
}

#[test]
fn gpu_the_label_strip_stays_on_top_of_a_painter_that_fills_the_viewport() {
    // Given
    let gpu = gpu();
    let mut app = ImageApp::new(Color::default());
    app.event(
        Event::Resized {
            width: WIDTH,
            height: HEIGHT,
        },
        &Platform::detached(),
    );
    let size = Size {
        width: 10,
        height: 10,
    };
    app.view_mut().set_painter(size, Box::new(Flood));
    let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, Color::default());

    // When
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: vapors_platform::Instant::now(),
    });

    // Then: the strip's corner is the strip, below it the painter's fill
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let pixel = |x: u32, y: u32| pixels[((y * WIDTH + x) * 4) as usize..][..4].to_vec();
    assert_eq!(pixel(0, 0), STRIP_COLOR.to_rgba8());
    assert_eq!(pixel(WIDTH / 2, strip_height(1.0) + 10), FILL.to_rgba8());
}
