//! The app's handle to the platform: what it can ask for from inside an
//! event. Answers arrive as later events, never as return values, so the
//! same code runs where the answer is a promise (the web) and where it is
//! a blocking dialog on another thread (native).

use winit::event_loop::EventLoopProxy;

use crate::picker;
use crate::window::Message;

/// Given to the app with every event.
pub struct Platform {
    proxy: EventLoopProxy<Message>,
}

impl Platform {
    pub(crate) fn new(proxy: EventLoopProxy<Message>) -> Self {
        Self { proxy }
    }

    pub(crate) fn proxy(&self) -> EventLoopProxy<Message> {
        self.proxy.clone()
    }

    /// Open the system's file picker for one image. The choice arrives as
    /// [`Event::FileOpened`](crate::Event::FileOpened); cancelling sends
    /// nothing. On the web this must be called from inside a user input
    /// event, which is where the app's `event` runs, or the browser
    /// refuses to open the picker.
    pub fn pick_file(&self) {
        picker::open(self.proxy());
    }
}
