//! GPU tier: each demo, updated at the viewport of each section of its
//! `.pixels` file (640x400, the size of the native window and the web
//! page's screenshot, and any `at WxH` section, such as a phone's
//! width) and painted offscreen, passes that section's checks.

use vapors_app::ScenePainter;
use vapors_elements::Viewport;
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_pixels::{Image, Spec, failures, parse};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};

/// The demo at `spec`'s viewport after its inputs, painted offscreen.
fn render(name: &str, spec: &Spec) -> Vec<u8> {
    let demo = vapors_demos::find(name).expect("a demo");
    let shared = vapors_demos::document(&demo).document;
    let [width, height] = spec.viewport;
    #[allow(clippy::cast_precision_loss)] // viewports are small
    let viewport = Viewport {
        width: width as f32,
        height: height as f32,
        scale: 1.0,
    };
    if let Err(errors) = shared.borrow_mut().update(viewport) {
        panic!("{name} at {width}x{height} was refused: {errors:#?}");
    }
    for input in &spec.inputs {
        vapors_demos::input(&shared, input);
    }
    let mut document = shared.borrow_mut();
    let painted = document
        .update(viewport)
        .unwrap_or_else(|errors| panic!("{name} at {width}x{height} was refused: {errors:#?}"));
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, width, height, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let size = Size { width, height };
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: size,
        scissor: None,
    };
    ScenePainter::new(&gpu.device, texture.format())
        .draw(&target, painted)
        .expect("paints");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_every_demo_passes_its_pixel_checks_offscreen_at_every_viewport() {
    let checked = vapors_demos::all()
        .iter()
        .filter(|demo| demo.pixels.is_some())
        .count();
    assert!(
        checked > 0,
        "no demo has a .pixels file: nothing would be checked"
    );
    let mut failed = Vec::new();
    for demo in vapors_demos::all() {
        let Some(text) = demo.pixels else { continue };
        for spec in parse(demo.name, text).expect("parses") {
            // When
            let rgba = render(demo.name, &spec);

            // Then
            let [width, height] = spec.viewport;
            let image = Image {
                width,
                height,
                scale: 1.0,
                rgba: &rgba,
            };
            let found = failures(&spec.checks, &image);
            if !found.is_empty() {
                failed.push(format!(
                    "{} at {width}x{height}: {}",
                    demo.name,
                    found.join("; ")
                ));
            }
        }
    }
    assert_eq!(failed, Vec::<String>::new());
}
