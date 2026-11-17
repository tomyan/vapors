//! `--counters`' report: per phase, the median cycles, instructions per
//! cycle, branch misses, L1d misses, last-level cache demand read
//! misses, and an estimate of the bytes those bring in. Diagnostics for
//! a quiet window: the counts move with whatever else shares the core
//! and its caches, so nothing judges them.

use std::fmt::Write;

use crate::counter_samples::CounterSamples;
use crate::hardware_counters::{
    BRANCH_MISSES, CYCLES, INSTRUCTIONS, L1D_MISSES, LLC_MISSES, Reading, llc_line_bytes,
};
use crate::report::PHASES;
use crate::run::Samples;
use crate::stats::median_count;

/// Printed under every counters section.
pub const QUIET_WINDOW: &str = "Hardware counts are diagnostics for a quiet window: they move with \
     what else runs on the core and its caches, and nothing judges them. L1d misses is the \
     kernel's L1-dcache-load-misses event: on Intel cores from Skylake on (as on the dev box) it \
     counts every line filled into L1d, loads, store RFOs and L1 prefetches alike; on other CPUs \
     it may count load misses only. LLC demand read misses is the kernel's LLC-load-misses: on \
     Intel, demand data reads only (no stores, instruction fetches or prefetches). LLC miss bytes \
     is an estimate: those misses times the LLC's line size from sysfs. IPC is the median of each \
     frame's own ratio over the frames that ran a cycle in the phase, so it can sit beside a \
     cycles median of about 0.";

const HEADER: &str = "| phase | cycles | IPC | branch misses | L1d misses | LLC demand read misses | LLC miss bytes (estimate) |\n\
                      |---|---|---|---|---|---|---|\n";

/// `samples`' hardware counts as a Markdown section.
///
/// # Errors
/// `samples` holds no hardware counts, or the LLC's line size cannot be
/// read (`llc_line.rs`).
pub fn counters_section(name: &str, samples: &Samples) -> Result<String, String> {
    counters_section_lined(name, samples, llc_line_bytes()?)
}

/// [`counters_section`] with the LLC's line size given.
///
/// # Errors
/// `samples` holds no hardware counts.
pub fn counters_section_lined(
    name: &str,
    samples: &Samples,
    line_bytes: u64,
) -> Result<String, String> {
    let kept = samples
        .counters
        .as_ref()
        .ok_or_else(|| format!("{name}: no hardware counts were taken"))?;
    let mut out = format!("### {name}\n\n{HEADER}");
    counter_row(&mut out, ("frame (CPU)", line_bytes), &kept.frame);
    for phase in PHASES {
        counter_row(&mut out, (phase, line_bytes), phase_readings(kept, phase));
    }
    let _ = writeln!(out, "\n{QUIET_WINDOW}");
    Ok(out)
}

fn phase_readings<'a>(kept: &'a CounterSamples, phase: &str) -> &'a [Reading] {
    kept.phases.get(phase).map_or(&[][..], Vec::as_slice)
}

fn counter_row(out: &mut String, (phase, line_bytes): (&str, u64), readings: &[Reading]) {
    let median = |event: usize| {
        let counts: Vec<u64> = readings.iter().map(|reading| reading[event]).collect();
        median_count(&counts)
    };
    let Some(cycles) = median(CYCLES) else {
        let _ = writeln!(out, "| {phase} | no samples | | | | | |");
        return;
    };
    let [branch, l1d, llc] =
        [BRANCH_MISSES, L1D_MISSES, LLC_MISSES].map(|event| median(event).unwrap_or(0));
    let _ = writeln!(
        out,
        "| {phase} | {cycles} | {} | {branch} | {l1d} | {llc} | {} |",
        ipc(readings),
        llc * line_bytes
    );
}

/// The median of each frame's instructions per cycle, to two places,
/// over the frames that ran a cycle in the phase (the cycles median
/// keeps every frame); `-` when none did.
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
