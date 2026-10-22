//! Scripts of raw touches that are, or are not, taps and double taps.

use super::tests::{Step, check};
use crate::event::{Event, TouchPhase};

use TouchPhase::{Cancelled, Ended, Moved, Started};

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
