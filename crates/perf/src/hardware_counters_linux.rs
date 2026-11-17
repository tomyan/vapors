//! The perf event group behind `HardwareCounters`, on Linux.

use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Hardware};
use perf_event::{Builder, Counter, Group};

use super::{GroupRead, Reading};

/// How to allow the events, for the open refusal.
pub const FIX: &str = "Allow them with `sudo sysctl kernel.perf_event_paranoid=1` (set again \
     after a reboot); if it is already 1 or lower, this CPU or VM lacks one of the events";

/// The group and its members, in `EVENTS` order.
pub struct Counters {
    group: Group,
    members: Vec<Counter>,
}

/// A builder for each event of `EVENTS`, in order.
fn builders() -> [Builder<'static>; 5] {
    let read_miss = |which| Cache {
        which,
        operation: CacheOp::READ,
        result: CacheResult::MISS,
    };
    [
        Builder::new(Hardware::CPU_CYCLES),
        Builder::new(Hardware::INSTRUCTIONS),
        Builder::new(Hardware::BRANCH_MISSES),
        Builder::new(read_miss(CacheId::L1D)),
        Builder::new(read_miss(CacheId::LL)),
    ]
}

pub fn open() -> Result<Counters, String> {
    let mut group = Group::builder()
        .exclude_kernel(true)
        .exclude_hv(true)
        .build_group()
        .map_err(|e| format!("perf_event_open of the group failed: {e}"))?;
    let members = members(&mut group)?;
    group
        .enable()
        .map_err(|e| format!("enabling the group failed: {e}"))?;
    Ok(Counters { group, members })
}

fn members(group: &mut Group) -> Result<Vec<Counter>, String> {
    let named = builders().into_iter().zip(super::EVENTS);
    named
        .map(|(mut builder, name)| {
            group
                .add(builder.exclude_kernel(true).exclude_hv(true))
                .map_err(|e| format!("perf_event_open of {name} failed: {e}"))
        })
        .collect()
}

/// The reading, and the nanoseconds the group was enabled and running,
/// as the read gives them.
pub fn read(counters: &mut Counters) -> Result<GroupRead, String> {
    let data = counters
        .group
        .read()
        .map_err(|e| format!("reading the group failed: {e}"))?;
    let mut reading = Reading::default();
    for (slot, member) in reading.iter_mut().zip(&counters.members) {
        *slot = data[member];
    }
    Ok((
        reading,
        data.time_enabled().map(nanos),
        data.time_running().map(nanos),
    ))
}

fn nanos(time: std::time::Duration) -> u64 {
    u64::try_from(time.as_nanos()).unwrap_or(u64::MAX)
}
