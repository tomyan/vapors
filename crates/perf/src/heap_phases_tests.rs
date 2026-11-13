//! The heap meter on spans with a fake live-bytes count: each phase's
//! net bytes arrive under its name, nested phases each count their own,
//! and taking the totals empties them.

use std::cell::Cell;

use super::HeapPhases;

thread_local! {
    static LIVE: Cell<i64> = const { Cell::new(0) };
}

fn live() -> i64 {
    LIVE.with(Cell::get)
}

fn allocate(bytes: i64) {
    LIVE.with(|live| live.set(live.get() + bytes));
}

#[test]
fn a_phase_is_charged_the_bytes_it_left_live() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        let _span = tracing::trace_span!("update phase", phase = "style").entered();
        allocate(300);
        allocate(-100);
    });

    // Then
    assert_eq!(phases.take().get("style"), Some(&200));
}

#[test]
fn a_phase_that_frees_more_than_it_allocates_is_charged_a_negative_count() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        let _span = tracing::trace_span!("frame phase", phase = "draw").entered();
        allocate(-64);
    });

    // Then
    assert_eq!(phases.take().get("draw"), Some(&-64));
}

#[test]
fn a_nested_phase_counts_inside_its_parent_too() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        let _update = tracing::trace_span!("frame phase", phase = "update").entered();
        allocate(10);
        let _layout = tracing::trace_span!("update phase", phase = "layout").entered();
        allocate(32);
    });

    // Then
    let totals = phases.take();
    assert_eq!(
        (totals.get("update"), totals.get("layout")),
        (Some(&42), Some(&32))
    );
}

#[test]
fn a_draw_step_and_other_spans_are_not_metered() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        let _step = tracing::trace_span!("draw step", phase = "submit").entered();
        let _other = tracing::trace_span!("other", phase = "style").entered();
        allocate(8);
    });

    // Then
    assert!(phases.take().is_empty());
}

#[test]
fn taking_the_totals_empties_them() {
    // Given
    let phases = HeapPhases::new(live);
    tracing::subscriber::with_default(phases.clone(), || {
        let _span = tracing::trace_span!("update phase", phase = "paint").entered();
        allocate(1);
    });
    drop(phases.take());

    // When
    let again = phases.take();

    // Then
    assert!(again.is_empty());
}
