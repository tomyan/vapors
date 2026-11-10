//! A [`SurfaceHandle`] and what it shares with its [`ElementsApp`]
//! (docs/gpu-surface.md). `invalidate` marks the surface stale, for the
//! next frame to pass to the painter, and wakes the loop with the waker
//! the app kept from the platform; it answers in three states: woken,
//! left for the first frame (the loop has not started), or refused
//! (unmounted, or the loop has ended: the app failed or was dropped).
//! `request_animation` does the same and keeps the surface rendering
//! every frame until `stop_animation`. Handles are single-threaded, as
//! the document is: another thread sends to the app's thread and wakes
//! the loop with a `Waker`, which is `Send`.
//!
//! [`ElementsApp`]: super::ElementsApp

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

use vapors_platform::Waker;

/// What the app and its handles share.
#[derive(Default)]
pub(super) struct Shared {
    /// Every mounted name, pending or on the painter.
    pub(super) mounted: BTreeSet<String>,
    /// Names invalidated since the last frame passed them on.
    pub(super) stale: BTreeSet<String>,
    /// Names the app animates, passed to the painter on every frame.
    pub(super) animating: BTreeSet<String>,
    /// Whether the app's loop has started, runs, or has ended.
    pub(super) run: Run,
}

/// The app's loop, as its handles see it.
#[derive(Default)]
pub(super) enum Run {
    /// No event or frame has reached the app: the first frame renders
    /// every surface, with no wake.
    #[default]
    NotStarted,
    /// The loop runs; the platform's waker, kept from the first event
    /// or frame.
    Running(Waker),
    /// The app failed or was dropped: no frame will come.
    Closed,
}

/// Invalidates one mounted surface from outside the app; cheap to clone.
#[derive(Clone)]
pub struct SurfaceHandle {
    pub(super) name: String,
    pub(super) shared: Rc<RefCell<Shared>>,
}

/// What [`SurfaceHandle::invalidate`] or
/// [`SurfaceHandle::request_animation`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Invalidated {
    /// It woke the running loop for a frame that renders the surface.
    Woke,
    /// The loop has not started; its first frame renders the surface.
    AtFirstFrame,
}

/// Why a [`SurfaceHandle`] call did nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum InvalidateError {
    /// The surface's renderer was unmounted.
    NotMounted(String),
    /// The event loop has ended (the app failed or was dropped), so no
    /// frame will render the surface.
    LoopClosed(String),
}

impl fmt::Display for InvalidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMounted(name) => write!(
                f,
                "the gpu-surface {name:?} has no renderer mounted, so the call did nothing; mount one with ElementsApp::mount and call through the handle it returns"
            ),
            Self::LoopClosed(name) => write!(
                f,
                "the event loop has ended (the app failed or was dropped), so the gpu-surface {name:?} will not render again; invalidate only while the app runs"
            ),
        }
    }
}

impl std::error::Error for InvalidateError {}

impl SurfaceHandle {
    /// The surface's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Renders the surface on the next frame that paints it, and wakes
    /// the loop for that frame.
    ///
    /// # Errors
    ///
    /// [`InvalidateError::NotMounted`] when the renderer was unmounted;
    /// [`InvalidateError::LoopClosed`] when the loop has ended.
    pub fn invalidate(&self) -> Result<Invalidated, InvalidateError> {
        self.mark(|_| ())
    }

    /// Renders the surface on every frame that paints it, from the next,
    /// and wakes the loop for that frame; frames continue until
    /// [`stop_animation`](Self::stop_animation). On a surface not painted
    /// (hidden, zero-sized, clipped out) the request wakes one frame that
    /// renders nothing, then asks for none; the request is kept, and the
    /// surface animates again once a frame paints it.
    ///
    /// # Errors
    ///
    /// As [`invalidate`](Self::invalidate).
    pub fn request_animation(&self) -> Result<Invalidated, InvalidateError> {
        self.mark(|shared| {
            shared.animating.insert(self.name.clone());
        })
    }

    /// Stops the frames [`request_animation`](Self::request_animation)
    /// asked for: from the next frame the surface renders only when
    /// invalidated, and the app returns to waiting once nothing else
    /// wants a frame. Stopping a surface that is not animating, or after
    /// the loop has ended, does nothing.
    ///
    /// # Errors
    ///
    /// [`InvalidateError::NotMounted`] when the renderer was unmounted.
    pub fn stop_animation(&self) -> Result<(), InvalidateError> {
        let mut shared = self.shared.borrow_mut();
        if !shared.mounted.contains(&self.name) {
            return Err(InvalidateError::NotMounted(self.name.clone()));
        }
        shared.animating.remove(&self.name);
        Ok(())
    }

    /// Marks the surface stale, with `also` applied to what is shared,
    /// and wakes the loop.
    fn mark(&self, also: impl FnOnce(&mut Shared)) -> Result<Invalidated, InvalidateError> {
        let mut shared = self.shared.borrow_mut();
        if !shared.mounted.contains(&self.name) {
            return Err(InvalidateError::NotMounted(self.name.clone()));
        }
        let waker = match &shared.run {
            Run::NotStarted => None,
            Run::Running(waker) => Some(waker.clone()),
            Run::Closed => return Err(InvalidateError::LoopClosed(self.name.clone())),
        };
        shared.stale.insert(self.name.clone());
        also(&mut shared);
        drop(shared);
        self.wake(waker)
    }

    fn wake(&self, waker: Option<Waker>) -> Result<Invalidated, InvalidateError> {
        let Some(waker) = waker else {
            return Ok(Invalidated::AtFirstFrame);
        };
        waker
            .wake()
            .map(|()| Invalidated::Woke)
            .map_err(|_| InvalidateError::LoopClosed(self.name.clone()))
    }
}
