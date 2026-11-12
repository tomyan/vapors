//! Checks that an instruction count can be trusted: the counter ran the
//! whole time, rises with work done, and covers every phase that was
//! timed. Only [`self_check`] needs the hardware; the rest are pure, so
//! the unit tier runs them anywhere.

use std::collections::BTreeMap;
use std::hint::black_box;

use crate::instructions::InstructionCounter;

/// Iterations of the self-check's loop. Each retires at least one
/// instruction, since `black_box` keeps the loop from being folded.
const LOOP_ITERATIONS: u64 = 100_000;

/// Refuses a reading whose counter was scheduled off the PMU for part of
/// the time it was enabled: the count is short by an unknown amount.
///
/// # Errors
/// `time_running` differs from `time_enabled` (both in nanoseconds).
pub fn whole_time(time_enabled: u64, time_running: u64) -> Result<(), String> {
    if time_enabled == time_running {
        return Ok(());
    }
    Err(format!(
        "the counter was not running for the whole time it was enabled (time_enabled \
         {time_enabled} ns, time_running {time_running} ns); stop other perf users \
         (perf stat, the NMI watchdog: sysctl kernel.nmi_watchdog=0)"
    ))
}

/// Refuses a count that rose by less than the work done must retire.
///
/// # Errors
/// `after - before` is below `at_least`.
pub fn rose_by_at_least(before: u64, after: u64, at_least: u64) -> Result<(), String> {
    let rose = after.saturating_sub(before);
    if rose >= at_least {
        return Ok(());
    }
    Err(format!(
        "the instruction counter rose by {rose} over a loop of at least {at_least} \
         instructions: it is not counting this thread's user-space instructions \
         (no PMU in this VM, or perf events restricted: check kernel.perf_event_paranoid)"
    ))
}

/// Checks a counter just opened: a loop the optimiser cannot fold must
/// raise the count by at least its iterations.
///
/// # Errors
/// A failed read, or a count that did not rise enough.
pub fn self_check(counter: &mut InstructionCounter) -> Result<(), String> {
    let before = counter.read()?;
    for i in 0..LOOP_ITERATIONS {
        black_box(i);
    }
    let after = counter.read()?;
    rose_by_at_least(before, after, LOOP_ITERATIONS)
}

/// Refuses timed phases that have no instruction count.
///
/// # Errors
/// Names the phases of `timed` that `counted` lacks.
pub fn phases_counted<T, U>(
    timed: &BTreeMap<String, T>,
    counted: &BTreeMap<String, U>,
) -> Result<(), String> {
    let missing: Vec<&str> = timed
        .keys()
        .filter(|phase| !counted.contains_key(*phase))
        .map(String::as_str)
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "phases timed but not counted in instructions: {}",
        missing.join(", ")
    ))
}

#[cfg(test)]
#[path = "instructions_check_tests.rs"]
mod tests;
