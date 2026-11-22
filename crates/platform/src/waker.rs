//! A request for a frame from outside the app's `event`: for an app that
//! returns [`Next::Wait`](crate::Next::Wait) and has work that starts
//! elsewhere (a page script's call, a future finishing, another thread).
//! `wake` posts a message to the loop, which asks the window for a frame;
//! wakes before that frame draw it once. Natively the waker is `Send`, as
//! the loop's proxy is; on the web there is one thread.
//!
//! The task hook's waker ([`Platform::task_waker`](crate::Platform::task_waker))
//! posts a wake of its own kind, [`Wake::Tasks`], which the loop hands to
//! [`App::tasks_woken`](crate::App::tasks_woken) rather than answering
//! with a frame (`docs/plans/async-components.md` section 3.3). Every
//! other waker's wake is [`Wake::Frame`] and still asks for a frame
//! without reaching the app.

use std::sync::Arc;

use winit::event_loop::EventLoopProxy;

use crate::Platform;
use crate::redraw::RedrawFlag;
use crate::window::{App, Message};

/// Why a wake asked for no frame: the event loop has ended, so the
/// window and its app are gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopClosed;

impl std::fmt::Display for LoopClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the event loop has ended, so the wake asked for no frame")
    }
}

impl std::error::Error for LoopClosed {}

/// Asks the loop for a frame; cheap to clone and keep.
#[derive(Clone)]
pub struct Waker {
    target: Target,
}

/// What a wake posted to the loop asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wake {
    /// A frame: surfaces, pastes, GPU error scopes, and an app's own waker.
    Frame,
    /// The app's woken tasks polled; the app asks for a frame if they
    /// changed something.
    Tasks,
}

/// The loop's answer to a posted wake.
pub(crate) fn deliver(wake: Wake, app: &mut dyn App, platform: &Platform) {
    match wake {
        Wake::Frame => platform.request_redraw(),
        Wake::Tasks => app.tasks_woken(platform),
    }
}

#[derive(Clone)]
enum Target {
    /// The window's loop, and the kind of wake posted to it.
    Loop(EventLoopProxy<Message>, Wake),
    /// A detached platform's flag, read back by
    /// [`Platform::take_redraw_request`](crate::Platform::take_redraw_request)
    /// for a frame waker, or
    /// [`Platform::take_task_wake`](crate::Platform::take_task_wake) for
    /// a task waker.
    Detached(Arc<RedrawFlag>),
}

impl Waker {
    pub(crate) fn for_loop(proxy: EventLoopProxy<Message>, wake: Wake) -> Self {
        Self {
            target: Target::Loop(proxy, wake),
        }
    }

    pub(crate) fn detached(flag: Arc<RedrawFlag>) -> Self {
        Self {
            target: Target::Detached(flag),
        }
    }

    /// Ask for a frame.
    ///
    /// # Errors
    ///
    /// [`LoopClosed`] when the loop has ended: no frame will come, and
    /// the caller says so if its work needed one.
    pub fn wake(&self) -> Result<(), LoopClosed> {
        match &self.target {
            Target::Loop(proxy, wake) => proxy
                .send_event(Message::Wake(*wake))
                .map_err(|_| LoopClosed),
            Target::Detached(flag) => {
                flag.raise();
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[path = "waker_tests.rs"]
mod tests;
