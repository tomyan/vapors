//! Hardware counts kept per frame: phases aligned by name, and every
//! nth frame picked, with no perf events.

use std::collections::BTreeMap;

use super::CounterSamples;

fn reading(at: u64) -> [u64; 5] {
    [at, at * 2, at * 3, at * 4, at * 5]
}

#[test]
fn a_phase_a_frame_did_not_enter_has_zeros_for_it() {
    // Given: style only in the second frame
    let mut kept = CounterSamples::default();

    // When
    kept.push(
        reading(1),
        BTreeMap::from([("update".to_owned(), reading(1))]),
    );
    kept.push(
        reading(2),
        BTreeMap::from([("style".to_owned(), reading(2))]),
    );

    // Then
    assert_eq!(kept.phases["style"], [[0; 5], reading(2)]);
    assert_eq!(kept.phases["update"], [reading(1), [0; 5]]);
}

#[test]
fn every_nth_picks_the_frames_and_their_phases_from_the_first() {
    // Given
    let mut kept = CounterSamples::default();
    for at in 0..5 {
        kept.push(
            reading(at),
            BTreeMap::from([("update".to_owned(), reading(at))]),
        );
    }

    // When
    let picked = kept.every_nth(1, 2);

    // Then
    assert_eq!(picked.frame, [reading(1), reading(3)]);
    assert_eq!(picked.phases["update"], [reading(1), reading(3)]);
}
