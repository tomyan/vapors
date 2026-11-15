//! Touch gestures from raw touches: one finger pans, two fingers pinch
//! about their midpoint and pan by its movement, a quick still press is a
//! tap, the second of a run of taps close together is a double tap, and
//! a lone finger lifting after it panned flings at its velocity.
//!
//! Pure: fed each touch with a monotonic time since its epoch, it
//! returns the gestures that touch completed. The window module supplies
//! the clock.

use std::time::Duration;

use crate::clock::Instant;
use crate::event::{Event, TouchPhase};
use crate::velocity_tracker::VelocityTracker;

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

/// The recogniser. One per window.
pub struct Gestures {
    /// The instant the touch times count from.
    epoch: Instant,
    fingers: Vec<Finger>,
    /// The single finger that may still become a tap.
    press: Option<Press>,
    /// The last tap, until it is too old to pair with.
    last_tap: Option<tap::Tap>,
    /// The gesture's finger, while it is the only one there has been:
    /// only it flings. RELAXED: a gesture that had a second finger does
    /// not fling, where Chromium's `GestureDetector` flings at `UP` at
    /// the last finger's own velocity, its history kept through the
    /// second finger's down, unless a finger lifted moving against
    /// another (`POINTER_UP`'s dot product clears the tracker); retired
    /// when a fixture records a two-finger fling.
    alone: Option<u64>,
    /// Its velocity.
    tracker: VelocityTracker,
    /// Lifts whose velocity could not be measured (`gestures_fling.rs`).
    unmeasured: u32,
    /// A confused stream (a repeated Started, an unknown finger) is
    /// reported once, because it repeats on every event after.
    reported: bool,
}

impl Default for Gestures {
    fn default() -> Self {
        Self::starting(Instant::now())
    }
}

impl Gestures {
    /// No fingers down; touch times count from now.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// No fingers down; touch times count from `epoch`, which a fling's
    /// time ([`Event::Fling`]) is on.
    #[must_use]
    pub fn starting(epoch: Instant) -> Self {
        Self {
            epoch,
            fingers: Vec::new(),
            press: None,
            last_tap: None,
            alone: None,
            tracker: VelocityTracker::default(),
            unmeasured: 0,
            reported: false,
        }
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
            TouchPhase::Moved => self.moved(id, at, now),
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
        self.alone = (self.fingers.len() == 1).then_some(id);
        if self.alone.is_some() {
            self.tracker.down(at, now);
        }
        self.press = (self.fingers.len() == 1).then_some(Press {
            id,
            at,
            started: now,
        });
        Vec::new()
    }

    fn moved(&mut self, id: u64, at: [f32; 2], now: Duration) -> Vec<Event> {
        let Some(index) = self.fingers.iter().position(|f| f.id == id) else {
            self.inconsistent(&format!("touch {id} moved without starting"));
            return Vec::new();
        };
        if self.alone == Some(id) {
            self.tracker.moved(at, now);
        }
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
            return self.fling(id, now);
        };
        if now.saturating_sub(press.started) > TAP_TIME || distance(press.at, at) > TAP_SLOP {
            return Vec::new();
        }
        self.tapped(at, press.started, now)
    }

    fn cancelled(&mut self, id: u64) -> Vec<Event> {
        self.alone.take_if(|alone| *alone == id);
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

#[path = "gestures_fling.rs"]
mod fling;
#[path = "gestures_motion.rs"]
mod motion;
use motion::{distance, pan, pinch};
#[path = "gestures_tap.rs"]
mod tap;

#[cfg(test)]
#[path = "gestures_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gestures_tap_tests.rs"]
mod tap_tests;

#[cfg(test)]
#[path = "gestures_fling_tests.rs"]
mod fling_tests;
