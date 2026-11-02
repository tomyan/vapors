//! Taps in a run: each tap's count (1, 2, 3, then 1 again, for a
//! document's `dblclick`), and the photo's double tap, which pairs taps:
//! every second quick tap.

use std::time::Duration;

use super::{DOUBLE_TAP_SLOP, DOUBLE_TAP_TIME, Gestures, distance};
use crate::event::Event;

/// The last tap, until it is too old to pair with.
pub(super) struct Tap {
    at: [f32; 2],
    ended: Duration,
    /// Its place in the run: 1, 2 or 3, then 1 again, as desktop
    /// headless Chromium 155 counts CDP touch taps (`clicks.events`).
    count: u8,
    /// It was the second of a pair, a double tap: the next tap starts a
    /// new pair.
    paired: bool,
}

/// The longest run of taps before the count starts again at one.
const TAP_RUN: u8 = 3;

impl Gestures {
    /// A tap at `at` with its count, and a double tap when it is the
    /// second of a pair.
    pub(super) fn tapped(&mut self, at: [f32; 2], started: Duration, now: Duration) -> Vec<Event> {
        let [x, y] = at;
        let follows = self.last_tap.as_ref().filter(|last| {
            started.saturating_sub(last.ended) <= DOUBLE_TAP_TIME
                && distance(last.at, at) <= DOUBLE_TAP_SLOP
        });
        let count = follows.map_or(1, |last| last.count % TAP_RUN + 1);
        let paired = follows.is_some_and(|last| !last.paired);
        self.last_tap = Some(Tap {
            at,
            ended: now,
            count,
            paired,
        });
        let clicks =
            std::num::NonZeroU32::new(u32::from(count)).unwrap_or(std::num::NonZeroU32::MIN);
        let mut events = vec![Event::Tap { x, y, clicks }];
        if paired {
            events.push(Event::DoubleTap { x, y });
        }
        events
    }
}
