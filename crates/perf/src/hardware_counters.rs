//! The calling thread's user-space hardware counters for `--counters`:
//! cycles, instructions, branch misses, L1d read misses and last-level
//! cache read misses, read together as one perf event group so their
//! ratios (instructions per cycle) are over the same instructions.
//! They show how the code meets the core (pipelining, branch
//! prediction, caches), and they move with whatever else shares the
//! core: diagnostics for a quiet window, never judged.

use crate::instructions_check::{rose_by_at_least, whole_time};

/// The events, in the order of a [`Reading`].
pub const EVENTS: [&str; 5] = [
    "cycles",
    "instructions",
    "branch misses",
    "L1d read misses",
    "LLC read misses",
];

/// One reading of every event, in [`EVENTS`] order.
pub type Reading = [u64; 5];

/// Where `cycles` is in a [`Reading`].
pub const CYCLES: usize = 0;
/// Where `instructions` is in a [`Reading`].
pub const INSTRUCTIONS: usize = 1;

/// Bytes a last-level cache miss brings in: one 64-byte line.
pub const LINE_BYTES: u64 = 64;

/// Iterations of the self-check's loop, as `instructions_check`'s.
const LOOP_ITERATIONS: u64 = 100_000;

/// The group of counters, enabled.
pub struct HardwareCounters(imp::Counters);

impl HardwareCounters {
    /// Opens the group on the calling thread, enables it and checks that
    /// a loop raises its instructions and cycles.
    ///
    /// # Errors
    /// Why the group cannot be opened, naming the sysctl that allows it;
    /// or a reading that fails its checks.
    pub fn open() -> Result<Self, String> {
        let mut counters = imp::open().map(Self).map_err(|why| refusal(&why))?;
        let before = counters.read()?;
        for i in 0..LOOP_ITERATIONS {
            std::hint::black_box(i);
        }
        let after = counters.read()?;
        rose_by_at_least(before[INSTRUCTIONS], after[INSTRUCTIONS], LOOP_ITERATIONS)?;
        rose_by_at_least(before[CYCLES], after[CYCLES], LOOP_ITERATIONS / 4)?;
        Ok(counters)
    }

    /// Every event's count since [`Self::open`].
    ///
    /// # Errors
    /// The read's failure, or a group that was not on the PMU for the
    /// whole time it was enabled.
    pub fn read(&mut self) -> Result<Reading, String> {
        let (reading, enabled, running) = imp::read(&mut self.0)?;
        whole_time(enabled, running)?;
        Ok(reading)
    }
}

pub(crate) fn refusal(why: &str) -> String {
    format!(
        "--counters needs Linux perf events for {}: {why}. Allow them with \
         `sudo sysctl kernel.perf_event_paranoid=1` (set again after a reboot); if it is already \
         1 or lower, this CPU or VM lacks one of the events",
        EVENTS.join(", ")
    )
}

#[cfg(target_os = "linux")]
#[path = "hardware_counters_linux.rs"]
mod imp;

#[cfg(not(target_os = "linux"))]
mod imp {
    pub struct Counters;

    pub fn open() -> Result<Counters, String> {
        Err("this is not Linux".to_owned())
    }

    pub fn read(_: &mut Counters) -> Result<(super::Reading, u64, u64), String> {
        Err("this is not Linux".to_owned())
    }
}

#[cfg(test)]
#[path = "hardware_counters_tests.rs"]
mod tests;
