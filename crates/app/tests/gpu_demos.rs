//! GPU tier: each demo, updated at the viewport of each section of its
//! `.pixels` file (640x400, the size of the native window and the web
//! page's screenshot, and any `at WxH` section, such as a phone's
//! width) and painted offscreen, passes that section's checks, and
//! draws the same pixels at 4 samples.

use vapors_app::ScenePainter;
use vapors_elements::Viewport;
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_pixels::{Image, Spec, failures, parse};
use vapors_renderer::{FrameEncoder, PassFormat, Samples, Target, clear};
use vapors_scene::{Color, Size};

/// The demo at `spec`'s viewport after its inputs, painted offscreen
/// at `samples`.
fn render(gpu: &Gpu, name: &str, spec: &Spec, samples: Samples) -> Vec<u8> {
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
    let texture = clear(&gpu.device, &gpu.queue, width, height, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let size = Size { width, height };
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        multisampled: None,
        depth: None,
        viewport: size,
        scissor: None,
    };
    let pass = PassFormat {
        format: texture.format(),
        samples,
    };
    ScenePainter::new(&gpu.device, pass)
        .expect("Rgba8Unorm is drawn")
        .draw(&target, painted)
        .expect("paints");
    encoder.submit(&gpu.queue).expect("no draw refused");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_every_demo_passes_its_pixel_checks_offscreen_at_every_viewport_and_draws_alike_at_4_samples()
{
    let gpu = gpu();
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
            let rgba = render(&gpu, demo.name, &spec, Samples::One);
            let four = render(&gpu, demo.name, &spec, Samples::Four);

            // Then: the checks pass, and 4 samples draw the same pixels,
            // as measured on 2026-11-17 on ANV (UHD 630) and llvmpipe:
            // quads and text on whole pixels cover every sample
            // (`docs/plans/flat-scene-f7.md`, Q7)
            let [width, height] = spec.viewport;
            if four != rgba {
                failed.push(format!(
                    "{} at {width}x{height}: 4 samples differ from 1",
                    demo.name
                ));
            }
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
