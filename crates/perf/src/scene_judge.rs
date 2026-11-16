//! What the 60 s run refuses (docs/plans/one-scene.md, 3.2): a frame
//! over 16.7 ms from the start of its `App::frame` to the GPU idle, a
//! device without timestamp queries, and a 1-minute load average of 1.5
//! or more, or one that cannot be read, at the start or the end.

use std::time::Duration;

use crate::sampler::JudgedFrame;
use vapors_renderer::TIMER_FEATURES;

/// The longest a frame may take to idle: one refresh of a 60 Hz display,
/// rounded as property 9 states it (docs/plans/perf-finish-line.md).
/// Not the frame clock's step (`sampler::INTERVAL`, 16 ms), which is
/// how far simulated time moves per frame.
pub const BUDGET: Duration = Duration::from_micros(16_700);

/// The most 1-minute load the run accepts: on a busy machine the times
/// measure the other work. The limit is the finish line's method
/// (docs/perf/timed-2026-11-12.md). Measured on the dev machine in the
/// window of docs/perf/scene-run-2026-11-13/log.txt: 0.87 to 1.19 at
/// the starts and ends of six back-to-back runs, so the run's own load
/// stays below it.
const MAX_LOAD: f64 = 1.5;

/// The frames whose frame-to-idle time is over [`BUDGET`].
#[must_use]
pub fn over_budget(frames: &[JudgedFrame]) -> Vec<&JudgedFrame> {
    frames.iter().filter(|frame| frame.idle > BUDGET).collect()
}

/// Whether a device with `features`, the adapter `adapter`, times its
/// frames' GPU work: whether the app's `GpuTimer` can be made on it.
///
/// # Errors
/// It lacks [`TIMER_FEATURES`], naming the adapter.
pub fn require_timestamps(features: wgpu::Features, adapter: &str) -> Result<(), String> {
    if features.contains(TIMER_FEATURES) {
        return Ok(());
    }
    Err(format!(
        "{adapter} lacks TIMESTAMP_QUERY, so the app's GpuTimer cannot read any frame's \
         GPU time; run on a device with it (the dev machine's Intel UHD 630 on Vulkan): \
         and run again"
    ))
}

/// The machine's 1-minute load average, from /proc/loadavg.
///
/// # Errors
/// It cannot be read or parsed.
pub fn load_average() -> Result<f64, String> {
    load_from(std::fs::read_to_string("/proc/loadavg"))
}

/// The first field of /proc/loadavg's `text`.
///
/// # Errors
/// The read failed, or the first field is not a number.
pub fn load_from(text: std::io::Result<String>) -> Result<f64, String> {
    let text = text.map_err(|error| format!("/proc/loadavg could not be read: {error}"))?;
    let first = text.split_whitespace().next().unwrap_or_default();
    first
        .parse()
        .map_err(|_| format!("/proc/loadavg starts {first:?}, not a load average"))
}

/// `load`, read at the run's `when`, if it is under [`MAX_LOAD`].
///
/// # Errors
/// It was not read, or it is [`MAX_LOAD`] or more, printing it.
pub fn quiet(when: &str, load: Result<f64, String>) -> Result<f64, String> {
    let load = load.map_err(|error| format!("the load at the {when} is unknown: {error}"))?;
    if load < MAX_LOAD {
        return Ok(load);
    }
    Err(format!(
        "the 1-minute load average is {load:.2} at the {when}; the run needs under \
         {MAX_LOAD}: wait until `uptime` shows it, then run again"
    ))
}

/// Whether an adapter of `device_type`, named `name` with `driver`,
/// rasterises on a GPU: an integrated or a discrete one. A software
/// adapter (`Cpu`: lavapipe, llvmpipe, `SwiftShader`) times the CPU
/// drawing the frame; a virtual or unclassified one (`VirtualGpu`,
/// `Other`) may be either, so neither is taken as a GPU.
///
/// # Errors
/// Any other type, naming it, the adapter and its driver.
pub fn require_hardware(
    device_type: wgpu::DeviceType,
    name: &str,
    driver: &str,
) -> Result<(), String> {
    let why = match device_type {
        wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu => return Ok(()),
        wgpu::DeviceType::Cpu => {
            "a software adapter, so the run would time the CPU drawing each frame"
        }
        wgpu::DeviceType::VirtualGpu | wgpu::DeviceType::Other => {
            "not known to be a GPU (it may rasterise in software), so its times may not be a GPU's"
        }
    };
    Err(format!(
        "{name} ({driver}) is {device_type:?}: {why}; run on an integrated or discrete GPU \
         (the dev machine's Intel UHD 630 on Vulkan)"
    ))
}

#[cfg(test)]
#[path = "scene_judge_tests.rs"]
mod tests;
