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
    down: Option<Point>,
    latched: Option<NodeId>,
    /// A finger touched since the mouse was last used.
    touching: bool,
}

impl TouchScroll {
    /// A finger went down at `at` (CSS pixels).
    pub(crate) fn down(&mut self, at: Point) {
        if self.down.is_none() {
            *self = Self {
                down: Some(at),
                latched: None,
                touching: true,
            };
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

    /// The gesture ended.
    pub(crate) fn up(&mut self) {
        *self = Self {
            touching: self.touching,
            ..Self::default()
        };
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
