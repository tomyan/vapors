//! A frame's counts: its rise with each phase's counts, a `write` phase
//! of zero added when the frame entered none, as the times are.

use std::collections::BTreeMap;

use vapors_renderer::WRITE_PHASE;

use super::*;

fn style_only<T>(value: T) -> Option<BTreeMap<String, T>> {
    Some(BTreeMap::from([("style".to_owned(), value)]))
}

#[test]
fn a_frame_that_entered_no_write_phase_has_one_of_zero_for_each_counter() {
    // Given
    let rise = Rise {
        instructions: Some(100),
        counters: Some(Reading::default()),
    };

    // When
    let counted = rise.with_phases((style_only(40), style_only(Reading::default())));

    // Then
    let (frame, phases) = counted.instructions.expect("instructions");
    assert_eq!((frame, phases.get(WRITE_PHASE)), (100, Some(&0)));
    let (_, counters) = counted.counters.expect("counters");
    assert_eq!(counters.get(WRITE_PHASE), Some(&Reading::default()));
}

#[test]
fn a_clock_without_counters_gives_no_counts() {
    // Given
    let rise = Rise {
        instructions: None,
        counters: None,
    };

    // When
    let counted = rise.with_phases((None, None));

    // Then
    assert!(counted.instructions.is_none() && counted.counters.is_none());
}
