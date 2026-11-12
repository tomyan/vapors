//! The frame's `tracing` spans (review-v75, M1): one per phase, in
//! order, each named `frame phase` and carrying the phase's name, as a
//! reader of the spans (area 10's recorder) finds them.

use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};
use vapors_platform::Instant;

use super::*;

/// Records each new span's name and `phase` field.
#[derive(Clone, Default)]
struct Spans(Arc<Mutex<Vec<(String, String)>>>);

struct PhaseField(String);

impl Visit for PhaseField {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "phase" {
            self.0 = value.to_owned();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "phase" {
            self.0 = format!("{value:?}");
        }
    }
}

impl Subscriber for Spans {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut phase = PhaseField(String::new());
        span.record(&mut phase);
        let mut spans = self.0.lock().expect("not poisoned");
        spans.push((span.metadata().name().to_owned(), phase.0));
        Id::from_u64(spans.len() as u64)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

/// Phases that do nothing.
struct Idle;

impl Phases for Idle {
    fn update(&mut self, _: Instant) -> Result<(), String> {
        Ok(())
    }
    fn draw(&mut self, _: Instant) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn a_frame_opens_one_span_per_phase_named_for_it() {
    // Given
    let spans = Spans::default();
    let recorded = spans.clone();

    // When
    tracing::subscriber::with_default(spans, || {
        run_frame(&mut Idle, Instant::now(), &SystemClock).expect("the frame runs");
    });

    // Then
    let expected: Vec<(String, String)> = [
        "input",
        "tasks",
        "tick",
        "update",
        "accessibility",
        "draw",
        "record",
    ]
    .iter()
    .map(|phase| ("frame phase".to_owned(), (*phase).to_owned()))
    .collect();
    assert_eq!(*recorded.0.lock().expect("not poisoned"), expected);
}

#[test]
fn every_phase_is_in_all_at_its_own_index() {
    // Then: FrameTimes indexes by `phase as usize`
    for (index, phase) in Phase::ALL.iter().enumerate() {
        assert_eq!(*phase as usize, index, "{phase:?}");
    }
}
