//! The perf event group behind `HardwareCounters`, on Linux.

use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Hardware};
use perf_event::{Builder, Counter, Group};

use super::Reading;

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
    let mut members = Vec::new();
    for (mut builder, name) in builders().into_iter().zip(super::EVENTS) {
        let member = group
            .add(builder.exclude_kernel(true).exclude_hv(true))
            .map_err(|e| format!("perf_event_open of {name} failed: {e}"))?;
        members.push(member);
    }
    group
        .enable()
        .map_err(|e| format!("enabling the group failed: {e}"))?;
    Ok(Counters { group, members })
}

/// The reading, and the nanoseconds the group was enabled and running.
pub fn read(counters: &mut Counters) -> Result<(Reading, u64, u64), String> {
    let data = counters
        .group
        .read()
        .map_err(|e| format!("reading the group failed: {e}"))?;
    let nanos = |time: Option<std::time::Duration>| {
        time.map_or(0, |t| u64::try_from(t.as_nanos()).unwrap_or(u64::MAX))
    };
    let mut reading = Reading::default();
    for (slot, member) in reading.iter_mut().zip(&counters.members) {
        *slot = data[member];
    }
    Ok((
        reading,
        nanos(data.time_enabled()),
        nanos(data.time_running()),
    ))
}
