//! A gesture's lone finger lifting past the tap slop flings at its
//! velocity (`velocity_tracker.rs`): an [`Event::Fling`] when it moved,
//! nothing when it had stopped, and, when the velocity could not be
//! measured, no fling, counted ([`Gestures::unmeasured_lifts`]) and
//! named in a debug build.

use std::time::Duration;

use super::Gestures;
use crate::event::Event;
use crate::velocity_tracker::{Unresolvable, Velocity};

impl Gestures {
    /// The lone finger `id` lifted past the tap slop at `now`.
    pub(super) fn fling(&mut self, id: u64, now: Duration) -> Vec<Event> {
        if self.alone.take() != Some(id) {
            return Vec::new();
        }
        self.tracker.up(now);
        match self.tracker.velocity(f32::MAX) {
            Velocity::Moving([vx, vy]) => vec![Event::Fling {
                vx,
                vy,
                at: self.epoch + now,
            }],
            Velocity::Stopped => Vec::new(),
            Velocity::Unresolvable(why) => {
                self.unmeasured(why);
                Vec::new()
            }
        }
    }

    /// How many lifts past the tap slop had a velocity that could not
    /// be measured, so did not fling.
    #[must_use]
    pub fn unmeasured_lifts(&self) -> u32 {
        self.unmeasured
    }

    fn unmeasured(&mut self, why: Unresolvable) {
        self.unmeasured += 1;
        if cfg!(debug_assertions) {
            crate::report::report_error(&format!(
                "touch fling: the lifted finger's velocity could not be measured ({why:?}; lift {} unmeasured), so it did not fling",
                self.unmeasured
            ));
        }
    }
}
