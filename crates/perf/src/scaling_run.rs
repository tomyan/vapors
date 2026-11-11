//! `--scaling <change>`: the change at each size of [`SIZES`], counted
//! in instructions, tabled and judged.

use std::collections::BTreeMap;

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
    let mut measured = Vec::new();
    for rows in SIZES {
        let name = format!("{rows} rows, {change:?}");
        let samples = timer
            .sample(
                &mut Rows::new(rows, change),
                settings_for(Some(rows), settings),
            )
            .map_err(|e| format!("{name}: {e}"))?;
        samples_counted(&name, &samples)?;
        measured.push(at_size(rows, &samples)?);
    }
    let scaling = Scaling::of(&measured)?;
    println!("{}", scaling_section(change, &scaling));
    println!("{}", verdict(change, &scaling)?);
    Ok(())
}

/// The median count of the frame and of each reported phase `samples`
/// entered.
fn at_size(rows: usize, samples: &Samples) -> Result<AtSize, String> {
    let kept = samples
        .instructions
        .as_ref()
        .ok_or_else(|| format!("{rows} rows: no instruction counts were taken"))?;
    let mut phases = BTreeMap::new();
    let named = std::iter::once(("frame (CPU)", &kept.frame)).chain(
        PHASES
            .iter()
            .filter_map(|&p| Some((p, kept.phases.get(p)?))),
    );
    for (phase, counts) in named {
        let median = median_count(counts)
            .ok_or_else(|| format!("{rows} rows: no samples of phase {phase}"))?;
        phases.insert(phase.to_owned(), median);
    }
    Ok(AtSize { rows, phases })
}
