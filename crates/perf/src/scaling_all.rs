//! `--scaling` over several changes: each judged as `--scaling <change>`
//! judges it, every one run whichever fail, and one verdict naming each
//! change that failed (`cargo xtask scaling` runs `--scaling all`).

use vapors_gpu_harness::Gpu;

use crate::rows::RowChange;
use crate::run::Settings;
use crate::scaling_run;

/// Runs every change of `changes`, prints each table and verdict, and
/// after each its [`result_line`].
///
/// # Errors
/// One or more changes failed: each failure's message, which names the
/// change, its phase, its growth per extra row and the command that
/// reproduces it.
pub fn run(gpu: &Gpu, changes: &[RowChange], settings: Settings) -> Result<(), String> {
    let failures: Vec<String> = changes
        .iter()
        .filter_map(|&change| {
            let result = scaling_run::run(gpu, change, settings);
            println!("{}", result_line(change, result.is_ok()));
            result.err()
        })
        .collect();
    verdict(changes.len(), &failures)
}

/// `scaling-result: <change> pass|fail`, the line
/// `scripts/nightly-scaling.sh` reads to tell which changes failed.
#[must_use]
pub fn result_line(change: RowChange, passed: bool) -> String {
    let result = if passed { "pass" } else { "fail" };
    format!("scaling-result: {change:?} {result}")
}

/// Passes when nothing failed; one failure of one change is returned as
/// it is, so `--scaling <change>` reads as it did before `all`.
///
/// # Errors
/// Any failure: the count and every message, in the order run.
pub fn verdict(judged: usize, failures: &[String]) -> Result<(), String> {
    match failures {
        [] => Ok(()),
        [only] if judged == 1 => Err(only.clone()),
        _ => Err(format!(
            "scaling: {} of {judged} changes failed:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        )),
    }
}

#[cfg(test)]
#[path = "scaling_all_tests.rs"]
mod tests;
