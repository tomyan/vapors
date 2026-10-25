//! A touch pan scrolls one container for the whole gesture, as
//! Chromium latches it: the first container the gesture moves, found
//! where the finger went down, takes every later pan, and a pan past
//! its end goes nowhere rather than out to the next one (F2c S9). It
//! also keeps the input mode for scroll thumbs: shown while a mouse is
//! in use, and on touch only while a gesture scrolls.

use vapors_dom::NodeId;
use vapors_elements::{Document, Point};

/// The gesture under way.
#[derive(Debug, Default)]
pub(crate) struct TouchScroll {
    /// The fingers down now, by id.
    fingers: Vec<u64>,
    down: Option<Point>,
    latched: Option<NodeId>,
    /// A finger touched since the mouse was last used.
    touching: bool,
}

impl TouchScroll {
    /// Finger `id` went down at `at` (CSS pixels); the first finger
    /// starts a gesture there.
    pub(crate) fn down(&mut self, id: u64, at: Point) {
        if self.fingers.is_empty() {
            *self = Self {
                down: Some(at),
                touching: true,
                ..Self::default()
            };
        }
        if !self.fingers.contains(&id) {
            self.fingers.push(id);
        }
    }

    /// The fingers moved the content by `delta` (CSS pixels, as a scroll
    /// delta); whether a container moved.
    pub(crate) fn pan(&mut self, document: &mut Document, delta: [f32; 2]) -> bool {
        if self.latched.is_none() {
            let Some(at) = self.down else { return false };
            self.latched = document.scroll_target(at, delta);
        }
        self.latched
            .is_some_and(|container| document.scroll_node_by(container, delta))
    }

    /// Finger `id` lifted or was taken away; the gesture ends when no
    /// finger is left down.
    pub(crate) fn up(&mut self, id: u64) {
        let Some(at) = self.fingers.iter().position(|f| *f == id) else {
            return;
        };
        self.fingers.remove(at);
        if self.fingers.is_empty() {
            *self = Self {
                touching: self.touching,
                ..Self::default()
            };
        }
    }

    /// The mouse moved or scrolled: thumbs show again.
    pub(crate) fn mouse(&mut self) {
        self.touching = false;
    }

    /// Whether scroll thumbs are drawn now.
    pub(crate) fn thumbs_shown(&self) -> bool {
        !self.touching || self.latched.is_some()
    }
}

#[cfg(test)]
#[path = "touch_scroll_tests.rs"]
mod tests;
