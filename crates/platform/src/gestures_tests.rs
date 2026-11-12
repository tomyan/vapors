//! Scripts of raw touches and the gestures they must produce: pans,
//! pinches and the finger bookkeeping. Taps are in `gestures_tap_tests.rs`.

use std::time::Duration;

use super::Gestures;
use crate::event::{Event, TouchPhase};

use TouchPhase::{Ended, Moved, Started};

/// One raw touch: finger, phase, x, y, milliseconds since the start.
pub(super) type Step = (u64, TouchPhase, f32, f32, u64);

pub(super) fn play(steps: &[Step]) -> Vec<Event> {
    let mut gestures = Gestures::new();
    let mut out = Vec::new();
    for &(id, phase, x, y, ms) in steps {
        out.extend(gestures.touch(id, phase, x, y, Duration::from_millis(ms)));
    }
    out
}

/// Each script must produce exactly its listed events.
pub(super) fn check(cases: &[(&str, &[Step], &[Event])]) {
    for (name, script, expected) in cases {
        // When
        let events = play(script);

        // Then
        assert_eq!(&events, expected, "{name}");
    }
}

#[test]
fn pans_and_pinches_follow_the_fingers() {
    // Given
    let cases: &[(&str, &[Step], &[Event])] = &[
        (
            "one finger past the tap slop pans by its own movement",
            &[
                (0, Started, 0.0, 0.0, 0),
                (0, Moved, 25.0, 5.0, 16),
                (0, Moved, 28.0, 5.0, 32),
            ],
            &[
                Event::Pan { dx: 25.0, dy: 5.0 },
                Event::Pan { dx: 3.0, dy: 0.0 },
            ],
        ),
        (
            "one finger within the tap slop pans nothing",
            &[(0, Started, 0.0, 0.0, 0), (0, Moved, 10.0, 5.0, 16)],
            &[],
        ),
        (
            "two fingers pinch about their midpoint and pan by its movement",
            &[
                (0, Started, 0.0, 0.0, 0),
                (1, Started, 100.0, 0.0, 10),
                (1, Moved, 200.0, 0.0, 20),
            ],
            &[
                Event::Pan { dx: 50.0, dy: 0.0 },
                Event::Pinch {
                    x: 100.0,
                    y: 0.0,
                    factor: 2.0,
                },
            ],
        ),
        (
            "a second finger lifting goes back to pan without a jump",
            &[
                (0, Started, 0.0, 0.0, 0),
                (1, Started, 100.0, 0.0, 10),
                (1, Ended, 100.0, 0.0, 20),
                (0, Moved, 10.0, 0.0, 30),
            ],
            &[Event::Pan { dx: 10.0, dy: 0.0 }],
        ),
    ];

    // When / Then
    check(cases);
}
#[test]
fn a_repeated_started_replaces_the_finger_instead_of_adding_one() {
    // Given: the same id starts twice, as a confused stream can send
    let script: &[Step] = &[
        (0, Started, 0.0, 0.0, 0),
        (0, Started, 50.0, 0.0, 10),
        (0, Moved, 60.0, 0.0, 20),
    ];

    // When
    let events = play(script);

    // Then: within the tap slop of the second start, so no pan, and no
    // phantom pinch
    assert_eq!(events, []);
}

#[test]
fn moved_and_ended_for_an_unknown_finger_are_ignored() {
    // Given
    let script: &[Step] = &[(9, Moved, 10.0, 0.0, 0), (9, Ended, 10.0, 0.0, 10)];

    // When
    let events = play(script);

    // Then
    assert_eq!(events, []);
}

#[test]
fn moving_a_third_finger_produces_nothing() {
    // Given
    let script: &[Step] = &[
        (0, Started, 0.0, 0.0, 0),
        (1, Started, 100.0, 0.0, 0),
        (2, Started, 50.0, 50.0, 0),
        (2, Moved, 60.0, 60.0, 10),
    ];

    // When
    let events = play(script);

    // Then
    assert_eq!(events, []);
}
