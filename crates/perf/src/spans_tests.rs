//! The phase clock on a real update: vapors-elements' spans arrive
//! under their phase names, and taking the times empties them.

use super::PhaseClock;
use crate::rows::rows_document;

fn names(clock: &PhaseClock) -> Vec<String> {
    clock.take().into_keys().collect()
}

#[test]
fn an_update_is_timed_as_its_style_layout_and_paint_phases() {
    // Given
    let clock = PhaseClock::default();
    let (document, viewport) = rows_document(10);

    // When
    tracing::subscriber::with_default(clock.clone(), || {
        document.borrow_mut().update(viewport).expect("updates");
    });

    // Then
    assert_eq!(names(&clock), ["layout", "paint", "style"]);
    assert_eq!(
        names(&clock),
        Vec::<String>::new(),
        "take left times behind"
    );
}

#[test]
fn a_span_outside_the_frame_and_update_phases_is_not_timed() {
    // Given
    let clock = PhaseClock::default();

    // When
    tracing::subscriber::with_default(clock.clone(), || {
        let _span = tracing::trace_span!("other", phase = "style").entered();
    });

    // Then
    assert_eq!(names(&clock), Vec::<String>::new());
}

#[test]
fn a_draw_step_is_timed_under_its_phase() {
    // Given
    let clock = PhaseClock::default();

    // When
    tracing::subscriber::with_default(clock.clone(), || {
        let _span = tracing::trace_span!("draw step", phase = "submit").entered();
    });

    // Then
    assert_eq!(names(&clock), ["submit"]);
}

#[test]
fn a_clock_without_a_counter_has_no_instruction_counts() {
    assert_eq!(PhaseClock::default().take_instructions(), Ok(None));
}
