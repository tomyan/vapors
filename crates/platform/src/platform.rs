//! The app's handle to the platform: what it can ask for from inside an
//! event. Answers arrive as later events, never as return values, so the
//! same code runs where the answer is a promise (the web) and where it is
//! a blocking dialog on another thread (native).

use winit::event_loop::EventLoopProxy;

use crate::PlatformError;
use crate::device_lost::{DeviceLost, Lost};
use crate::file_kind::{self, FileKind};
use crate::picker;
use crate::redraw::RedrawFlag;
use crate::window::Message;

/// Given to the app with every event.
pub struct Platform {
    proxy: EventLoopProxy<Message>,
    lost: DeviceLost,
    redraw: RedrawFlag,
}

impl Platform {
    pub(crate) fn new(proxy: EventLoopProxy<Message>) -> Self {
        Self {
            proxy,
            lost: DeviceLost::default(),
            redraw: RedrawFlag::default(),
        }
    }

    /// Ask for a frame: what an app that returns
    /// [`Next::Wait`](crate::Next::Wait) calls from `event` when what it
    /// shows has changed. Several calls before the frame draw it once.
    pub fn request_redraw(&self) {
        self.redraw.raise();
    }

    /// Whether a frame was asked for since the last call; clears it.
    pub(crate) fn take_redraw_request(&self) -> bool {
        self.redraw.take()
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

    pub(crate) fn proxy(&self) -> EventLoopProxy<Message> {
        self.proxy.clone()
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
    /// extensions; the picker is not opened.
    pub fn pick_file(&self, kinds: &[FileKind]) -> Result<(), PlatformError> {
        file_kind::check(kinds)?;
        picker::open(self.proxy(), kinds.to_vec());
        Ok(())
    }
}
