//! Taps in a run: each tap's count, and the double tap that is the
//! second of a run.

use std::time::Duration;

use super::{DOUBLE_TAP_SLOP, DOUBLE_TAP_TIME, Gestures, distance};
use crate::event::Event;

/// The last tap, until it is too old to pair with.
pub(super) struct Tap {
    at: [f32; 2],
    ended: Duration,
    /// Its place in the run: 1, 2 or 3, then 1 again, as Chromium
    /// counts taps (the elements corpus's `clicks.events`: of five
    /// quick taps, the 2nd and the 5th send `dblclick`).
    count: u8,
}

/// The longest run of taps before the count starts again at one.
const TAP_RUN: u8 = 3;

impl Gestures {
    /// A tap at `at`, and a double tap when it is the second of a run.
    pub(super) fn tapped(&mut self, at: [f32; 2], started: Duration, now: Duration) -> Vec<Event> {
        let [x, y] = at;
        let count = self.tap_count(at, started);
        self.last_tap = Some(Tap {
            at,
            ended: now,
            count,
        });
        let mut events = vec![Event::Tap { x, y }];
        if count == 2 {
            events.push(Event::DoubleTap { x, y });
        }
        events
    }

    /// The count of a tap at `at` that started at `started`: one more
    /// than the last tap's when it follows soon and close, else 1.
    fn tap_count(&self, at: [f32; 2], started: Duration) -> u8 {
        match &self.last_tap {
            Some(last)
                if started.saturating_sub(last.ended) <= DOUBLE_TAP_TIME
                    && distance(last.at, at) <= DOUBLE_TAP_SLOP =>
            {
                last.count % TAP_RUN + 1
            }
            _ => 1,
        }
    }
}
