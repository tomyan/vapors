//! Platform: a window with a wgpu surface, a frame loop and input events.
//!
//! This is the seed of the framework's platform layer. It opens one
//! window (on the web, the page's canvas), picks the adapter by the
//! `vapors-gpu` rule (on the web, the browser's choice), hands every input
//! to the app as an [`Event`] and every redraw as a [`Frame`].

mod attributes;
mod clock;
mod error;
mod event;
mod gestures;
mod handler;
mod input;
mod report;
mod surface;
mod tasks;
#[cfg(target_family = "wasm")]
mod web;
mod window;

pub use error::PlatformError;
pub use event::{Event, TouchPhase};
pub use gestures::Gestures;
pub use window::{App, Frame, Next, WindowOptions, run};
