//! Why a window could not be opened or drawn to.

use std::fmt;

/// Every way the platform layer can fail to give the app a frame.
#[derive(Debug)]
pub enum PlatformError {
    /// The windowing system refused an event loop or a window.
    Windowing(String),
    /// No adapter can present to this window; the message lists what was found.
    NoAdapter(vapors_gpu::NoAdapter),
    /// The adapter would not open a device.
    Device(String),
    /// The surface offered no format the renderer accepts.
    Surface(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windowing(message) => write!(f, "windowing: {message}"),
            Self::NoAdapter(error) => write!(f, "{error}"),
            Self::Device(message) => write!(f, "device: {message}"),
            Self::Surface(message) => write!(f, "surface: {message}"),
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<vapors_gpu::NoAdapter> for PlatformError {
    fn from(error: vapors_gpu::NoAdapter) -> Self {
        Self::NoAdapter(error)
    }
}
