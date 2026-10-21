//! Platform: a window with a wgpu surface and a frame loop.
//!
//! This is the seed of the framework's platform layer. For now it opens
//! one window, picks the adapter by the `vapors-gpu` rule, and calls a
//! frame handler with the surface texture each time the window redraws.

mod error;
mod surface;
mod window;

pub use error::PlatformError;
pub use window::{Frame, Next, WindowOptions, run};
