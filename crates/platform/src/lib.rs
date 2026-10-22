//! Platform: a window with a wgpu surface, a frame loop and input events.
//!
//! This is the seed of the framework's platform layer. For now it opens
//! one window, picks the adapter by the `vapors-gpu` rule, hands every
//! input to the app as an [`Event`] and every redraw as a [`Frame`].

mod clock;
mod error;
mod event;
mod gestures;
mod input;
mod surface;
mod window;

pub use error::PlatformError;
pub use event::{Event, TouchPhase};
pub use gestures::Gestures;
pub use window::{App, Frame, Next, WindowOptions, run};
