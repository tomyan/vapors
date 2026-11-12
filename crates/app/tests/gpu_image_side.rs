//! GPU tier: an `ElementsApp` frame tells its document the device's
//! longest texture side, so an image over WebGPU's guaranteed 8192 px
//! that the device can upload is registered after the first frame.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::clear;
use vapors_scene::{Color, RgbaImage, Size};
use vapors_text::FontSystem;

fn row(width: u32) -> RgbaImage {
    RgbaImage {
        size: Size { width, height: 1 },
        rgba: [0, 0, 0, 255].repeat(width as usize),
    }
}

fn run_frame(app: &mut ElementsApp, gpu: &Gpu) {
    let texture = clear(&gpu.device, &gpu.queue, 16, 16, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now: Instant::now(),
    });
}

#[test]
fn gpu_after_a_frame_the_document_accepts_an_image_as_wide_as_the_device_allows() {
    // Given: a device whose limit is over the guaranteed 8192
    let gpu = gpu();
    let limit = gpu.device.limits().max_texture_dimension_2d;
    assert!(
        limit > 8192,
        "this test needs a device over 8192 px; {limit}"
    );
    let document = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let mut app = ElementsApp::new(Rc::clone(&document));
    assert!(
        document
            .borrow_mut()
            .register_image("wide.png", row(limit))
            .is_err()
    );

    // When
    run_frame(&mut app, &gpu);

    // Then
    let mut document = document.borrow_mut();
    assert_eq!(document.register_image("wide.png", row(limit)), Ok(()));
    assert!(
        document
            .register_image("wider.png", row(limit + 1))
            .is_err()
    );
}
