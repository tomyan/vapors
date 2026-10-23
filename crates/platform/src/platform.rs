//! The app's handle to the platform: what it can ask for from inside an
//! event. Answers arrive as later events, never as return values, so the
//! same code runs where the answer is a promise (the web) and where it is
//! a blocking dialog on another thread (native).

use std::cell::Cell;
use std::sync::Arc;

use winit::event_loop::EventLoopProxy;

use crate::Cursor;

use crate::PlatformError;
use crate::device_lost::{DeviceLost, Lost};
use crate::file_kind::{self, FileKind};
use crate::picker;
use crate::redraw::RedrawFlag;
use crate::waker::Waker;
use crate::window::Message;

/// Given to the app with every event.
pub struct Platform {
    /// `None` for a detached platform, which has no event loop.
    proxy: Option<EventLoopProxy<Message>>,
    lost: DeviceLost,
    redraw: Arc<RedrawFlag>,
    cursor: Cell<Option<Cursor>>,
}

impl Platform {
    pub(crate) fn new(proxy: EventLoopProxy<Message>) -> Self {
        Self {
            proxy: Some(proxy),
            ..Self::detached()
        }
    }

    /// A platform with no event loop, to drive an app's `event` in a
    /// test without a window: its requests are recorded and can be read
    /// back, and `pick_file` refuses.
    #[must_use]
    pub fn detached() -> Self {
        Self {
            proxy: None,
            lost: DeviceLost::default(),
            redraw: Arc::default(),
            cursor: Cell::new(None),
        }
    }

    /// Ask for a frame: what an app that returns
    /// [`Next::Wait`](crate::Next::Wait) calls from `event` when what it
    /// shows has changed. Several calls before the frame draw it once.
    pub fn request_redraw(&self) {
        self.redraw.raise();
    }

    /// A handle that asks for a frame from outside `event`: from a page
    /// script's call, a future, or another thread natively. Keep it from
    /// the first event; an app returning [`Next::Wait`](crate::Next::Wait)
    /// calls `wake` when work that started elsewhere has something to
    /// show. A detached platform's waker raises the request
    /// [`take_redraw_request`](Self::take_redraw_request) reads.
    #[must_use]
    pub fn waker(&self) -> Waker {
        match &self.proxy {
            Some(proxy) => Waker::for_loop(proxy.clone()),
            None => Waker::detached(Arc::clone(&self.redraw)),
        }
    }

    /// Whether a frame was asked for since the last call; clears it.
    /// The loop reads it after each event; a test may too.
    #[must_use]
    pub fn take_redraw_request(&self) -> bool {
        self.redraw.take()
    }

    /// Show `cursor` over the window, from the end of the event being
    /// handled; the last call in an event wins.
    pub fn set_cursor(&self, cursor: Cursor) {
        self.cursor.set(Some(cursor));
    }

    /// The cursor asked for since the last call; clears it.
    pub(crate) fn take_cursor(&self) -> Option<Cursor> {
        self.cursor.take()
    }

    /// The state the window's device is watched into.
    pub(crate) fn lost_state(&self) -> DeviceLost {
        self.lost.clone()
    }

    /// The window's GPU device loss, once it has happened. After a loss
    /// the platform draws no more frames and has reported it; an app
    /// that makes GPU calls outside `frame` (a readback still running)
    /// asks here first, since such a call on a lost device panics in
    /// wgpu. A deliberate `Device::destroy` counts as a loss (reason
    /// `Destroyed`); dropping the device does not (pinned by
    /// `tests/gpu_device_lost.rs`).
    #[must_use]
    pub fn device_lost(&self) -> Option<Lost> {
        self.lost.get()
    }

    /// The loop's proxy.
    ///
    /// # Panics
    /// On a detached platform, which the loop is never given.
    pub(crate) fn proxy(&self) -> EventLoopProxy<Message> {
        self.proxy
            .clone()
            .expect("the window's own platform has a proxy; only Platform::detached lacks one")
    }

    /// Open the system's file picker for one file of one of `kinds`. The
    /// choice arrives as [`Event::FileOpened`](crate::Event::FileOpened);
    /// cancelling sends nothing. On the web this must be called from
    /// inside a user input event, which is where the app's `event` runs,
    /// or the browser refuses to open the picker.
    ///
    /// # Errors
    ///
    /// [`PlatformError::Picker`] when `kinds` is empty or a kind has no
    /// extensions, or the platform is detached; the picker is not opened.
    pub fn pick_file(&self, kinds: &[FileKind]) -> Result<(), PlatformError> {
        file_kind::check(kinds)?;
        let Some(proxy) = &self.proxy else {
            return Err(PlatformError::Picker(
                "this platform is detached (Platform::detached), so it has no picker to open"
                    .to_owned(),
            ));
        };
        picker::open(proxy.clone(), kinds.to_vec());
        Ok(())
    }
}
