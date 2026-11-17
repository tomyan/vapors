//! `--counters`' report: per phase, the median cycles, instructions per
//! cycle, branch misses, L1d and last-level cache read misses, and the
//! bytes those last-level misses bring in. Diagnostics for a quiet
//! window: the counts move with whatever else shares the core and its
//! caches, so nothing judges them.

use std::fmt::Write;

use crate::counter_samples::CounterSamples;
use crate::hardware_counters::{CYCLES, INSTRUCTIONS, LINE_BYTES, Reading};
use crate::report::PHASES;
use crate::run::Samples;
use crate::stats::median_count;

/// Printed under every counters section.
pub const QUIET_WINDOW: &str = "Hardware counts are diagnostics for a quiet window: they move with \
     what else runs on the core and its caches, and nothing judges them. LLC miss bytes are LLC \
     read misses times a 64-byte line: reads only, prefetches not counted.";

const HEADER: &str = "| phase | cycles | IPC | branch misses | L1d read misses | LLC read misses | LLC miss bytes |\n\
                      |---|---|---|---|---|---|---|\n";

/// `samples`' hardware counts as a Markdown section.
///
/// # Errors
/// `samples` holds no hardware counts.
pub fn counters_section(name: &str, samples: &Samples) -> Result<String, String> {
    let kept = samples
        .counters
        .as_ref()
        .ok_or_else(|| format!("{name}: no hardware counts were taken"))?;
    let mut out = format!("### {name}\n\n{HEADER}");
    counter_row(&mut out, "frame (CPU)", &kept.frame);
    for phase in PHASES {
        counter_row(&mut out, phase, phase_readings(kept, phase));
    }
    let _ = writeln!(out, "\n{QUIET_WINDOW}");
    Ok(out)
}

fn phase_readings<'a>(kept: &'a CounterSamples, phase: &str) -> &'a [Reading] {
    kept.phases.get(phase).map_or(&[][..], Vec::as_slice)
}

fn counter_row(out: &mut String, phase: &str, readings: &[Reading]) {
    let median = |event: usize| {
        let counts: Vec<u64> = readings.iter().map(|reading| reading[event]).collect();
        median_count(&counts)
    };
    let Some(cycles) = median(CYCLES) else {
        let _ = writeln!(out, "| {phase} | no samples | | | | | |");
        return;
    };
    let [branch, l1d, llc] = [2, 3, 4].map(|event| median(event).unwrap_or(0));
    let _ = writeln!(
        out,
        "| {phase} | {cycles} | {} | {branch} | {l1d} | {llc} | {} |",
        ipc(readings),
        llc * LINE_BYTES
    );
}

/// The median of each frame's instructions per cycle, to two places;
/// `-` when no frame ran a cycle in the phase.
fn ipc(readings: &[Reading]) -> String {
    let per_thousand: Vec<u64> = readings
        .iter()
        .filter(|reading| reading[CYCLES] > 0)
        .map(|reading| reading[INSTRUCTIONS] * 1000 / reading[CYCLES])
        .collect();
    median_count(&per_thousand).map_or_else(
        || "-".to_owned(),
        |ipc| format!("{}.{:02}", ipc / 1000, ipc % 1000 / 10),
    )
}

#[cfg(test)]
#[path = "counters_report_tests.rs"]
mod tests;
