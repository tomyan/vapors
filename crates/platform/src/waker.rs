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

    /// Ask for a frame. After the loop has ended there is no frame to
    /// draw and nothing to say: the wake is dropped.
    pub fn wake(&self) {
        match &self.target {
            Target::Loop(proxy) => crate::tasks::send(proxy, Message::Wake),
            Target::Detached(flag) => flag.raise(),
        }
    }
}

#[cfg(test)]
#[path = "waker_tests.rs"]
mod tests;
