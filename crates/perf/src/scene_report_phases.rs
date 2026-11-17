//! The scene run's CPU time per phase (`report::EVERY_PHASE`, in frame
//! order): median and 99th percentile over the run's frames, a phase a
//! frame did not enter counted as 0 in it. A phase inside another
//! (`batches` to `submit` and `surface` inside `draw`) is part of that
//! phase's time too.

use std::fmt::Write;
use std::time::Duration;

use crate::report::EVERY_PHASE;
use crate::sampler::JudgedFrame;
use crate::scene_report::micros;
use crate::stats::nearest_rank;

/// The phase table of `frames`, pushed onto `out`.
pub fn phases_table(out: &mut String, frames: &[JudgedFrame]) {
    out.push_str("| CPU phase | median µs | p99 µs |\n|---|---|---|\n");
    for phase in EVERY_PHASE {
        let times: Vec<Duration> = frames
            .iter()
            .map(|frame| frame.phases.get(phase).copied().unwrap_or_default())
            .collect();
        let [median, p99] =
            [50, 99].map(|p| nearest_rank(&times, p).map_or_else(|| "no samples".into(), micros));
        let _ = writeln!(out, "| {phase} | {median} | {p99} |");
    }
    out.push('\n');
}

#[cfg(test)]
#[path = "scene_report_phases_tests.rs"]
mod tests;
