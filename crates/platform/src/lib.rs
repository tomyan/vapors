//! Platform: a window with a wgpu surface, a frame loop and input events.
//!
//! This is the seed of the framework's platform layer. It opens one
//! window (on the web, the page's canvas), picks the adapter by the
//! `vapors-gpu` rule (on the web, the browser's choice), hands every input
//! to the app as an [`Event`] and every redraw as a [`Frame`].

mod attributes;
mod clock;
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
mod picker;
#[cfg(target_family = "wasm")]
mod picker_web;
mod platform;
mod redraw;
mod report;
mod surface;
mod surface_texture;
mod tasks;
#[cfg(target_family = "wasm")]
mod web;
mod webgpu;
mod window;

pub use device_lost::{DeviceLost, Lost};
pub use error::PlatformError;
pub use event::{Event, TouchPhase};
pub use file_kind::FileKind;
pub use gestures::Gestures;
pub use platform::Platform;
pub use report::{report_error, report_status};
pub use window::{App, Frame, Next, WindowOptions, run};

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
