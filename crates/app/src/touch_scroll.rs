//! A touch pan scrolls one container for the whole gesture, as
//! Chromium latches it: the first container the gesture moves, found
//! where the finger went down, takes every later pan, and a pan past
//! its end goes nowhere rather than out to the next one (F2c S9). It
//! also keeps the input mode for scroll thumbs: shown while a mouse is
//! in use, and on touch shown by each scroll, the gesture's or its
//! fling's, then faded (`thumb_fade.rs`). A fling (`fling.rs`) runs
//! until it ends or a finger goes down.

use vapors_dom::NodeId;
use vapors_elements::{Document, Point};
use vapors_platform::Instant;

use crate::fling::Fling;
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
}

impl TouchScroll {
    /// Finger `id` went down at `at` (CSS pixels); the first finger
    /// stops a fling and starts a gesture there.
    pub(crate) fn down(&mut self, id: u64, at: Point) {
        if self.fingers.is_empty() {
            *self = Self {
                down: Some(at),
                touching: true,
                fades: std::mem::take(&mut self.fades),
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
        }
        let Some(container) = self.latched else {
            return false;
        };
        let moved = document.scroll_node_by(container, delta);
        if moved {
            self.fades.scrolled(container, now);
        }
        moved
    }

    /// Finger `id` lifted or was taken away; the gesture ends when no
    /// finger is left down.
    pub(crate) fn up(&mut self, id: u64) {
        let Some(at) = self.fingers.iter().position(|f| *f == id) else {
            return;
        };
        self.fingers.remove(at);
        if self.fingers.is_empty() {
            self.end_gesture();
        }
    }

    /// Every finger was taken away at once (the window lost focus); a
    /// fling runs on.
    pub(crate) fn forget(&mut self) {
        self.end_gesture();
    }

    /// No gesture: the input mode and a fling are kept, and the
    /// container the gesture scrolled, for its fling.
    fn end_gesture(&mut self) {
        *self = Self {
            touching: self.touching,
            fades: std::mem::take(&mut self.fades),
            released: self.latched,
            fling: self.fling.take(),
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

    /// Whether a finger is down.
    pub(crate) fn finger_down(&self) -> bool {
        !self.fingers.is_empty()
    }

    /// The wheel scrolled or a button went down: the fling stops.
    pub(crate) fn stop_fling(&mut self) {
        self.fling = None;
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
        if !running {
            self.fling = None;
        }
        running
    }
}

#[cfg(test)]
#[path = "touch_scroll_tests.rs"]
mod tests;
