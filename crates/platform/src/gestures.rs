//! Touch gestures from raw touches: one finger pans, two fingers pinch
//! about their midpoint and pan by its movement, a quick still press is a
//! tap, and two taps close together are a double tap.
//!
//! Pure: fed each touch with a monotonic time, it returns the gestures
//! that touch completed. The window module supplies the clock.

use std::time::Duration;

use crate::event::{Event, TouchPhase};

// The numbers follow Android's ViewConfiguration where it has one:
// DOUBLE_TAP_TIMEOUT is 300 ms, touch slop 8 dp, double-tap slop 100 dp,
// long press 400 ms. Distances here are physical pixels with no density
// available yet, so the slops are chosen in pixels: 20 px is 8 dp at
// 2.5x, the density of most phones; 30 px for a double tap is far below
// Android's 100 dp because ours toggles the fit and a second finger
// landing somewhere else should be a new tap, not a toggle.

/// A press longer than this is not a tap; it is under Android's 400 ms
/// long press so a long press never counts.
const TAP_TIME: Duration = Duration::from_millis(300);
/// A press that moves further than this is not a tap, and a finger pans
/// only once it has moved further than this.
pub const TAP_SLOP: f32 = 20.0;
/// A second tap starting later than this after the first ended is a new tap.
const DOUBLE_TAP_TIME: Duration = Duration::from_millis(300);
/// A second tap further than this from the first is a new tap.
const DOUBLE_TAP_SLOP: f32 = 30.0;

struct Finger {
    id: u64,
    at: [f32; 2],
}

struct Press {
    id: u64,
    at: [f32; 2],
    started: Duration,
}

struct Tap {
    at: [f32; 2],
    ended: Duration,
}

/// The recogniser. One per window.
#[derive(Default)]
pub struct Gestures {
    fingers: Vec<Finger>,
    /// The single finger that may still become a tap.
    press: Option<Press>,
    /// The last tap, until it is too old to pair with.
    last_tap: Option<Tap>,
    /// A confused stream (a repeated Started, an unknown finger) is
    /// reported once, because it repeats on every event after.
    reported: bool,
}

impl Gestures {
    /// No fingers down.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one raw touch. `now` is any monotonic clock.
    pub fn touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        x: f32,
        y: f32,
        now: Duration,
    ) -> Vec<Event> {
        let at = [x, y];
        match phase {
            TouchPhase::Started => self.started(id, at, now),
            TouchPhase::Moved => self.moved(id, at),
            TouchPhase::Ended => self.ended(id, at, now),
            TouchPhase::Cancelled => self.cancelled(id),
        }
    }

    fn started(&mut self, id: u64, at: [f32; 2], now: Duration) -> Vec<Event> {
        if let Some(finger) = self.fingers.iter_mut().find(|f| f.id == id) {
            finger.at = at;
            self.inconsistent(&format!("touch {id} started while already down"));
        } else {
            self.fingers.push(Finger { id, at });
        }
        self.press = (self.fingers.len() == 1).then_some(Press {
            id,
            at,
            started: now,
        });
        Vec::new()
    }

    fn moved(&mut self, id: u64, at: [f32; 2]) -> Vec<Event> {
        let Some(index) = self.fingers.iter().position(|f| f.id == id) else {
            self.inconsistent(&format!("touch {id} moved without starting"));
            return Vec::new();
        };
        let before = std::mem::replace(&mut self.fingers[index].at, at);
        let pressed = self.press.as_ref().filter(|press| press.id == id);
        // within the tap slop a finger is still a possible tap and pans
        // nothing; past it the press is dropped and it pans, so a tap
        // and a pan are exclusive, as Chromium's slop makes them
        // (review-v78, H1)
        let within_slop = pressed.is_some_and(|press| distance(press.at, at) <= TAP_SLOP);
        if pressed.is_some() && !within_slop {
            self.press = None;
        }
        match self.fingers.len() {
            1 if within_slop => Vec::new(),
            1 => vec![pan(before, at)],
            2 => pinch(self.fingers[1 - index].at, before, at),
            _ => Vec::new(),
        }
    }

    fn ended(&mut self, id: u64, at: [f32; 2], now: Duration) -> Vec<Event> {
        if !self.fingers.iter().any(|f| f.id == id) {
            self.inconsistent(&format!("touch {id} ended without starting"));
            return Vec::new();
        }
        self.fingers.retain(|f| f.id != id);
        let Some(press) = self.press.take_if(|p| p.id == id) else {
            return Vec::new();
        };
        if now.saturating_sub(press.started) > TAP_TIME || distance(press.at, at) > TAP_SLOP {
            return Vec::new();
        }
        self.tapped(at, press.started, now)
    }

    fn tapped(&mut self, at: [f32; 2], started: Duration, now: Duration) -> Vec<Event> {
        let [x, y] = at;
        let mut events = vec![Event::Tap { x, y }];
        let pairs = self.last_tap.take().is_some_and(|last| {
            started.saturating_sub(last.ended) <= DOUBLE_TAP_TIME
                && distance(last.at, at) <= DOUBLE_TAP_SLOP
        });
        if pairs {
            events.push(Event::DoubleTap { x, y });
        } else {
            self.last_tap = Some(Tap { at, ended: now });
        }
        events
    }

    fn cancelled(&mut self, id: u64) -> Vec<Event> {
        self.fingers.retain(|f| f.id != id);
        self.press.take_if(|p| p.id == id);
        Vec::new()
    }

    fn inconsistent(&mut self, message: &str) {
        if !self.reported {
            self.reported = true;
            crate::report::report_error(&format!("touch stream: {message}; ignored"));
        }
    }
}

fn pan(before: [f32; 2], after: [f32; 2]) -> Event {
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
fn pinch(other: [f32; 2], before: [f32; 2], after: [f32; 2]) -> Vec<Event> {
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

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[cfg(test)]
#[path = "gestures_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gestures_tap_tests.rs"]
mod tap_tests;
