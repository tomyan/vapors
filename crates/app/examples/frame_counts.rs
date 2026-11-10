//! What a frame uploads and draws, for hello, counter and stress at
//! 640x400 and 1x, as `ScenePainter::frame_counts` reports it:
//! `cargo run -p vapors-app --example frame_counts`. Prints the first
//! frame and the second of two identical frames per demo, the figures in
//! docs/plans/iu1.md. Needs a GPU adapter (the gpu tier's).

use vapors_app::{FrameCounts, ScenePainter};
use vapors_elements::Viewport;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

fn main() {
    let gpu = gpu();
    for name in ["hello", "counter", "stress"] {
        let demo = match vapors_demos::find(name) {
            Ok(demo) => demo,
            Err(error) => {
                eprintln!("FAIL frame_counts: {error}");
                std::process::exit(1);
            }
        };
        let shown = vapors_demos::document(&demo);
        let (first, idle) = two_frames(&gpu, &shown);
        print_row(name, "first", &first);
        print_row(name, "idle", &idle);
    }
}

/// The counts of the first frame and of the second, with nothing changed.
fn two_frames(gpu: &Gpu, shown: &vapors_demos::Shown) -> (FrameCounts, FrameCounts) {
    let mut document = shown.document.borrow_mut();
    let scene = document.update(VIEWPORT).expect("updates");
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
    let mut painter = ScenePainter::new(&gpu.device, texture.format()).expect("Rgba8Unorm");
    painter.draw(&target, scene).expect("paints");
    let first = painter.frame_counts();
    painter.draw(&target, scene).expect("paints");
    (first, painter.frame_counts())
}

fn print_row(name: &str, frame: &str, counts: &FrameCounts) {
    let total = counts.total();
    println!(
        "{name:8} {frame:6} draw calls {:5}  buffer bytes {:8}  texture bytes {:9}",
        total.draw_calls, total.buffer_bytes, total.texture_bytes
    );
}
