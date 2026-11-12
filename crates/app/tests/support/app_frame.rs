//! One `ElementsApp` frame into a fresh, readable target.

use vapors_app::ElementsApp;
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_platform::{App, Frame, Instant, Next};
use vapors_renderer::clear;
use vapors_scene::Color;

use super::SIZE;

/// One app frame into a fresh `SIZE` square: what it asked for next,
/// and the frame's RGBA bytes.
pub fn app_frame(app: &mut ElementsApp, gpu: &Gpu) -> (Next, Vec<u8>) {
    app_frame_at(app, gpu, Instant::now())
}

/// As [`app_frame`], with the frame's time `now`.
pub fn app_frame_at(app: &mut ElementsApp, gpu: &Gpu, now: Instant) -> (Next, Vec<u8>) {
    let texture = clear(&gpu.device, &gpu.queue, SIZE, SIZE, Color::default());
    let next = app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
        now,
    });
    (next, read_rgba8(&gpu.device, &gpu.queue, &texture))
}
