//! The heap meter refuses what it cannot place: a span with no phase,
//! a phase name under both span kinds, an exit without an enter, and a
//! second enter before the exit.

use super::HeapPhases;
use super::tests::{allocate, live};

#[test]
fn a_metered_span_without_a_phase_is_refused() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        let _span = tracing::trace_span!("update phase").entered();
        allocate(4);
    });

    // Then
    let refused = phases.take().expect_err("no phase field");
    assert!(refused.contains("no phase field"), "{refused}");
}

#[test]
fn one_phase_name_under_both_span_kinds_is_refused_naming_it() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        drop(tracing::trace_span!("frame phase", phase = "paint").entered());
        drop(tracing::trace_span!("update phase", phase = "paint").entered());
    });

    // Then
    let refused = phases.take().expect_err("paint is both kinds");
    assert!(refused.contains("\"paint\""), "{refused}");
}

#[test]
fn a_span_exiting_without_an_enter_is_refused() {
    // Given
    let phases = HeapPhases::new(live);
    let id = tracing::subscriber::with_default(phases.clone(), || {
        tracing::trace_span!("update phase", phase = "style")
            .id()
            .expect("metered")
    });

    // When
    tracing::Subscriber::exit(&phases, &id);

    // Then
    let refused = phases.take().expect_err("exited without an enter");
    assert!(refused.contains("exited without an enter"), "{refused}");
}

#[test]
fn a_span_entered_twice_before_it_exits_is_refused() {
    // Given
    let phases = HeapPhases::new(live);
    let span = tracing::subscriber::with_default(phases.clone(), || {
        tracing::trace_span!("frame phase", phase = "draw")
    });
    let id = span.id().expect("metered");

    // When
    tracing::Subscriber::enter(&phases, &id);
    tracing::Subscriber::enter(&phases, &id);

    // Then
    let refused = phases.take().expect_err("entered twice");
    assert!(refused.contains("entered again"), "{refused}");
}
