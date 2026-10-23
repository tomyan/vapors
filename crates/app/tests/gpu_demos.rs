//! GPU tier: each demo, updated at the page's 640x400 and painted
//! offscreen, passes its own `.pixels` checks: the same checks the
//! native window's and the web page's screenshots are held to.

use vapors_app::ScenePainter;
use vapors_elements::Viewport;
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_pixels::{Image, failures, parse};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 400;

fn render(name: &str) -> Vec<u8> {
    let demo = vapors_demos::find(name).expect("a demo");
    let document = vapors_demos::document(&demo).expect("its style parses");
    let mut document = document.borrow_mut();
    let viewport = Viewport {
        width: 640.0,
        height: 400.0,
        scale: 1.0,
    };
    let painted = document
        .update(viewport)
        .unwrap_or_else(|errors| panic!("{name} was refused: {errors:#?}"));
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, WIDTH, HEIGHT, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let size = Size {
        width: WIDTH,
        height: HEIGHT,
    };
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: size,
    };
    ScenePainter::new(&gpu.device, texture.format())
        .draw(&target, painted)
        .expect("paints");
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_every_demo_passes_its_pixel_checks_offscreen() {
    let checked = vapors_demos::all()
        .iter()
        .filter(|demo| demo.pixels.is_some())
        .count();
    assert!(
        checked > 0,
        "no demo has a .pixels file: nothing would be checked"
    );
    for demo in vapors_demos::all() {
        // Given
        let Some(text) = demo.pixels else { continue };
        let checks = parse(demo.name, text).expect("parses");

        // When
        let rgba = render(demo.name);

        // Then
        let image = Image {
            width: WIDTH,
            height: HEIGHT,
            scale: 1.0,
            rgba: &rgba,
        };
        assert_eq!(
            failures(&checks, &image),
            Vec::<String>::new(),
            "{}",
            demo.name
        );
    }
}
