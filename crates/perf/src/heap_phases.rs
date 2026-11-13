//! A `tracing` subscriber that meters the heap of a frame's phases
//! (finish-line property 8, memory per element): the live bytes each
//! `frame phase` and `update phase` span left behind, allocated less
//! freed, summed by phase name between its enter and exit as
//! `PhaseClock` sums times. A nested phase counts in its parent too.
//!
//! Its own maps are made with room before any span, so its bookkeeping
//! does not allocate inside a metered phase; what a map grows past that
//! room is charged to the phase it grew in.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};

use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

use crate::spans::PhaseField;

/// The spans it meters, by name.
const METERED: [&str; 2] = ["frame phase", "update phase"];

/// Room for the spans of one frame, made before the first.
const ROOM: usize = 64;

/// The subscriber; clones share the totals.
#[derive(Clone)]
pub struct HeapPhases(Arc<Mutex<Meter>>);

struct Meter {
    live: fn() -> i64,
    next_id: u64,
    phases: HashMap<u64, String>,
    entered: HashMap<u64, i64>,
    totals: BTreeMap<String, i64>,
}

impl HeapPhases {
    /// A meter reading the process's live heap bytes from `live`.
    #[must_use]
    pub fn new(live: fn() -> i64) -> Self {
        Self(Arc::new(Mutex::new(Meter {
            live,
            next_id: 0,
            phases: HashMap::with_capacity(ROOM),
            entered: HashMap::with_capacity(ROOM),
            totals: BTreeMap::new(),
        })))
    }

    /// Each phase's net live bytes since the last take, by phase name.
    #[must_use]
    pub fn take(&self) -> BTreeMap<String, i64> {
        std::mem::take(&mut self.meter().totals)
    }

    fn meter(&self) -> MutexGuard<'_, Meter> {
        self.0.lock().expect("a heap meter panicked while metering")
    }
}

impl Subscriber for HeapPhases {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.is_span() && METERED.contains(&metadata.name())
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut phase = PhaseField(String::new());
        span.record(&mut phase);
        let mut meter = self.meter();
        meter.next_id += 1;
        let id = meter.next_id;
        meter.phases.insert(id, phase.0);
        Id::from_u64(id)
    }

    fn enter(&self, span: &Id) {
        let mut meter = self.meter();
        let now = (meter.live)();
        meter.entered.insert(span.into_u64(), now);
    }

    fn exit(&self, span: &Id) {
        let mut guard = self.meter();
        let meter = &mut *guard;
        let now = (meter.live)();
        let id = span.into_u64();
        let (Some(entered), Some(phase)) = (meter.entered.remove(&id), meter.phases.get(&id))
        else {
            return;
        };
        let phase = phase.clone();
        *meter.totals.entry(phase).or_default() += now - entered;
    }

    fn try_close(&self, span: Id) -> bool {
        self.meter().phases.remove(&span.into_u64());
        true
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
}

#[cfg(test)]
#[path = "heap_phases_tests.rs"]
mod tests;
