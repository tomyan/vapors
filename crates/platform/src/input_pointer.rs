//! What `Input` does with the pointer's position: where a wheel the page
//! placed moves it, and where a pinch is anchored.

use winit::event::WindowEvent;

use super::Input;
use crate::event::Event;

impl Input {
    /// A wheel event the page placed (`wheel_position`) as the pointer
    /// moving there, sent before the scroll, and kept as the pointer's
    /// position.
    pub(super) fn wheel_placement(&mut self, event: &WindowEvent) -> Option<Event> {
        if !matches!(event, WindowEvent::MouseWheel { .. }) {
            return None;
        }
        let [x, y] = crate::wheel_position::take()?;
        let moved = Event::PointerMoved { x, y };
        self.track(&moved);
        Some(moved)
    }

    /// A trackpad pinch, anchored at the pointer, else at the window's
    /// centre (winit's pinch carries no position); dropped before the
    /// window has a size, when there is nothing to zoom.
    #[allow(clippy::cast_possible_truncation)] // a trackpad delta is a small fraction
    pub(super) fn pinch(&self, delta: f64) -> Option<Event> {
        let [x, y] = self
            .pointer
            .or_else(|| self.size.map(|[width, height]| [width / 2.0, height / 2.0]))?;
        Some(Event::Pinch {
            x,
            y,
            factor: 1.0 + delta as f32,
        })
    }
}
