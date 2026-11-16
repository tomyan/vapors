//! GPU tier (lavapipe): the example's first five frames at 640x400, the
//! last held to `one_scene.pixels`, the file the page contract holds the
//! browser's screenshot to; and the same frames with the pointer over
//! the cubes, the group's `:hover` colouring the middle cube.

use vapors_gpu_harness::read_rgba8;
use vapors_one_scene::OneScene;
use vapors_one_scene::schedule::time_of;
use vapors_pixels::{DEFAULT_VIEWPORT, Image, failures, parse, section};
use vapors_platform::{App, Event, Frame, Instant, Platform};
use vapors_renderer::clear;
use vapors_scene::Color;

const PIXELS: &str = include_str!("../one_scene.pixels");

/// The first five frames at 640x400, the pointer moved to `pointer`
/// after the first; the last frame's pixels.
fn five_frames(pointer: Option<[f32; 2]>) -> Vec<u8> {
    let gpu = vapors_gpu_harness::gpu();
    let mut scene = OneScene::new(Some(5)).expect("mounts");
    let [width, height] = DEFAULT_VIEWPORT;
    let start = Instant::now();
    let mut rgba = Vec::new();
    for index in 0..5 {
        if let (1, Some([x, y])) = (index, pointer) {
            scene.event(Event::PointerMoved { x, y }, &Platform::detached());
        }
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
    rgba
}

#[test]
fn gpu_the_fifth_frame_holds_the_pixels_the_browser_is_held_to() {
    // Given
    let [width, height] = DEFAULT_VIEWPORT;

    // When
    let rgba = five_frames(None);

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

#[test]
fn gpu_the_pointer_over_the_cubes_colours_the_middle_cube_by_the_groups_hover() {
    // Given: the surface fills the page, so its middle is (320, 200)
    let [width, _] = DEFAULT_VIEWPORT;

    // When
    let rgba = five_frames(Some([320.0, 200.0]));

    // Then: cube 2's front face (the file's 320 220) is `.spin:hover .two`
    let at = (220 * width as usize + 320) * 4;
    let pixel = &rgba[at..at + 4];
    let want = [0xe6, 0x7e, 0x22, 0xff];
    let near = pixel
        .iter()
        .zip(want)
        .all(|(&got, want): (&u8, u8)| got.abs_diff(want) <= 1);
    assert!(near, "cube 2's front face is {pixel:?}, not #e67e22");
}
