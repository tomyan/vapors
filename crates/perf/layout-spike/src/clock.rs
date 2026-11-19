//! The time vapors-elements spends in its `layout` phase of an update
//! (`update phase` spans, as vapors-perf's `PhaseClock` reads them),
//! apart from style and paint. A subscriber that sums that one phase,
//! and refuses a run in which it saw none.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[derive(Default)]
struct State {
    next: u64,
    layout_spans: HashSet<u64>,
    entered: Option<Instant>,
    /// Layout spans entered and exited.
    seen: usize,
    total: Duration,
}

#[derive(Clone, Default)]
struct LayoutClock(Arc<Mutex<State>>);

struct IsLayout(bool);

impl Visit for IsLayout {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0 |= field.name() == "phase" && value == "layout";
    }

    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

impl Subscriber for LayoutClock {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.is_span() && metadata.name() == "update phase"
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut layout = IsLayout(false);
        span.record(&mut layout);
        let mut state = self.0.lock().expect("clock");
        state.next += 1;
        let id = state.next;
        if layout.0 {
            state.layout_spans.insert(id);
        }
        Id::from_u64(id)
    }

    fn enter(&self, span: &Id) {
        let mut state = self.0.lock().expect("clock");
        if state.layout_spans.contains(&span.into_u64()) {
            state.entered = Some(Instant::now());
        }
    }

    fn exit(&self, span: &Id) {
        let mut state = self.0.lock().expect("clock");
        if let (true, Some(start)) = (
            state.layout_spans.contains(&span.into_u64()),
            state.entered.take(),
        ) {
            state.total += start.elapsed();
            state.seen += 1;
        }
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, _: &Event<'_>) {}
}

/// The clock saw no layout span in the work it timed.
#[derive(Debug, PartialEq, Eq)]
pub struct NoLayoutSpan;

impl std::fmt::Display for NoLayoutSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the layout-phase clock saw no `update phase` span with phase = \"layout\", so its \
             time would be a false 0: vapors-elements renamed or stopped emitting the span, \
             emits `phase` other than as a str, or compiled tracing out; fix src/clock.rs to \
             match vapors-elements/src/update.rs, then reproduce: cargo test --offline \
             --test clock"
        )
    }
}

/// The time `work` spends in `update phase` spans named `layout`.
///
/// # Errors
/// `work` entered no such span.
///
/// # Panics
/// A clock lock poisoned by a panic in `work`.
pub fn layout_phase(work: impl FnOnce()) -> Result<Duration, NoLayoutSpan> {
    let clock = LayoutClock::default();
    tracing::subscriber::with_default(clock.clone(), || {
        // a callsite first hit with no subscriber (another thread, or an
        // untimed layout before this one) keeps "never" until rebuilt
        tracing::callsite::rebuild_interest_cache();
        work();
    });
    let state = clock.0.lock().expect("clock");
    if state.seen == 0 {
        return Err(NoLayoutSpan);
    }
    Ok(state.total)
}
