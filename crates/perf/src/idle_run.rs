//! The idle measurement at each size of `--scaling Idle` (`idle.rs`):
//! the rows page left alone for [`INTERVAL`] of the app's time, tabled
//! and judged after the instruction counts. Every size is measured
//! before any refusal is reported, so one size's failure does not hide
//! the others'.

use crate::idle::{INTERVAL, IdleCounts, section, verdict};
use crate::rows::{RowChange, Rows};
use crate::run::Settings;
use crate::sampler::Sampler;
use crate::scaling::SIZES;

/// Measures the rows page idle at every size, prints the table and
/// the verdict. `settings` are the run's own, for the reproduce command.
///
/// # Errors
/// Every size that could not be measured, named; or a size where the
/// app asked for a frame or did an update, each named with the fix and
/// the command.
pub fn run(sampler: &mut Sampler, settings: Settings) -> Result<(), String> {
    let reproduce = format!(
        "vapors-perf {} {} --scaling Idle",
        settings.runs, settings.warmup
    );
    let (measured, refused): (Vec<_>, Vec<_>) = SIZES
        .iter()
        .map(|&rows| (rows, measure(sampler, rows, &reproduce)))
        .partition(|(_, result)| result.is_ok());
    if !refused.is_empty() {
        let each: Vec<String> = refused
            .into_iter()
            .filter_map(|(_, result)| result.err())
            .collect();
        return Err(each.join("\n"));
    }
    let measured: Vec<(usize, IdleCounts)> = measured
        .into_iter()
        .filter_map(|(rows, result)| result.ok().map(|counts| (rows, counts)))
        .collect();
    println!("{}", section(INTERVAL, &measured));
    println!("{}", verdict(INTERVAL, &measured, &reproduce)?);
    Ok(())
}

fn measure(sampler: &mut Sampler, rows: usize, reproduce: &str) -> Result<IdleCounts, String> {
    let workload = &mut Rows::new(rows, RowChange::Idle);
    sampler
        .idle(workload, INTERVAL, reproduce)
        .map_err(|e| format!("{rows} rows: {e}"))
}
