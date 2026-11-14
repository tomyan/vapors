//! Why a window could not be opened or drawn to.

use std::fmt;

/// Every way the platform layer can fail to give the app a frame.
#[derive(Debug)]
pub enum PlatformError {
    /// The windowing system refused an event loop or a window.
    Windowing(String),
    /// No adapter can present to this window; the message lists what was found.
    NoAdapter(vapors_gpu::NoAdapter),
    /// The browser offered no WebGPU adapter.
    NoWebGpu(String),
    /// The adapter would not open a device.
    Device(String),
    /// The surface offered no format the renderer accepts.
    Surface(String),
    /// The file picker was asked for something it cannot show.
    Picker(String),
    /// The device was lost; the sentence names wgpu's reason and the fix.
    DeviceLost(String),
    /// A GPU call failed with no error scope to catch it; the sentence
    /// names wgpu's error and the fix.
    UncapturedGpuError(String),
    /// The app's `App::start`, before the first frame, refused.
    Start(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windowing(message) => write!(f, "windowing: {message}"),
            Self::NoAdapter(error) => write!(f, "{error}"),
            Self::NoWebGpu(message) => write!(
                f,
                "no WebGPU adapter: {message}. Fix: open this page in a browser with WebGPU \
                 (Chrome or Edge 113+, Safari 26+, Firefox 141+) over https or localhost."
            ),
            Self::Device(message) => write!(f, "device: {message}"),
            Self::Surface(message) => write!(f, "surface: {message}"),
            Self::Picker(message) => write!(f, "picker: {message}"),
            Self::Start(message) => write!(f, "the app could not start on this GPU: {message}"),
            Self::DeviceLost(sentence) | Self::UncapturedGpuError(sentence) => {
                write!(f, "{sentence}")
            }
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<vapors_gpu::LimitsBelowDefault> for PlatformError {
    fn from(error: vapors_gpu::LimitsBelowDefault) -> Self {
        Self::Device(error.to_string())
    }
}

impl From<vapors_gpu::NoAdapter> for PlatformError {
    fn from(error: vapors_gpu::NoAdapter) -> Self {
        Self::NoAdapter(error)
    }
}
