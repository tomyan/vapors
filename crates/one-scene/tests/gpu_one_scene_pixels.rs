//! GPU tier (lavapipe): the example's first five frames at 640x400, the
//! last held to `one_scene.pixels`, the file the page contract holds the
//! browser's screenshot to.

use vapors_gpu_harness::read_rgba8;
use vapors_one_scene::OneScene;
use vapors_one_scene::schedule::time_of;
use vapors_pixels::{DEFAULT_VIEWPORT, Image, failures, parse, section};
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::clear;
use vapors_scene::Color;

const PIXELS: &str = include_str!("../one_scene.pixels");

#[test]
fn gpu_the_fifth_frame_holds_the_pixels_the_browser_is_held_to() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let mut scene = OneScene::new(Some(5)).expect("mounts");
    let [width, height] = DEFAULT_VIEWPORT;
    let start = Instant::now();

    // When
    let mut rgba = Vec::new();
    for index in 0..5 {
        let texture = clear(&gpu.device, &gpu.queue, width, height, Color::default());
        let now = start + time_of(index);
        let (adapter, device, queue) = (&gpu.adapter, &gpu.device, &gpu.queue);
        let index = index as u64;
        scene.frame(&Frame {
            adapter,
            device,
            queue,
            texture: &texture,
            index,
            scale_factor: 1.0,
            now,
        });
        rgba = read_rgba8(&gpu.device, &gpu.queue, &texture);
    }

    // Then
    let specs = parse("one_scene.pixels", PIXELS).expect("parses");
    let spec = section(&specs, DEFAULT_VIEWPORT).expect("a 640x400 section");
    let image = Image {
        width,
        height,
        scale: 1.0,
        rgba: &rgba,
    };
    assert_eq!(failures(&spec.checks, &image), Vec::<String>::new());
}
