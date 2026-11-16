//! The pan and pinch a finger's movement makes (`gestures.rs`).

use crate::event::Event;

pub(super) fn pan(before: [f32; 2], after: [f32; 2]) -> Event {
    Event::Pan {
        dx: after[0] - before[0],
        dy: after[1] - before[1],
    }
}

/// The pan and pinch from one finger moving while `other` stays put.
/// The pan comes first and the pinch is anchored at the new midpoint:
/// applied in that order, the image point under the midpoint stays put.
/// (Pinching first about the new midpoint and then panning would move
/// it by (factor - 1) times the midpoint's movement every event.)
pub(super) fn pinch(other: [f32; 2], before: [f32; 2], after: [f32; 2]) -> Vec<Event> {
    let spread_before = distance(other, before);
    let spread_after = distance(other, after);
    // Coincident fingers have no spread to compare; factor 1.0 is the
    // right answer, not a division by zero.
    let factor = if spread_before > 0.0 {
        spread_after / spread_before
    } else {
        1.0
    };
    let mid_before = midpoint(other, before);
    let mid_after = midpoint(other, after);
    vec![
        pan(mid_before, mid_after),
        Event::Pinch {
            x: mid_after[0],
            y: mid_after[1],
            factor,
        },
    ]
}

fn midpoint(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0].midpoint(b[0]), a[1].midpoint(b[1])]
}

pub(super) fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
