//! Scripts of raw touches and the flings they end in.

use super::tests::{Step, play};
use crate::event::{Event, TouchPhase};

use TouchPhase::{Cancelled, Ended, Moved, Started};

/// The flings in what `steps` produce.
fn flings(steps: &[Step]) -> Vec<[f32; 2]> {
    play(steps)
        .into_iter()
        .filter_map(|event| match event {
            Event::Fling { vx, vy } => Some([vx, vy]),
            _ => None,
        })
        .collect()
}

/// Finger 0 moving down at 1000 px/s, a move every 8 ms from 0 to 80 ms.
fn drag_down() -> Vec<Step> {
    let mut steps = vec![(0, Started, 100.0, 0.0, 0)];
    #[allow(clippy::cast_precision_loss, reason = "a few milliseconds")]
    steps.extend((1..=10).map(|n| (0, Moved, 100.0, 8.0 * n as f32, 8 * n)));
    steps
}

#[test]
fn a_lone_finger_lifting_after_a_pan_flings_at_its_velocity() {
    // Given
    let mut steps = drag_down();

    // When
    steps.push((0, Ended, 100.0, 80.0, 88));

    // Then
    let found = flings(&steps);
    assert_eq!(found.len(), 1, "{found:?}");
    let [vx, vy] = found[0];
    assert!(vx.abs() < 0.01 && (vy - 1000.0).abs() < 1.0, "{vx}, {vy}");
}

#[test]
fn a_finger_that_stopped_80_ms_before_lifting_does_not_fling() {
    // Given
    let mut steps = drag_down();

    // When
    steps.push((0, Ended, 100.0, 80.0, 160));

    // Then
    assert_eq!(flings(&steps), Vec::<[f32; 2]>::new());
}

#[test]
fn a_finger_lifting_within_the_tap_slop_does_not_fling() {
    // Given a quick 15 px flick, inside the 20 px slop
    let steps: &[Step] = &[
        (0, Started, 0.0, 0.0, 0),
        (0, Moved, 0.0, 15.0, 8),
        (0, Ended, 0.0, 15.0, 16),
    ];

    // Then
    assert_eq!(flings(steps), Vec::<[f32; 2]>::new());
}

#[test]
fn a_gesture_with_a_second_finger_does_not_fling() {
    // Given a second finger landing and dragging down too
    let mut steps = drag_down();
    steps.push((1, Started, 200.0, 0.0, 82));
    for (n, y) in [(1, 8.0), (2, 16.0), (3, 24.0)] {
        steps.push((1, Moved, 200.0, y, 82 + 8 * n));
    }

    // When both lift, the second first
    steps.push((1, Ended, 200.0, 24.0, 108));
    steps.push((0, Ended, 100.0, 80.0, 110));

    // Then
    assert_eq!(flings(&steps), Vec::<[f32; 2]>::new());
}

#[test]
fn a_cancelled_finger_does_not_fling() {
    // Given
    let mut steps = drag_down();

    // When
    steps.push((0, Cancelled, 100.0, 80.0, 88));

    // Then
    assert_eq!(flings(&steps), Vec::<[f32; 2]>::new());
}
