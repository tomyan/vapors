//! Scripts of raw touches that are, or are not, taps and double taps.

use super::tests::{Step, check, play};
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

#[test]
fn a_tap_and_a_pan_are_exclusive() {
    // Given: a wobble inside the slop, and a drag past it (review-v78, H1:
    // a touch a scroll cancelled must not also click, as in Chromium)
    let wobble: &[Step] = &[
        (0, Started, 10.0, 60.0, 0),
        (0, Moved, 10.0, 50.0, 50),
        (0, Ended, 10.0, 50.0, 100),
    ];
    let drag: &[Step] = &[
        (0, Started, 10.0, 60.0, 0),
        (0, Moved, 10.0, 30.0, 50),
        (0, Ended, 10.0, 30.0, 100),
    ];

    // When / Then: the wobble taps and pans nothing; the drag pans and does not tap
    assert_eq!(play(wobble), [Event::Tap { x: 10.0, y: 50.0 }]);
    assert_eq!(play(drag), [Event::Pan { dx: 0.0, dy: -30.0 }]);
}
