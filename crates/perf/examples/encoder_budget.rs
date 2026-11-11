//! How many draws one `FrameEncoder` holds before `CommandEncoder::finish`
//! fails or slows: N one-pixel quad draws, each its own `QuadPipeline`
//! draw, recorded in one pass (consecutive draws on one view) or one
//! pass per draw (a raw record between draws), then finished, submitted
//! and waited for, each on a device of its own. Prints the times and
//! any GPU error, per N.
//!
//! `cargo run --release -p vapors-perf --example encoder_budget [-- N...]`

use std::time::{Duration, Instant};

use vapors_gpu_harness::{Gpu, gpu};
use vapors_renderer::{EncoderBudget, FrameEncoder, QuadPipeline, Target, clear};
use vapors_scene::{Color, Quad, Rect, Size};

const SIDE: u32 = 256;
const COUNTS: [u32; 7] = [1_000, 3_000, 9_000, 27_000, 81_000, 243_000, 729_000];

fn main() {
    println!("adapter: {:?}", gpu().adapter.get_info().name);
    println!("draws  passes  record ms  submit+wait ms  error");
    let asked: Vec<u32> = std::env::args()
        .skip(1)
        .map(|arg| arg.parse().unwrap_or_else(|_| panic!("not a count: {arg}")))
        .collect();
    for count in if asked.is_empty() {
        COUNTS.to_vec()
    } else {
        asked
    } {
        for split in [false, true] {
            let (record, done, error) = run(&gpu(), count, split);
            let passes = if split { count } else { 1 };
            println!(
                "{count:>7} {passes:>7} {:>10.1} {:>15.1}  {error}",
                ms(record),
                ms(done)
            );
        }
    }
}

fn run(gpu: &Gpu, count: u32, split: bool) -> (Duration, Duration, String) {
    let texture = clear(&gpu.device, &gpu.queue, SIDE, SIDE, Color::default());
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let scopes = [
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Validation,
    ]
    .map(|filter| gpu.device.push_error_scope(filter));
    let start = Instant::now();
    let encoder = recorded(gpu, &view, count, split);
    let recording = start.elapsed();
    let submitted = Instant::now();
    let refused = encoder.submit(&gpu.queue).err();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the device answers");
    let done = submitted.elapsed();
    let errors = popped(scopes).chain(refused.map(|r| r.to_string()));
    (recording, done, errors.collect::<Vec<_>>().join("; "))
}

/// `count` draws into an encoder with no budget, so it never flushes;
/// with `split`, each draw in a pass of its own.
fn recorded(gpu: &Gpu, view: &wgpu::TextureView, count: u32, split: bool) -> FrameEncoder {
    let mut pipeline = QuadPipeline::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    let unlimited = EncoderBudget {
        passes: u32::MAX,
        draws: u32::MAX,
    };
    let encoder = FrameEncoder::with_budget(&gpu.device, unlimited);
    for index in 0..count {
        pipeline.draw(&target(gpu, &encoder, view), &[pixel(index)]);
        if split {
            encoder.record(|_| ());
        }
    }
    encoder
}

/// The errors `scopes` caught, innermost first.
fn popped(scopes: [wgpu::ErrorScopeGuard; 2]) -> impl Iterator<Item = String> {
    scopes
        .into_iter()
        .rev()
        .filter_map(|scope| pollster::block_on(scope.pop()).map(|e| e.to_string()))
}

fn target<'a>(gpu: &'a Gpu, encoder: &'a FrameEncoder, view: &'a wgpu::TextureView) -> Target<'a> {
    Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder,
        view,
        viewport: Size {
            width: SIDE,
            height: SIDE,
        },
        scissor: None,
    }
}

#[allow(clippy::cast_precision_loss)] // pixel positions under 256
fn pixel(index: u32) -> Quad {
    let (x, y) = ((index % SIDE) as f32, ((index / SIDE) % SIDE) as f32);
    Quad::fill(
        Rect::new(x, y, x + 1.0, y + 1.0),
        Color::from_rgba8(255, 0, 0, 255),
    )
}

fn ms(time: Duration) -> f64 {
    time.as_secs_f64() * 1000.0
}
