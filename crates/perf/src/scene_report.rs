//! The 60 s run as Markdown (docs/plans/one-scene.md, 3.2): median,
//! 99th percentile and maximum of CPU, GPU and frame-to-idle time, the
//! frames over budget, each frame's GPU work, the first 20 frames on
//! their own, and the load average at the start and the end.

use std::fmt::Write;
use std::time::Duration;

use crate::driver::{RUN_FRAMES, frame_time};
use crate::sampler::JudgedFrame;
use crate::scene_judge::{BUDGET, over_budget};
use crate::stats::{median_count, nearest_rank};

/// Frames printed one by one from the first: they hold first-use costs.
const FIRST: usize = 20;

/// `frames` of the run, with the load at its start and its end (`None`
/// when it could not be read).
#[must_use]
pub fn report(frames: &[JudgedFrame], start: f64, end: Option<f64>) -> String {
    let end = end.map_or_else(|| "unread".into(), |end| format!("{end:.2}"));
    let mut out = format!(
        "### racer, one scene, 60 s\n\nFrames {} ({RUN_FRAMES} planned); over {} ms to idle: {}. \
         Load average (1 minute) at the start {start:.2}, at the end {end}.\n\n",
        frames.len(),
        BUDGET.as_secs_f64() * 1e3,
        over_budget(frames).len()
    );
    times_table(&mut out, frames);
    work_table(&mut out, frames);
    first_frames(&mut out, frames);
    out
}

fn times_table(out: &mut String, frames: &[JudgedFrame]) {
    out.push_str("| time | median µs | p99 µs | max µs |\n|---|---|---|---|\n");
    for (name, pick) in TIMES {
        let times: Vec<Duration> = frames.iter().map(pick).collect();
        let [median, p99, max] = [50, 99, 100]
            .map(|p| nearest_rank(&times, p).map_or_else(|| "no samples".into(), micros));
        let _ = writeln!(out, "| {name} | {median} | {p99} | {max} |");
    }
    out.push('\n');
}

type Pick<T> = fn(&JudgedFrame) -> T;

const TIMES: [(&str, Pick<Duration>); 3] = [
    ("frame (CPU)", |f| f.cpu),
    ("GPU time", |f| f.gpu),
    ("frame to idle", |f| f.idle),
];

const WORK: [(&str, Pick<u64>); 3] = [
    ("draw calls", |f| f.counts.frame.total().draw_calls),
    ("buffer bytes", |f| f.counts.frame.total().buffer_bytes),
    ("texture bytes (atlas pages and images)", |f| {
        f.counts.frame.total().texture_bytes
    }),
];

fn work_table(out: &mut String, frames: &[JudgedFrame]) {
    out.push_str("| per frame | median | max |\n|---|---|---|\n");
    for (name, pick) in WORK {
        let counts: Vec<u64> = frames.iter().map(pick).collect();
        let shown =
            |value: Option<u64>| value.map_or_else(|| "no samples".into(), |v| v.to_string());
        let median = shown(median_count(&counts));
        let max = shown(counts.iter().max().copied());
        let _ = writeln!(out, "| {name} | {median} | {max} |");
    }
    out.push('\n');
}

fn first_frames(out: &mut String, frames: &[JudgedFrame]) {
    out.push_str("| first frames | CPU µs | GPU µs | to idle µs |\n|---|---|---|---|\n");
    for f in frames.iter().take(FIRST) {
        let [cpu, gpu, idle] = [f.cpu, f.gpu, f.idle].map(micros);
        let _ = writeln!(out, "| {} | {cpu} | {gpu} | {idle} |", f.index);
    }
}

/// One line per frame over budget: its index, its time on the frame
/// clock, its times and phases, and what it did.
#[must_use]
pub fn over_budget_lines(frames: &[&JudgedFrame]) -> String {
    frames.iter().map(|f| over_budget_line(f) + "\n").collect()
}

fn over_budget_line(f: &JudgedFrame) -> String {
    let phases: Vec<String> = f
        .phases
        .iter()
        .map(|(name, time)| format!("{name} {}", micros(*time)))
        .collect();
    let work = f.counts.frame.total();
    format!(
        "frame {} at {:.3} s: to idle {} µs, CPU {} µs, GPU {} µs; phases (µs) {}; draw calls {}, \
         buffer bytes {}, texture bytes {}, restyled {}, scene items {}",
        f.index,
        frame_time(f.index).as_secs_f64(),
        micros(f.idle),
        micros(f.cpu),
        micros(f.gpu),
        phases.join(", "),
        work.draw_calls,
        work.buffer_bytes,
        work.texture_bytes,
        f.counts.update.elements_restyled,
        f.counts.update.scene_items_built
    )
}

fn micros(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1e6)
}

#[cfg(test)]
#[path = "scene_report_tests.rs"]
mod tests;
