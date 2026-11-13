//! A `tracing` subscriber that meters the heap of a frame's phases
//! (finish-line property 8, memory per element): the live bytes each
//! `frame phase` and `update phase` span changed by, allocated less
//! freed, summed by phase name between its enter and exit as
//! `PhaseClock` sums times. A nested phase counts in its parent too.
//!
//! The live count is the process's, and the subscriber is set for one
//! thread: today's measuring run is single-threaded; a phase on another
//! thread would not be metered, and another thread's allocations would
//! be charged to the open phase. Its own maps are made with room before
//! any span; the totals' entries and tracing's first registration of a
//! callsite still allocate, a few hundred bytes in all, charged to the
//! phase open then.
//!
//! What it cannot place it keeps as a failure that `take` returns: a
//! metered span with no `phase` field, a span exiting that never
//! entered, and a phase name used by both span kinds.

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

/// A span's phase and the handles to it still open.
struct Span {
    phase: String,
    handles: usize,
}

struct Meter {
    live: fn() -> i64,
    next_id: u64,
    spans: HashMap<u64, Span>,
    entered: HashMap<u64, i64>,
    totals: BTreeMap<String, i64>,
    kinds: HashMap<String, &'static str>,
    failure: Option<String>,
}

impl HeapPhases {
    /// A meter reading the process's live heap bytes from `live`.
    #[must_use]
    pub fn new(live: fn() -> i64) -> Self {
        Self(Arc::new(Mutex::new(Meter {
            live,
            next_id: 0,
            spans: HashMap::with_capacity(ROOM),
            entered: HashMap::with_capacity(ROOM),
            totals: BTreeMap::new(),
            kinds: HashMap::with_capacity(ROOM),
            failure: None,
        })))
    }

    /// Each phase's net live bytes since the last take, by phase name.
    ///
    /// # Errors
    /// The first span it could not place since the last take, naming it.
    pub fn take(&self) -> Result<BTreeMap<String, i64>, String> {
        let mut meter = self.meter();
        let totals = std::mem::take(&mut meter.totals);
        meter.failure.take().map_or(Ok(totals), Err)
    }

    fn meter(&self) -> MutexGuard<'_, Meter> {
        self.0.lock().expect("a heap meter panicked while metering")
    }
}

impl Meter {
    fn fail(&mut self, failure: String) {
        self.failure.get_or_insert(failure);
    }

    /// Refuses a phase name another span kind has used.
    fn claim(&mut self, phase: &str, kind: &'static str) {
        let held = *self.kinds.entry(phase.to_owned()).or_insert(kind);
        if held != kind {
            self.fail(format!(
                "phase {phase:?} is both a {held:?} and a {kind:?} span; their bytes would merge"
            ));
        }
    }
}

impl Subscriber for HeapPhases {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.is_span() && METERED.contains(&metadata.name())
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut phase = PhaseField(String::new());
        span.record(&mut phase);
        let kind = span.metadata().name();
        let mut meter = self.meter();
        if phase.0.is_empty() {
            meter.fail(format!("a {kind:?} span has no phase field"));
        }
        meter.claim(&phase.0, kind);
        meter.next_id += 1;
        let id = meter.next_id;
        let entry = Span {
            phase: phase.0,
            handles: 1,
        };
        meter.spans.insert(id, entry);
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
        let (Some(entered), Some(open)) = (meter.entered.remove(&id), meter.spans.get(&id)) else {
            meter.fail(format!(
                "span {id} exited without an enter or after its close"
            ));
            return;
        };
        let phase = open.phase.clone();
        *meter.totals.entry(phase).or_default() += now - entered;
    }

    fn clone_span(&self, span: &Id) -> Id {
        if let Some(open) = self.meter().spans.get_mut(&span.into_u64()) {
            open.handles += 1;
        }
        span.clone()
    }

    fn try_close(&self, span: Id) -> bool {
        let mut meter = self.meter();
        let id = span.into_u64();
        let Some(open) = meter.spans.get_mut(&id) else {
            return false;
        };
        open.handles -= 1;
        let closed = open.handles == 0;
        if closed {
            meter.spans.remove(&id);
        }
        closed
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
}

#[cfg(test)]
#[path = "heap_phases_tests.rs"]
mod tests;
