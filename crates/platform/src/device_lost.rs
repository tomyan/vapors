//! A lost GPU device: wgpu's lost callback (WebGPU's `device.lost` on
//! the web) records the loss here, the frame loop stops before its next
//! frame and reports it with the fix, and apps can ask. On phones a
//! device is lost when a tab is backgrounded or the GPU resets; without
//! this, the next GPU call panics in wgpu, the panic reaches only the
//! console, and the page shows its last frame as if alive.
//!
//! Recovery (a new device, every resource rebuilt) is not done here.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::PlatformError;

/// What wgpu said when the device was lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lost {
    /// wgpu's `DeviceLostReason`, by name: `Destroyed` or `Unknown`.
    pub reason: String,
    /// The driver's or the browser's message; may be empty.
    pub message: String,
}

/// Whether the window's device has been lost, shared between wgpu's
/// callback, the frame loop and the app (`Platform::device_lost`).
#[derive(Debug, Clone, Default)]
pub struct DeviceLost(Arc<Mutex<Option<Lost>>>);

impl DeviceLost {
    /// Record `device`'s loss here when wgpu reports it, then call
    /// `then` (the platform wakes its loop with it).
    pub fn watch(&self, device: &wgpu::Device, then: impl Fn() + Send + 'static) {
        let state = self.clone();
        device.set_device_lost_callback(move |reason, message| {
            state.record(Lost {
                reason: format!("{reason:?}"),
                message,
            });
            then();
        });
    }

    /// The loss, once there has been one.
    #[must_use]
    pub fn get(&self) -> Option<Lost> {
        self.lock().clone()
    }

    /// Keep the first loss: a later report is about the same device.
    pub fn record(&self, lost: Lost) {
        self.lock().get_or_insert(lost);
    }

    fn lock(&self) -> MutexGuard<'_, Option<Lost>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Where the app runs, for the fix a loss names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A page: reloading it opens a new device.
    Web,
    /// A native window: the app has to be started again.
    Native,
}

impl Target {
    /// The target this build is for.
    #[must_use]
    pub fn this_build() -> Self {
        if cfg!(target_family = "wasm") {
            Self::Web
        } else {
            Self::Native
        }
    }
}

/// `the GPU device was lost (<reason>: <message>); fix: <fix>`.
#[must_use]
pub fn sentence(lost: &Lost, target: Target, app: &str) -> String {
    let message = if lost.message.is_empty() {
        "no message"
    } else {
        &lost.message
    };
    let fix = match target {
        Target::Web => "reload the page".to_owned(),
        Target::Native => format!("restart {app}"),
    };
    format!(
        "the GPU device was lost ({}: {message}); fix: {fix}",
        lost.reason
    )
}

/// The error that stops the frame loop, when the device is lost.
#[must_use]
pub fn stop_for(lost: Option<Lost>, target: Target, app: &str) -> Option<PlatformError> {
    lost.map(|lost| PlatformError::DeviceLost(sentence(&lost, target, app)))
}

#[cfg(test)]
#[path = "device_lost_tests.rs"]
mod tests;
