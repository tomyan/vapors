//! The idle measurement at each size of `--scaling Idle` (`idle.rs`):
//! the rows page left alone for [`INTERVAL`] of the app's time, tabled
//! and judged after the instruction counts.

use crate::idle::{INTERVAL, IdleCounts, section, verdict};
use crate::rows::{RowChange, Rows};
use crate::sampler::Sampler;
use crate::scaling::SIZES;

/// Measures the rows page idle at every size, prints the table and
/// the verdict.
///
/// # Errors
/// A size that could not be measured, or a size where the app asked
/// for a frame or did an update, each named with the fix and the
/// command.
pub fn run(sampler: &mut Sampler) -> Result<(), String> {
    let measured = SIZES
        .iter()
        .map(|&rows| Ok((rows, measure(sampler, rows)?)))
        .collect::<Result<Vec<_>, String>>()?;
    println!("{}", section(INTERVAL, &measured));
    println!("{}", verdict(INTERVAL, &measured)?);
    Ok(())
}

fn measure(sampler: &mut Sampler, rows: usize) -> Result<IdleCounts, String> {
    let workload = &mut Rows::new(rows, RowChange::Idle);
    sampler
        .idle(workload, INTERVAL)
        .map_err(|e| format!("{rows} rows: {e}"))
}
