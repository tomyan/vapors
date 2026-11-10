//! When the painter's surfaces want the next frame: at once while one
//! is stale or animating, or when the waker it was given is woken by a
//! GPU error scope's error or deadline (docs/gpu-surface.md, Animation).

use std::task::Waker;

use super::super::ScenePainter;
use super::{Mounted, Surfaces};

impl ScenePainter {
    /// True when an active surface (one the last frame painted) is
    /// invalidated or animating: the app should draw another frame.
    #[must_use]
    pub fn wants_frame(&self) -> bool {
        self.surfaces.mounted.values().any(Mounted::wants_frame)
    }

    /// Sets the waker a surface's GPU error scope wakes when an error
    /// arrives after the frame, or its deadline passes first (on the
    /// web): the owner's loop then draws a frame that refuses the
    /// surface. A result with no error wakes nothing. Without a waker,
    /// either is seen on the next frame drawn for another reason. One
    /// that arrived before the waker was set wakes it now.
    pub fn wake_with(&mut self, waker: Waker) {
        self.surfaces.wake_with(waker);
    }

    /// Whether the painter has been given a waker ([`Self::wake_with`]).
    #[must_use]
    pub fn has_waker(&self) -> bool {
        self.surfaces.waker.borrow().is_some()
    }
}

impl Surfaces {
    fn wake_with(&mut self, waker: Waker) {
        let mut mounted = self.mounted.values_mut();
        if mounted.any(|mounted| mounted.scopes.error_waiting()) {
            waker.wake_by_ref();
        }
        *self.waker.borrow_mut() = Some(waker);
    }
}

#[cfg(test)]
#[path = "painter_surface_wake_tests.rs"]
mod tests;
