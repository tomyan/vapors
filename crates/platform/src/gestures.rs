//! Touch gestures from raw touches: one finger pans, two fingers pinch
//! about their midpoint and pan by its movement, a quick still press is a
//! tap, and two taps close together are a double tap.
//!
//! Pure: fed each touch with a monotonic time, it returns the gestures
//! that touch completed. The window module supplies the clock.

use std::time::Duration;

use crate::event::{Event, TouchPhase};

/// A press longer than this is not a tap.
const TAP_TIME: Duration = Duration::from_millis(300);
/// A press that moves further than this is not a tap.
const TAP_SLOP: f32 = 20.0;
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
        self.fingers.push(Finger { id, at });
        self.press = (self.fingers.len() == 1).then_some(Press {
            id,
            at,
            started: now,
        });
        Vec::new()
    }

    fn moved(&mut self, id: u64, at: [f32; 2]) -> Vec<Event> {
        let Some(index) = self.fingers.iter().position(|f| f.id == id) else {
            return Vec::new();
        };
        let before = std::mem::replace(&mut self.fingers[index].at, at);
        if let Some(press) = &self.press
            && press.id == id
            && distance(press.at, at) > TAP_SLOP
        {
            self.press = None;
        }
        match self.fingers.len() {
            1 => vec![pan(before, at)],
            2 => pinch(self.fingers[1 - index].at, before, at),
            _ => Vec::new(),
        }
    }

    fn ended(&mut self, id: u64, at: [f32; 2], now: Duration) -> Vec<Event> {
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
}

fn pan(before: [f32; 2], after: [f32; 2]) -> Event {
    Event::Pan {
        dx: after[0] - before[0],
        dy: after[1] - before[1],
    }
}

/// The pinch and pan from one finger moving while `other` stays put.
fn pinch(other: [f32; 2], before: [f32; 2], after: [f32; 2]) -> Vec<Event> {
    let spread_before = distance(other, before);
    let spread_after = distance(other, after);
    let factor = if spread_before > 0.0 {
        spread_after / spread_before
    } else {
        1.0
    };
    let mid_before = midpoint(other, before);
    let mid_after = midpoint(other, after);
    vec![
        Event::Pinch {
            x: mid_after[0],
            y: mid_after[1],
            factor,
        },
        pan(mid_before, mid_after),
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
