//! A workload's samples as Markdown: its counts, then a row per phase
//! with the median, 90th percentile and maximum in microseconds.

use std::fmt::Write;
use std::time::Duration;

use crate::run::{Counts, Samples};
use crate::stats::Distribution;

/// The phases reported, in frame order: the app's `update` (which holds
/// `style`, `layout` and `paint`) and `draw` (encoding, uploads and
/// submits). The app's other phases (input, tasks, tick, accessibility,
/// record) do no work in these workloads and are left out.
pub const PHASES: [&str; 5] = ["update", "style", "layout", "paint", "draw"];

/// `samples` of the workload `name` as a Markdown section.
#[must_use]
pub fn section(name: &str, samples: &Samples) -> String {
    let mut out = format!("### {name}\n\n{}\n\n", counts(&samples.counts));
    out.push_str("| phase | median µs | p90 µs | max µs |\n|---|---|---|---|\n");
    row(&mut out, "frame (CPU)", &samples.frame);
    for phase in PHASES {
        let times = samples.phases.get(phase).map_or(&[][..], Vec::as_slice);
        row(&mut out, phase, times);
    }
    match &samples.gpu {
        Some(spans) => row(&mut out, "GPU span", spans),
        None => out.push_str("| GPU span | no timestamp queries | | |\n"),
    }
    out
}

fn row(out: &mut String, phase: &str, times: &[Duration]) {
    let Some(d) = Distribution::of(times) else {
        let _ = writeln!(out, "| {phase} | no samples | | |");
        return;
    };
    let _ = writeln!(
        out,
        "| {phase} | {} | {} | {} |",
        micros(d.median),
        micros(d.p90),
        micros(d.max)
    );
}

fn micros(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1e6)
}

/// The last frame's counts on one line.
fn counts(counts: &Counts) -> String {
    let (u, gpu) = (&counts.update, counts.frame.total());
    format!(
        "Counts (last frame): restyled {}, text layouts {}, laid out {}, elements visited {}, \
         boxes built {}, layout calls {}, boxes painted {}, scene items {}; draw calls {}, \
         buffer bytes {}, texture bytes {}.",
        u.elements_restyled,
        counts.text_layouts,
        u.boxes_laid_out,
        u.elements_visited,
        u.boxes_built,
        u.layout_calls,
        u.boxes_painted,
        u.scene_items_built,
        gpu.draw_calls,
        gpu.buffer_bytes,
        gpu.texture_bytes
    )
}
