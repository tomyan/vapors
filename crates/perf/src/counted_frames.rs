//! A workload's last frame counted and nothing else: no timing, no GPU
//! clock, no per-frame readback, no tracing dispatch. For tests that pin
//! counts (`tests/gpu_f3_baselines.rs`), where a sampled run spends most
//! of its time on what a count does not need.

use vapors_app::FrameCounts;
use vapors_gpu_harness::Gpu;
use vapors_platform::{Frame, Instant, Start};
use vapors_renderer::clear;
use vapors_scene::Color;

use crate::gpu_time::wait;
use crate::run::Workload;

/// How long one frame's GPU work may take before the run fails: a frame
/// of these pages takes about a millisecond of GPU time (the baselines'
/// recorded runs, `docs/perf/f3-baselines-2026-11-17/`), so 10 s is only
/// ever reached by a hang.
const FRAME_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

/// The counts of the frame after `changes` of `workload`'s changes, as a
/// sampled run's frame `changes - 1` counts: the first `changes - 1`
/// made with no frame between them, a priming frame drawn, the last
/// change made, and the measured frame drawn.
///
/// # Errors
/// `App::start` refusing, a frame the workload's check refuses, or a GPU
/// wait past its deadline.
///
/// # Panics
/// When `changes` is 0.
pub fn last_frame_counts(
    gpu: &Gpu,
    workload: &mut dyn Workload,
    changes: usize,
) -> Result<FrameCounts, String> {
    assert!(changes > 0, "a measured frame follows at least one change");
    let (width, height) = workload.size();
    let texture = clear(&gpu.device, &gpu.queue, width, height, Color::default());
    started(gpu, workload, texture.format())?;
    for index in 0..changes - 1 {
        workload.change(index);
    }
    drawn(gpu, workload, &texture, 0)?;
    workload.change(changes - 1);
    drawn(gpu, workload, &texture, 1)?;
    Ok(workload.counts().frame)
}

fn started(
    gpu: &Gpu,
    workload: &mut dyn Workload,
    format: wgpu::TextureFormat,
) -> Result<(), String> {
    let (width, height) = workload.size();
    let start = Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format,
        width,
        height,
        scale_factor: 1.0,
    };
    workload
        .app()
        .start(&start)
        .map_err(|error| format!("App::start: {error}"))
}

fn drawn(
    gpu: &Gpu,
    workload: &mut dyn Workload,
    texture: &wgpu::Texture,
    index: u64,
) -> Result<(), String> {
    let frame = Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture,
        index,
        scale_factor: 1.0,
        now: Instant::now(),
        resumed: false,
    };
    let next = workload.app().frame(&frame);
    wait(&gpu.device, FRAME_DEADLINE)?;
    workload
        .check(next)
        .map_err(|error| format!("frame {index}: {error}"))
}
