//! GPU tier: the stress demo's counter ticks each frame its per-frame
//! hook runs, and its ink changes between two frames (F2c S7).

use vapors_app::ScenePainter;
use vapors_elements::Viewport;
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

/// The tick's text, x 8..120 and y 6..30 at 640x400.
const TICK: [u32; 4] = [8, 6, 120, 30];

fn painted_tick(shown: &vapors_demos::Shown) -> Vec<u8> {
    let mut document = shown.document.borrow_mut();
    let painted = document.update(VIEWPORT).expect("updates");
    let gpu = gpu();
    let texture = clear(&gpu.device, &gpu.queue, 640, 400, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: Size {
            width: 640,
            height: 400,
        },
        scissor: None,
    };
    ScenePainter::new(&gpu.device, texture.format())
        .draw(&target, painted)
        .expect("paints");
    let rgba = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let [left, top, right, bottom] = TICK;
    (top..bottom)
        .flat_map(|y| (left..right).map(move |x| ((y * 640 + x) * 4) as usize))
        .flat_map(|at| rgba[at..at + 3].to_vec())
        .collect()
}

#[test]
fn gpu_the_stress_counters_ink_changes_from_one_frame_to_the_next() {
    // Given
    let demo = vapors_demos::find("stress").expect("the stress demo");
    let mut shown = vapors_demos::document(&demo);
    let mut per_frame = shown.per_frame.take().expect("stress changes every frame");
    per_frame(&[]);
    let first = painted_tick(&shown);

    // When
    per_frame(&[("update", std::time::Duration::from_millis(1))]);
    let second = painted_tick(&shown);

    // Then
    assert_ne!(
        first, second,
        "the tick's pixels are the same after a frame"
    );
}
