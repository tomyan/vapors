//! Platform: a window with a wgpu surface, a frame loop and input events.
//!
//! This is the seed of the framework's platform layer. It opens one
//! window (on the web, the page's canvas), picks the adapter by the
//! `vapors-gpu` rule (on the web, the browser's choice), hands every input
//! to the app as an [`Event`] and every redraw as a [`Frame`].

mod attributes;
#[cfg(not(target_family = "wasm"))]
mod clipboard;
mod clock;
mod cursor;
mod device_lost;
mod error;
mod event;
mod file_kind;
mod gestures;
mod handler;
mod input;
#[cfg(not(target_family = "wasm"))]
mod log_forward;
pub mod page;
mod paste_key;
mod picker;
#[cfg(target_family = "wasm")]
mod picker_web;
mod platform;
mod redraw;
mod report;
mod surface;
mod surface_texture;
mod tasks;
mod waker;
#[cfg(target_family = "wasm")]
mod web;
mod webgpu;
mod window;

#[cfg(not(target_family = "wasm"))]
pub use clipboard::{
    Clipboard, ClipboardImage, DEADLINE as CLIPBOARD_DEADLINE, MAX_BYTES as CLIPBOARD_MAX_BYTES,
    SystemClipboard,
};
pub use clock::Instant;
pub use cursor::Cursor;
pub use device_lost::{DeviceLost, Lost};
pub use error::PlatformError;
pub use event::{Event, LINE_PIXELS, TouchPhase};
pub use file_kind::FileKind;
pub use gestures::{Gestures, TAP_SLOP};
pub use platform::Platform;
pub use report::{report_error, report_status};
pub use waker::{LoopClosed, Waker};
pub use window::{App, Frame, Next, WindowOptions, run};

/// Hide the page's own HTML controls drawn over the canvas (the Open
/// button and the status line), for an app that draws its own; errors
/// are still shown, and a page without the Open button is not an error.
/// Call it before [`run`]. Natively there is no page, so it does
/// nothing.
pub fn hide_page_controls() {
    #[cfg(target_family = "wasm")]
    web::hide_page_controls();
}

/// The value of `name` in the page URL's query string (`?demo=hello`).
/// Natively there is no URL, so it is always `None`; a native app takes
/// the same choice from its command line.
#[must_use]
pub fn query_parameter(name: &str) -> Option<String> {
    #[cfg(target_family = "wasm")]
    return page::query_value(&web::search(), name);
    #[cfg(not(target_family = "wasm"))]
    {
        let _ = name;
        None
    }
}
