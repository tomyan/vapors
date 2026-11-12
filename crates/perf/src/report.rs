//! A workload's samples as Markdown: its counts, then a row per phase
//! with the median, 90th percentile and maximum in microseconds.

use std::fmt::Write;
use std::time::Duration;

use crate::instruction_samples::InstructionSamples;
use crate::run::{Counts, Samples};
use crate::stats::{Distribution, median_count};

/// The phases reported, in frame order: the app's `update` (which holds
/// `style`, `layout` and `paint`) and `draw`, which holds `encode` (each
/// draw call's bind group and render pass, and `finish` on the frame's
/// encoder), `write` (`Queue::write_buffer` and `write_texture`) and
/// `submit` (`Queue::submit`: once per frame, plus once per glyph atlas
/// repack and per `gpu-surface` that renders); the rest of draw is the
/// painter's own work. The app's other phases (input, tasks, tick,
/// accessibility, record) do no work in these workloads and are left out.
pub const PHASES: [&str; 8] = [
    "update", "style", "layout", "paint", "draw", "encode", "write", "submit",
];

/// `samples` of the workload `name` as a Markdown section.
#[must_use]
pub fn section(name: &str, samples: &Samples) -> String {
    let mut out = times_section(name, samples, &PHASES);
    match &samples.gpu {
        Some(spans) => row(&mut out, SPAN, spans),
        None => no_timestamps(&mut out, SPAN),
    }
    match &samples.gpu_time {
        Some(times) => row(&mut out, "GPU time", times),
        None => no_timestamps(&mut out, "GPU time"),
    }
    if let Some(resolution) = samples.gpu_resolution {
        let _ = writeln!(out, "| GPU time resolution | {resolution} | | |");
    }
    out
}

/// `section` without the GPU rows: for a workload whose frames are not
/// timed on the GPU (`Workload::times_gpu`).
#[must_use]
pub fn times_section(name: &str, samples: &Samples, phases: &[&str]) -> String {
    let mut out = format!("### {name}\n\n{}\n\n", counts(&samples.counts));
    out.push_str("| phase | median µs | p90 µs | max µs |\n|---|---|---|---|\n");
    row(&mut out, "frame (CPU)", &samples.frame);
    for &phase in phases {
        let times = samples.phases.get(phase).map_or(&[][..], Vec::as_slice);
        row(&mut out, phase, times);
    }
    out
}

/// `samples`' instructions as a Markdown section: the counts line, then
/// each phase's median instructions per frame.
///
/// # Errors
/// `samples` holds no instruction counts.
pub fn instructions_section(name: &str, samples: &Samples) -> Result<String, String> {
    let kept = samples
        .instructions
        .as_ref()
        .ok_or_else(|| format!("{name}: no instruction counts were taken"))?;
    let mut out = format!("### {name}\n\n{}\n\n", counts(&samples.counts));
    out.push_str("| phase | median instructions |\n|---|---|\n");
    instruction_row(&mut out, "frame (CPU)", &kept.frame);
    for phase in PHASES {
        instruction_row(&mut out, phase, phase_counts(kept, phase));
    }
    Ok(out)
}

fn phase_counts<'a>(kept: &'a InstructionSamples, phase: &str) -> &'a [u64] {
    kept.phases.get(phase).map_or(&[][..], Vec::as_slice)
}

fn instruction_row(out: &mut String, phase: &str, counts: &[u64]) {
    let _ = match median_count(counts) {
        Some(median) => writeln!(out, "| {phase} | {median} |"),
        None => writeln!(out, "| {phase} | no samples |"),
    };
}

/// The span from before the frame's first submit to after its last.
const SPAN: &str = "GPU span (upper bound)";

fn no_timestamps(out: &mut String, phase: &str) {
    let _ = writeln!(out, "| {phase} | no timestamp queries | | |");
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

/// The last frame's counts on one line, and where its styles came from
/// on a second (analysis PR 2: shared, computed, unkeyed, unshareable;
/// live after the update).
fn counts(counts: &Counts) -> String {
    let (u, gpu) = (&counts.update, counts.frame.total());
    format!(
        "Counts (last frame): restyled {}, text layouts {}, laid out {}, elements visited {}, \
         boxes built {}, layout calls {}, boxes painted {}, boxes shifted {}, scene items {}, \
         culled {}; draw calls {}, buffer bytes {}, texture bytes {}, uniform writes {}, \
         bind groups {}, render passes {}.\n{}",
        u.elements_restyled,
        counts.text_layouts,
        u.boxes_laid_out,
        u.elements_visited,
        u.boxes_built,
        u.layout_calls,
        u.boxes_painted,
        u.boxes_shifted,
        u.scene_items_built,
        counts.frame.culled,
        gpu.draw_calls,
        gpu.buffer_bytes,
        gpu.texture_bytes,
        gpu.uniform_writes,
        gpu.bind_groups_made,
        counts.render_passes,
        style_counts(u)
    )
}

/// Where the last frame's restyled elements took their styles from.
fn style_counts(u: &vapors_elements::UpdateCounts) -> String {
    format!(
        "Styles (last frame): shared {}, computed {}, unkeyed {}, unshareable {}; live {}.",
        u.styles_shared, u.styles_computed, u.styles_unkeyed, u.styles_unshareable, u.styles_live
    )
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
