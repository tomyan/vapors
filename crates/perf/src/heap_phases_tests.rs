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

fn totals(phases: &HeapPhases) -> std::collections::BTreeMap<String, i64> {
    phases.take().expect("every span placed")
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
    assert_eq!(totals(&phases).get("style"), Some(&200));
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
    assert_eq!(totals(&phases).get("draw"), Some(&-64));
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
    let totals = totals(&phases);
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
    assert!(totals(&phases).is_empty());
}

#[test]
fn taking_the_totals_empties_them() {
    // Given
    let phases = HeapPhases::new(live);
    tracing::subscriber::with_default(phases.clone(), || {
        let _span = tracing::trace_span!("update phase", phase = "paint").entered();
        allocate(1);
    });
    drop(totals(&phases));

    // When
    let again = totals(&phases);

    // Then
    assert!(again.is_empty());
}

#[test]
fn bytes_left_by_one_phase_and_freed_by_the_next_are_charged_to_each() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        {
            let _layout = tracing::trace_span!("update phase", phase = "layout").entered();
            allocate(500);
        }
        let _paint = tracing::trace_span!("update phase", phase = "paint").entered();
        allocate(80);
        allocate(-200);
    });

    // Then
    let totals = totals(&phases);
    assert_eq!((totals["layout"], totals["paint"]), (500, -120));
}

#[test]
fn sibling_phases_are_each_charged_their_own_bytes() {
    // Given
    let phases = HeapPhases::new(live);

    // When
    tracing::subscriber::with_default(phases.clone(), || {
        for (phase, bytes) in [("accessibility", 7), ("draw", 900), ("record", 3)] {
            let _span = tracing::trace_span!("frame phase", phase).entered();
            allocate(bytes);
        }
    });

    // Then
    let totals = totals(&phases);
    let charged: Vec<i64> = ["accessibility", "draw", "record"]
        .map(|name| totals[name])
        .to_vec();
    assert_eq!(charged, [7, 900, 3]);
}

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
fn a_cloned_span_is_metered_until_its_last_handle_closes() {
    // Given
    let phases = HeapPhases::new(live);

    // When: the first handle is dropped before the clone enters
    tracing::subscriber::with_default(phases.clone(), || {
        let first = tracing::trace_span!("frame phase", phase = "tasks");
        let clone = first.clone();
        drop(first);
        let _entered = clone.entered();
        allocate(64);
    });

    // Then
    assert_eq!(totals(&phases).get("tasks"), Some(&64));
}
