//! A touch pan scrolls one container for the whole gesture, as
//! Chromium latches it: the first container the gesture moves, found
//! where the finger went down, takes every later pan, and a pan past
//! its end goes nowhere rather than out to the next one (F2c S9). It
//! also keeps the input mode for scroll thumbs: shown while a mouse is
//! in use, and on touch shown by each scroll, the gesture's or its
//! fling's, then faded (`thumb_fade.rs`). A fling (`fling.rs`) runs
//! until it ends or a finger goes down, and the booster
//! (`fling_booster.rs`) watches the gestures and flings for a fling to
//! boost.

use vapors_dom::NodeId;
use vapors_elements::{Document, Point};
use vapors_platform::Instant;

use crate::fling::Fling;
use crate::fling_booster::{FlingBooster, Observed};
use crate::thumb_fade::{Phase, ThumbFades};

/// The gesture under way.
#[derive(Debug, Default)]
pub(crate) struct TouchScroll {
    /// The fingers down now, by id.
    fingers: Vec<u64>,
    down: Option<Point>,
    latched: Option<NodeId>,
    /// The container the last gesture scrolled, once its fingers lifted:
    /// its fling's.
    released: Option<NodeId>,
    /// A finger touched since the mouse was last used.
    touching: bool,
    /// The fling under way.
    fling: Option<Fling>,
    /// Each container's touch thumbs' fade, kept across gestures.
    fades: ThumbFades,
    booster: FlingBooster,
}

impl TouchScroll {
    /// Finger `id` went down at `at` (CSS pixels) at `now`; the first
    /// finger stops a fling and starts a gesture there.
    pub(crate) fn down(&mut self, id: u64, at: Point, now: Instant) {
        if self.fingers.is_empty() {
            if self.fling.is_some() {
                self.booster.observe(Observed::FlingCancel, now);
            }
            *self = Self {
                down: Some(at),
                touching: true,
                fades: std::mem::take(&mut self.fades),
                booster: std::mem::take(&mut self.booster),
                ..Self::default()
            };
        }
        if !self.fingers.contains(&id) {
            self.fingers.push(id);
        }
    }

    /// The fingers moved the content by `delta` (CSS pixels, as a scroll
    /// delta) at `now`; whether a container moved.
    pub(crate) fn pan(&mut self, document: &mut Document, delta: [f32; 2], now: Instant) -> bool {
        if self.latched.is_none() {
            let Some(at) = self.down else { return false };
            self.latched = document.scroll_target(at, delta);
            if self.latched.is_some() {
                self.booster.observe(Observed::ScrollBegin, now);
            }
        }
        let Some(container) = self.latched else {
            return false;
        };
        self.booster.observe(Observed::ScrollUpdate(delta), now);
        let moved = document.scroll_node_by(container, delta);
        if moved {
            self.fades.scrolled(container, now);
        }
        moved
    }

    /// Finger `id` lifted or was taken away at `now`; the gesture ends
    /// when no finger is left down.
    pub(crate) fn up(&mut self, id: u64, now: Instant) {
        let Some(at) = self.fingers.iter().position(|f| *f == id) else {
            return;
        };
        self.fingers.remove(at);
        if self.fingers.is_empty() {
            self.end_gesture(now);
        }
    }

    /// Every finger was taken away at once (the window lost focus) at
    /// `now`; a fling runs on.
    pub(crate) fn forget(&mut self, now: Instant) {
        self.end_gesture(now);
    }

    /// No gesture: the input mode, a fling and the booster are kept, and
    /// the container the gesture scrolled, for its fling.
    fn end_gesture(&mut self, now: Instant) {
        if self.latched.is_some() {
            self.booster.observe(Observed::ScrollEnd, now);
        }
        *self = Self {
            touching: self.touching,
            fades: std::mem::take(&mut self.fades),
            released: self.latched,
            fling: self.fling.take(),
            booster: std::mem::take(&mut self.booster),
            ..Self::default()
        };
    }

    /// The mouse moved or scrolled: thumbs show again.
    pub(crate) fn mouse(&mut self) {
        self.touching = false;
    }

    /// Every container's thumbs drawn as they are at `now`: fully while
    /// the mouse is in use, else each at its own touch fade.
    pub(crate) fn draw_thumbs(&mut self, document: &mut Document, now: Instant) {
        if self.touching {
            document.set_thumb_opacity(0.0);
            self.fades.apply(document, now);
        } else {
            document.set_thumb_opacity(1.0);
            self.fades.hide_all(document);
        }
    }

    /// How many containers' fades are left.
    #[cfg(test)]
    pub(crate) fn fades_left(&self) -> usize {
        self.fades.len()
    }

    /// How opaque `container`'s thumbs are at `now`.
    #[cfg(test)]
    pub(crate) fn thumb_opacity(&self, container: NodeId, now: Instant) -> f32 {
        if self.touching {
            self.fades.opacity(container, now)
        } else {
            1.0
        }
    }

    /// Where the touch thumbs' fades are at `now` (hidden with the
    /// mouse, whose thumbs do not fade).
    pub(crate) fn thumb_phase(&self, now: Instant) -> Phase {
        if self.touching {
            self.fades.phase(now)
        } else {
            Phase::Hidden
        }
    }

    /// The container the gesture that just ended scrolled, if any, for
    /// its one fling.
    pub(crate) fn take_released(&mut self) -> Option<NodeId> {
        self.released.take()
    }

    /// `fling` replaces the one under way.
    pub(crate) fn fling(&mut self, fling: Fling) {
        self.fling = Some(fling);
    }

    /// The velocity a touch fling released at `velocity` at `now` starts
    /// at, boosted or not (`FlingBooster::start`).
    pub(crate) fn boosted(&mut self, velocity: [f32; 2], now: Instant) -> [f32; 2] {
        self.booster.observe(Observed::FlingStart, now);
        self.booster.start(velocity, now)
    }

    /// The running fling's speed at its start.
    #[cfg(test)]
    pub(crate) fn fling_starting_speed(&self) -> Option<f32> {
        self.fling.as_ref().map(Fling::starting_speed)
    }

    /// What the booster would boost the next fling by.
    #[cfg(test)]
    pub(crate) fn boost_from(&self) -> [f32; 2] {
        self.booster.previous_start()
    }

    /// A fling not from a touch: nothing to boost from or into.
    pub(crate) fn forget_boost(&mut self) {
        self.booster.reset();
    }

    /// Whether a finger is down.
    pub(crate) fn finger_down(&self) -> bool {
        !self.fingers.is_empty()
    }

    /// The wheel scrolled or a button went down: the fling stops, and
    /// nothing boosts from it (`FlingController::StopFling` resets the
    /// booster).
    pub(crate) fn stop_fling(&mut self) {
        self.fling = None;
        self.booster.reset();
    }

    /// Whether a fling is running.
    pub(crate) fn flinging(&self) -> bool {
        self.fling.is_some()
    }

    /// The fling moved on to `now`; whether one is still running.
    pub(crate) fn advance(&mut self, document: &mut Document, now: Instant) -> bool {
        let Some(fling) = self.fling.as_mut() else {
            return false;
        };
        let before = document.scroll_offset(fling.container());
        let running = fling.advance(document, now);
        if document.scroll_offset(fling.container()) != before {
            self.fades.scrolled(fling.container(), now);
        }
        self.booster.progress(fling.velocity());
        if !running {
            // a fling that ended, or met the edge, boosts nothing
            // (`ProgressFling` and `StopFlingingIfNecessary` reset it)
            self.fling = None;
            self.booster.reset();
        }
        running
    }
}

#[cfg(test)]
#[path = "touch_scroll_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "touch_scroll_thumb_tests.rs"]
mod thumb_tests;

#[cfg(test)]
#[path = "touch_scroll_boost_tests.rs"]
mod boost_tests;

#[cfg(test)]
#[path = "touch_scroll_chain_tests.rs"]
mod chain_tests;
