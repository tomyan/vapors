//! `--scaling <change>`: the change at each size of [`SIZES`], counted
//! in instructions, tabled and judged.

use vapors_gpu_harness::Gpu;

use crate::instructions_check::{checked_counter, samples_counted};
use crate::report::PHASES;
use crate::rows::{RowChange, Rows};
use crate::run::{Samples, Settings};
use crate::sampler::Sampler;
use crate::scaling::{AtSize, SIZES, Scaling};
use crate::scaling_report::{scaling_section, verdict};
use crate::spans::PhaseClock;
use crate::stats::median_count;
use crate::workloads::settings_for;

/// Runs `change` at every size, prints the table and the verdict.
///
/// # Errors
/// Counts that cannot be trusted (no counter, a multiplexed reading, a
/// phase timed and not counted), a size missing a phase, or growth
/// beyond the tolerance.
pub fn run(gpu: &Gpu, change: RowChange, settings: Settings) -> Result<(), String> {
    let mut timer = Sampler::with_clock(gpu, PhaseClock::with_instructions(checked_counter()?));
    let measured = SIZES
        .iter()
        .map(|&rows| measure(&mut timer, rows, change, settings))
        .collect::<Result<Vec<_>, _>>()?;
    let scaling = Scaling::of(&measured)?;
    println!("{}", scaling_section(change, &scaling));
    println!("{}", verdict(change, &scaling)?);
    Ok(())
}

fn measure(
    timer: &mut Sampler,
    rows: usize,
    change: RowChange,
    settings: Settings,
) -> Result<AtSize, String> {
    let name = format!("{rows} rows, {change:?}");
    let workload = &mut Rows::new(rows, change);
    let samples = timer
        .sample(workload, settings_for(Some(rows), settings))
        .map_err(|e| format!("{name}: {e}"))?;
    samples_counted(&name, &samples)?;
    at_size(rows, &samples)
}

/// The median count of the frame and of each phase `samples` recorded.
///
/// # Errors
/// A recorded phase that is not in [`PHASES`]: the judge does not know it.
fn at_size(rows: usize, samples: &Samples) -> Result<AtSize, String> {
    let kept = samples
        .instructions
        .as_ref()
        .ok_or_else(|| format!("{rows} rows: no instruction counts were taken"))?;
    if let Some(unknown) = kept.phases.keys().find(|p| !PHASES.contains(&p.as_str())) {
        return Err(unknown_phase(rows, unknown));
    }
    let named = std::iter::once(("frame (CPU)", &kept.frame))
        .chain(kept.phases.iter().map(|(p, counts)| (p.as_str(), counts)));
    let phases = named
        .map(|(phase, counts)| Ok((phase.to_owned(), median(rows, phase, counts)?)))
        .collect::<Result<_, String>>()?;
    Ok(AtSize { rows, phases })
}

fn median(rows: usize, phase: &str, counts: &[u64]) -> Result<u64, String> {
    median_count(counts).ok_or_else(|| format!("{rows} rows: no samples of phase {phase}"))
}

fn unknown_phase(rows: usize, phase: &str) -> String {
    format!(
        "{rows} rows: phase {phase} was recorded and is not in report::PHASES, so \
         --scaling cannot judge it; add it to PHASES in crates/perf/src/report.rs"
    )
}

#[cfg(test)]
#[path = "scaling_run_tests.rs"]
mod tests;
