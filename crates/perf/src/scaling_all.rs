//! `--scaling` over several changes: each judged as `--scaling <change>`
//! judges it, every one run whichever fail, and one verdict naming each
//! change that failed (`cargo xtask scaling` runs `--scaling all`).

use vapors_gpu_harness::Gpu;

use crate::rows::RowChange;
use crate::run::Settings;
use crate::scaling_run;

/// Runs every change of `changes`, prints each table and verdict, and
/// the lines `scripts/nightly-scaling.sh` reads: [`changes_line`] first,
/// then after each change its [`error_block`] when it failed and its
/// [`result_line`]. `scripts/fixtures/nightly-scaling.log` pins them.
///
/// # Errors
/// One or more changes failed: each failure's message, which names the
/// change, its phase, its growth per extra row and the command that
/// reproduces it.
pub fn run(gpu: &Gpu, changes: &[RowChange], settings: Settings) -> Result<(), String> {
    println!("{}", changes_line(changes));
    let failures: Vec<String> = changes
        .iter()
        .filter_map(|&change| {
            let result = scaling_run::run(gpu, change, settings);
            if let Err(error) = &result {
                println!("{}", error_block(change, error));
            }
            println!("{}", result_line(change, result.is_ok()));
            result.err()
        })
        .collect();
    verdict(changes.len(), &failures)
}

/// `scaling-changes: <change> ...`: every change this run judges, so a
/// change with no [`result_line`] is known to be unjudged.
#[must_use]
pub fn changes_line(changes: &[RowChange]) -> String {
    let names: Vec<String> = changes.iter().map(|change| format!("{change:?}")).collect();
    format!("scaling-changes: {}", names.join(" "))
}

/// `error`, why `change` failed, between `scaling-error-begin: <change>`
/// and `scaling-error-end: <change>`, whatever the error's wording.
#[must_use]
pub fn error_block(change: RowChange, error: &str) -> String {
    format!("scaling-error-begin: {change:?}\n{error}\nscaling-error-end: {change:?}")
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
