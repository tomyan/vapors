//! A request for a frame from outside the app's `event`: for an app that
//! returns [`Next::Wait`](crate::Next::Wait) and has work that starts
//! elsewhere (a page script's call, a future finishing, another thread).
//! `wake` posts a message to the loop, which asks the window for a frame;
//! wakes before that frame draw it once. Natively the waker is `Send`, as
//! the loop's proxy is; on the web there is one thread.

use std::sync::Arc;

use winit::event_loop::EventLoopProxy;

use crate::redraw::RedrawFlag;
use crate::window::Message;

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

#[derive(Clone)]
enum Target {
    /// The window's loop.
    Loop(EventLoopProxy<Message>),
    /// A detached platform's redraw flag, read back by
    /// [`Platform::take_redraw_request`](crate::Platform::take_redraw_request).
    Detached(Arc<RedrawFlag>),
}

impl Waker {
    pub(crate) fn for_loop(proxy: EventLoopProxy<Message>) -> Self {
        Self {
            target: Target::Loop(proxy),
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
            Target::Loop(proxy) => proxy.send_event(Message::Wake).map_err(|_| LoopClosed),
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
