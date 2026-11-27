//! The calling thread's user-space hardware counters for `--counters`:
//! cycles, instructions, branch misses, and the kernel's generic L1d and
//! last-level cache miss events, read together as one perf event group
//! so their ratios (instructions per cycle) are over the same
//! instructions. They show how the code meets the core (pipelining,
//! branch prediction, caches), and they move with whatever else shares
//! the core: diagnostics for a quiet window, never judged.

use crate::instructions_check::whole_time;

/// The events, in the order of a [`Reading`], named for what they count:
/// the kernel's `L1-dcache-load-misses` and `LLC-load-misses`, whose
/// meaning per CPU the report's note gives (`counters_report.rs`).
pub const EVENTS: [&str; 5] = [
    "cycles",
    "instructions",
    "branch misses",
    "L1d misses",
    "LLC demand read misses",
];

/// One reading of every event, in [`EVENTS`] order.
pub type Reading = [u64; 5];

/// Where `cycles` is in a [`Reading`].
pub const CYCLES: usize = 0;
/// Where `instructions` is in a [`Reading`].
pub const INSTRUCTIONS: usize = 1;
/// Where `branch misses` is in a [`Reading`].
pub const BRANCH_MISSES: usize = 2;
/// Where the L1d misses are in a [`Reading`].
pub const L1D_MISSES: usize = 3;
/// Where the LLC demand read misses are in a [`Reading`].
pub const LLC_MISSES: usize = 4;

pub use crate::llc_line::llc_line_bytes;

/// A group's reading as perf gives it: the counts, and the nanoseconds
/// the group was enabled and running when the read format carries them.
pub(crate) type GroupRead = (Reading, Option<u64>, Option<u64>);

/// The group of counters, enabled.
pub struct HardwareCounters(imp::Counters);

impl HardwareCounters {
    /// Opens the group on the calling thread, enables it, and checks that
    /// work which must raise each event raises it
    /// (`hardware_counters_check.rs`); the LLC's line size must be
    /// readable, as the report needs it.
    ///
    /// # Errors
    /// Why the group cannot be opened, naming the fix; an unreadable line
    /// size; or an event that does not count. In a test tier other than
    /// pmu, the refusal of `counter_tier::allowed`.
    pub fn open() -> Result<Self, String> {
        crate::counter_tier::allowed()?;
        llc_line_bytes()?;
        let mut counters = imp::open().map(Self).map_err(|why| refusal(&why))?;
        check::every_event_counts(&mut counters)?;
        Ok(counters)
    }

    /// Every event's count since [`Self::open`].
    ///
    /// # Errors
    /// The read's failure, or a reading [`checked`] refuses.
    pub fn read(&mut self) -> Result<Reading, String> {
        checked(imp::read(&mut self.0)?)
    }
}

/// The counts of a group that was on the PMU for the whole time it was
/// enabled: a multiplexed reading, short by an unknown amount, is
/// refused, and so is a read that does not say, since a missing time is
/// not a zero.
///
/// # Errors
/// Either time missing, or the two different.
pub(crate) fn checked((reading, enabled, running): GroupRead) -> Result<Reading, String> {
    let (Some(enabled), Some(running)) = (enabled, running) else {
        return Err(format!(
            "the counter group's read carries no time {} (enabled {enabled:?}, running \
             {running:?}), so a multiplexed reading cannot be told from a whole one; fix: keep \
             TOTAL_TIME_ENABLED and TOTAL_TIME_RUNNING in the group's read format \
             (crates/perf/src/hardware_counters_linux.rs)",
            if enabled.is_none() {
                "enabled"
            } else {
                "running"
            }
        ));
    };
    whole_time(enabled, running)?;
    Ok(reading)
}

pub(crate) fn refusal(why: &str) -> String {
    format!(
        "--counters needs Linux perf events for {}: {why}. {}",
        EVENTS.join(", "),
        imp::FIX
    )
}

#[path = "hardware_counters_check.rs"]
mod check;

#[cfg(target_os = "linux")]
#[path = "hardware_counters_linux.rs"]
mod imp;

#[cfg(not(target_os = "linux"))]
mod imp {
    pub const FIX: &str = "They exist only on Linux; run --counters there";

    pub struct Counters;

    pub fn open() -> Result<Counters, String> {
        Err("this is not Linux".to_owned())
    }

    pub fn read(_: &mut Counters) -> Result<super::GroupRead, String> {
        Err("this is not Linux".to_owned())
    }
}

#[cfg(test)]
#[path = "hardware_counters_tests.rs"]
mod tests;
