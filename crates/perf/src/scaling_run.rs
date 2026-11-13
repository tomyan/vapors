//! `--scaling <change>`: the change at each size of [`SIZES`], counted
//! in instructions, tabled and judged.

use vapors_gpu_harness::Gpu;

use crate::instruction_samples::InstructionSamples;
use crate::instructions_check::{checked_counter, samples_counted};
use crate::report::EVERY_PHASE;
use crate::rows::{RowChange, Rows};
use crate::run::{Samples, Settings};
use crate::sampler::Sampler;
use crate::scaling::{AtSize, SIZES, Scaling};
use crate::scaling_report::{scaling_section, verdicts};
use crate::spans::PhaseClock;
use crate::stats::median_count;
use crate::workloads::settings_for;

/// Runs `change` at every size, prints a table per kind of frame and
/// the verdicts.
///
/// # Errors
/// Counts that cannot be trusted (no counter, a multiplexed reading, a
/// phase timed and not counted), a size missing a phase, or growth
/// beyond the tolerance in any kind of frame.
pub fn run(gpu: &Gpu, change: RowChange, settings: Settings) -> Result<(), String> {
    let mut timer = Sampler::with_clock(gpu, PhaseClock::with_instructions(checked_counter()?));
    let measured = SIZES
        .iter()
        .map(|&rows| measure(&mut timer, rows, change, settings))
        .collect::<Result<Vec<_>, _>>()?;
    let kinds = per_kind(change, measured)?;
    for (title, scaling) in &kinds {
        println!("{}", scaling_section(title, scaling));
    }
    println!("{}", verdicts(change, &kinds)?);
    Ok(())
}

/// One size's medians, one per kind of frame `change` makes.
fn measure(
    timer: &mut Sampler,
    rows: usize,
    change: RowChange,
    settings: Settings,
) -> Result<Vec<AtSize>, String> {
    let name = format!("{rows} rows, {change:?}");
    let workload = &mut Rows::new(rows, change);
    let settings = settings_for(Some(rows), settings);
    let samples = timer
        .sample(workload, settings)
        .map_err(|e| format!("{name}: {e}"))?;
    samples_counted(&name, &samples)?;
    by_kind(rows, &samples, change.frame_kinds().len(), settings.warmup)
}

/// The medians of each of `kinds` kinds of frame in `samples`, whose
/// first kept frame is frame `warmup`: frame `i` is of kind `i % kinds`.
///
/// # Errors
/// As [`at_size`], for any kind.
pub(crate) fn by_kind(
    rows: usize,
    samples: &Samples,
    kinds: usize,
    warmup: usize,
) -> Result<Vec<AtSize>, String> {
    let kept = kept_counts(rows, samples)?;
    let first_of = |kind: usize| (kind + kinds - warmup % kinds) % kinds;
    (0..kinds)
        .map(|kind| medians(rows, &kept.every_nth(first_of(kind), kinds), &EVERY_PHASE))
        .collect()
}

/// Each kind of frame's title and its series across sizes, from
/// `measured`, a list of kinds per size.
fn per_kind(
    change: RowChange,
    measured: Vec<Vec<AtSize>>,
) -> Result<Vec<(String, Scaling)>, String> {
    let kinds = change.frame_kinds();
    let mut by_kind: Vec<Vec<AtSize>> = kinds.iter().map(|_| Vec::new()).collect();
    for size in measured {
        by_kind
            .iter_mut()
            .zip(size)
            .for_each(|(sizes, at)| sizes.push(at));
    }
    kinds
        .iter()
        .zip(by_kind)
        .map(|(kind, sizes)| Ok((title(change, kind), Scaling::of(&sizes)?)))
        .collect()
}

/// `change`'s name, and the kind of frame when it makes more than one.
fn title(change: RowChange, kind: &str) -> String {
    if change.frame_kinds().len() == 1 {
        return format!("{change:?}");
    }
    format!("{change:?}, {kind} frames")
}

/// The median count of the frame and of each phase `samples` recorded.
///
/// # Errors
/// A recorded phase that is not in [`PHASES`]: the judge does not know it.
pub(crate) fn at_size(rows: usize, samples: &Samples, known: &[&str]) -> Result<AtSize, String> {
    medians(rows, kept_counts(rows, samples)?, known)
}

fn kept_counts(rows: usize, samples: &Samples) -> Result<&InstructionSamples, String> {
    samples
        .instructions
        .as_ref()
        .ok_or_else(|| format!("{rows} rows: no instruction counts were taken"))
}

fn medians(rows: usize, kept: &InstructionSamples, known: &[&str]) -> Result<AtSize, String> {
    if let Some(unknown) = kept.phases.keys().find(|p| !known.contains(&p.as_str())) {
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
