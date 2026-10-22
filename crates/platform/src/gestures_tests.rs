//! Scripts of raw touches and the gestures they must produce.

use std::time::Duration;

use super::Gestures;
use crate::event::{Event, TouchPhase};

use TouchPhase::{Cancelled, Ended, Moved, Started};

/// One raw touch: finger, phase, x, y, milliseconds since the start.
type Step = (u64, TouchPhase, f32, f32, u64);

fn play(steps: &[Step]) -> Vec<Event> {
    let mut gestures = Gestures::new();
    let mut out = Vec::new();
    for &(id, phase, x, y, ms) in steps {
        out.extend(gestures.touch(id, phase, x, y, Duration::from_millis(ms)));
    }
    out
}

/// Each script must produce exactly its listed events.
fn check(cases: &[(&str, &[Step], &[Event])]) {
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
            "one finger moving pans by its own movement",
            &[
                (0, Started, 0.0, 0.0, 0),
                (0, Moved, 10.0, 5.0, 16),
                (0, Moved, 13.0, 5.0, 32),
            ],
            &[
                Event::Pan { dx: 10.0, dy: 5.0 },
                Event::Pan { dx: 3.0, dy: 0.0 },
            ],
        ),
        (
            "two fingers pinch about their midpoint and pan by its movement",
            &[
                (0, Started, 0.0, 0.0, 0),
                (1, Started, 100.0, 0.0, 10),
                (1, Moved, 200.0, 0.0, 20),
            ],
            &[
                Event::Pinch {
                    x: 100.0,
                    y: 0.0,
                    factor: 2.0,
                },
                Event::Pan { dx: 50.0, dy: 0.0 },
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
fn taps_and_double_taps_depend_on_time_and_distance() {
    // Given
    let cases: &[(&str, &[Step], &[Event])] = &[
        (
            "a quick still press is a tap, not a double tap",
            &[(0, Started, 10.0, 10.0, 0), (0, Ended, 12.0, 11.0, 100)],
            &[Event::Tap { x: 12.0, y: 11.0 }],
        ),
        (
            "a second tap soon and close is a double tap",
            &[
                (0, Started, 10.0, 10.0, 0),
                (0, Ended, 12.0, 11.0, 100),
                (1, Started, 15.0, 12.0, 300),
                (1, Ended, 15.0, 12.0, 350),
            ],
            &[
                Event::Tap { x: 12.0, y: 11.0 },
                Event::Tap { x: 15.0, y: 12.0 },
                Event::DoubleTap { x: 15.0, y: 12.0 },
            ],
        ),
        (
            "a second tap too late is two single taps",
            &[
                (0, Started, 10.0, 10.0, 0),
                (0, Ended, 10.0, 10.0, 100),
                (1, Started, 10.0, 10.0, 500),
                (1, Ended, 10.0, 10.0, 550),
            ],
            &[
                Event::Tap { x: 10.0, y: 10.0 },
                Event::Tap { x: 10.0, y: 10.0 },
            ],
        ),
        (
            "a second tap too far away is two single taps",
            &[
                (0, Started, 10.0, 10.0, 0),
                (0, Ended, 10.0, 10.0, 100),
                (1, Started, 50.0, 10.0, 200),
                (1, Ended, 50.0, 10.0, 250),
            ],
            &[
                Event::Tap { x: 10.0, y: 10.0 },
                Event::Tap { x: 50.0, y: 10.0 },
            ],
        ),
    ];

    // When / Then
    check(cases);
}
#[test]
fn slow_moved_cancelled_and_two_finger_presses_are_not_taps() {
    // Given
    let cases: &[(&str, &[Step], &[Event])] = &[
        (
            "a slow press is not a tap",
            &[(0, Started, 0.0, 0.0, 0), (0, Ended, 0.0, 0.0, 400)],
            &[],
        ),
        (
            "a press that moved too far is a pan, not a tap",
            &[
                (0, Started, 0.0, 0.0, 0),
                (0, Moved, 30.0, 0.0, 50),
                (0, Ended, 30.0, 0.0, 100),
            ],
            &[Event::Pan { dx: 30.0, dy: 0.0 }],
        ),
        (
            "a cancelled touch produces nothing further",
            &[
                (0, Started, 0.0, 0.0, 0),
                (0, Cancelled, 0.0, 0.0, 10),
                (0, Moved, 10.0, 0.0, 20),
                (0, Ended, 10.0, 0.0, 30),
            ],
            &[],
        ),
        (
            "two fingers down and up is not a tap",
            &[
                (0, Started, 0.0, 0.0, 0),
                (1, Started, 10.0, 0.0, 10),
                (1, Ended, 10.0, 0.0, 50),
                (0, Ended, 0.0, 0.0, 60),
            ],
            &[],
        ),
    ];

    // When / Then
    check(cases);
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
