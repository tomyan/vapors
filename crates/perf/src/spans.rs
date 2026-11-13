//! A `tracing` subscriber that times the phases of a frame: vapors-app's
//! `frame phase` spans (input to record; `update` and `draw` do the
//! work), vapors-elements' `update phase` spans (`style`, `layout`,
//! `paint`), and the `draw step` spans vapors-renderer, vapors-text and
//! vapors-app open inside draw (`batches`, `encode`, `write`,
//! `submit`), each named by its `phase` field. It sums the time between each span's
//! enter and exit by phase until the times are taken.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

use crate::instructions::InstructionCounter;
use crate::span_instructions::InstructionMeter;

/// The spans it times, by name.
const TIMED: [&str; 3] = ["frame phase", "update phase", "draw step"];

/// The subscriber; clones share the times.
#[derive(Clone, Default)]
pub struct PhaseClock(Arc<Mutex<Spans>>);

#[derive(Default)]
struct Spans {
    next_id: u64,
    phases: HashMap<u64, String>,
    entered: HashMap<u64, Instant>,
    totals: BTreeMap<String, Duration>,
    meter: Option<InstructionMeter>,
}

impl PhaseClock {
    /// A clock that also counts each phase's instructions with `counter`.
    #[must_use]
    pub fn with_instructions(counter: InstructionCounter) -> Self {
        let spans = Spans {
            meter: Some(InstructionMeter::new(counter)),
            ..Spans::default()
        };
        Self(Arc::new(Mutex::new(spans)))
    }

    /// Each phase's instructions since the last take, by phase name;
    /// `None` when the clock has no counter.
    ///
    /// # Errors
    /// A counter read that failed since the last take.
    pub fn take_instructions(&self) -> Result<Option<BTreeMap<String, u64>>, String> {
        self.spans()
            .meter
            .as_mut()
            .map(InstructionMeter::take)
            .transpose()
    }

    /// The counter now; `None` when the clock has no counter.
    ///
    /// # Errors
    /// The read's failure.
    pub fn instructions_now(&self) -> Result<Option<u64>, String> {
        self.spans()
            .meter
            .as_mut()
            .map(InstructionMeter::reading)
            .transpose()
    }

    /// Each phase's time since the last take, by phase name; the times
    /// start again from none.
    #[must_use]
    pub fn take(&self) -> BTreeMap<String, Duration> {
        std::mem::take(&mut self.spans().totals)
    }

    fn spans(&self) -> MutexGuard<'_, Spans> {
        self.0.lock().expect("a phase clock panicked while timing")
    }
}

/// Reads a span's `phase` field.
struct PhaseField(String);

impl Visit for PhaseField {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "phase" {
            value.clone_into(&mut self.0);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "phase" {
            self.0 = format!("{value:?}");
        }
    }
}

impl Subscriber for PhaseClock {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.is_span() && TIMED.contains(&metadata.name())
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut phase = PhaseField(String::new());
        span.record(&mut phase);
        let mut spans = self.spans();
        spans.next_id += 1;
        let id = spans.next_id;
        spans.phases.insert(id, phase.0);
        Id::from_u64(id)
    }

    fn enter(&self, span: &Id) {
        let mut spans = self.spans();
        if let Some(meter) = spans.meter.as_mut() {
            meter.enter(span.into_u64());
        }
        spans.entered.insert(span.into_u64(), Instant::now());
    }

    fn exit(&self, span: &Id) {
        let exited = Instant::now();
        let mut guard = self.spans();
        let spans = &mut *guard;
        let id = span.into_u64();
        if let (Some(meter), Some(phase)) = (spans.meter.as_mut(), spans.phases.get(&id)) {
            meter.exit(id, phase);
        }
        let (Some(entered), Some(phase)) = (spans.entered.remove(&id), spans.phases.get(&id))
        else {
            return;
        };
        let phase = phase.clone();
        *spans.totals.entry(phase).or_default() += exited - entered;
    }

    fn try_close(&self, span: Id) -> bool {
        self.spans().phases.remove(&span.into_u64());
        true
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
}

#[cfg(test)]
#[path = "spans_tests.rs"]
mod tests;
