//! What the 60 s run refuses (docs/plans/one-scene.md, 3.2): a frame
//! over 16.7 ms from the start of its `App::frame` to the GPU idle, a
//! device without timestamp queries, and a 1-minute load average of 1.5
//! or more, or one that cannot be read, at the start or the end.

use std::time::Duration;

use crate::gpu_time::FEATURES;
use crate::sampler::JudgedFrame;

/// A 60 Hz display's frame (docs/plans/perf-finish-line.md, property 9).
pub const BUDGET: Duration = Duration::from_micros(16_700);

/// The most 1-minute load the run accepts: on a busy machine the times
/// measure the other work.
const MAX_LOAD: f64 = 1.5;

/// How to run it again.
pub const REPRODUCE: &str = "cargo run --release -p vapors-perf -- --scene-run";

/// The frames whose frame-to-idle time is over [`BUDGET`].
#[must_use]
pub fn over_budget(frames: &[JudgedFrame]) -> Vec<&JudgedFrame> {
    frames.iter().filter(|frame| frame.idle > BUDGET).collect()
}

/// Whether a device with `features`, the adapter `adapter`, times its
/// frames' GPU work.
///
/// # Errors
/// It lacks the timestamp features, naming the adapter.
pub fn require_timestamps(features: wgpu::Features, adapter: &str) -> Result<(), String> {
    if features.contains(FEATURES) {
        return Ok(());
    }
    Err(format!(
        "{adapter} lacks TIMESTAMP_QUERY and TIMESTAMP_QUERY_INSIDE_ENCODERS, so no frame's \
         GPU time can be read; run on a device with both (the dev machine's Intel UHD 630 \
         on Vulkan): {REPRODUCE}"
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
         {MAX_LOAD}: wait until `uptime` shows it, then {REPRODUCE}"
    ))
}

#[cfg(test)]
#[path = "scene_judge_tests.rs"]
mod tests;
