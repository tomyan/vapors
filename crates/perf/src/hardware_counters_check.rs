//! The check `HardwareCounters::open` runs: work that must raise each
//! event raises it. An event that opens but does not count (a VM that
//! exposes the PMU but not `OFFCORE_RESPONSE`, or one that filters event
//! codes) would read 0 in every phase, which a reader cannot tell from
//! a phase that missed nothing, so it is refused, naming the event.

use std::hint::black_box;

use super::{
    BRANCH_MISSES, CYCLES, EVENTS, HardwareCounters, INSTRUCTIONS, L1D_MISSES, LLC_MISSES, Reading,
};

/// Iterations of the plain loop, as `instructions_check`'s.
const LOOP_ITERATIONS: u64 = 100_000;
/// Branches on random bits: about half mispredict.
const RANDOM_BRANCHES: u64 = 100_000;
/// The chase's buffer: 64 MiB, past any last-level cache this harness
/// meets, walked in a random cycle of its 64-byte lines so neither the
/// caches nor the prefetchers hold the next line.
const CHASE_LINES: usize = (64 << 20) / 64;
/// `u64`s in a 64-byte line.
const LINE_WORDS: usize = 8;

/// Each event rose over work that must raise it.
///
/// # Errors
/// A failed read, or an event that rose by less than its work needs,
/// named.
pub(super) fn every_event_counts(counters: &mut HardwareCounters) -> Result<(), String> {
    let (before, after) = around(counters, plain_loop)?;
    rose(INSTRUCTIONS, before, after, LOOP_ITERATIONS)?;
    rose(CYCLES, before, after, LOOP_ITERATIONS / 4)?;
    let (before, after) = around(counters, random_branches)?;
    rose(BRANCH_MISSES, before, after, RANDOM_BRANCHES / 8)?;
    let chase = chain();
    let (before, after) = around(counters, || walk(&chase))?;
    let lines = CHASE_LINES as u64;
    rose(L1D_MISSES, before, after, lines / 2)?;
    rose(LLC_MISSES, before, after, lines / 8)
}

fn around(
    counters: &mut HardwareCounters,
    work: impl FnOnce() -> u64,
) -> Result<(Reading, Reading), String> {
    let before = counters.read()?;
    black_box(work());
    Ok((before, counters.read()?))
}

/// Refuses `event` when it rose by less than `at_least`.
pub(super) fn rose(
    event: usize,
    before: Reading,
    after: Reading,
    at_least: u64,
) -> Result<(), String> {
    let rose = after[event].saturating_sub(before[event]);
    if rose >= at_least {
        return Ok(());
    }
    Err(format!(
        "--counters: {} rose by {rose} over work that must raise it by at least {at_least}: the \
         event opens but does not count on this CPU or VM, so its column would read as measured \
         zeros; run --counters on a machine whose PMU counts it (bare metal), or use \
         --instructions",
        EVENTS[event]
    ))
}

fn plain_loop() -> u64 {
    (0..LOOP_ITERATIONS).map(black_box).sum()
}

/// Branches on xorshift bits; `black_box` in the taken arm keeps it a
/// branch rather than a conditional move.
fn random_branches() -> u64 {
    let mut bits = 0x9E37_79B9_7F4A_7C15_u64;
    let mut taken = 0;
    for _ in 0..RANDOM_BRANCHES {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        if black_box(bits) & 1 == 1 {
            taken = black_box(taken + 1);
        }
    }
    taken
}

/// The buffer, each line's first word the index of the next line in one
/// random cycle through every line (Sattolo's shuffle).
fn chain() -> Vec<u64> {
    let mut order: Vec<usize> = (0..CHASE_LINES).collect();
    let mut bits = 0x2545_F491_4F6C_DD1D_u64;
    for last in (1..CHASE_LINES).rev() {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        let pick = usize::try_from(bits % last as u64).expect("below a usize");
        order.swap(last, pick);
    }
    let mut buffer = vec![0u64; CHASE_LINES * LINE_WORDS];
    for (at, &line) in order.iter().enumerate() {
        buffer[line * LINE_WORDS] = order[(at + 1) % CHASE_LINES] as u64;
    }
    buffer
}

/// Every line once, each load's address the last load's value.
fn walk(buffer: &[u64]) -> u64 {
    let mut line = 0u64;
    for _ in 0..CHASE_LINES {
        let at = usize::try_from(line).expect("an index") * LINE_WORDS;
        line = black_box(buffer[at]);
    }
    line
}

#[cfg(test)]
#[path = "hardware_counters_check_tests.rs"]
mod tests;
