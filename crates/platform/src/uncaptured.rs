//! A GPU error that no error scope caught: a validation error, out of
//! memory or an internal error from a call outside the painter's surface
//! scopes. wgpu's native default handler panics; on the web the browser
//! logs it to the console and drops the command buffer, and the page
//! keeps its last frame as if alive. The handler installed here records
//! the first such error and wakes the loop, which stops naming it, on
//! both targets.

use std::sync::Mutex;

use crate::PlatformError;
use crate::device_lost::Target;
use crate::first_report::FirstReport;

/// What wgpu said about an error no scope caught.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uncaptured {
    /// wgpu's error kind: `Validation`, `OutOfMemory` or `Internal`.
    pub kind: &'static str,
    /// wgpu's or the browser's description.
    pub message: String,
}

impl Uncaptured {
    /// The kind and description of `error`.
    #[must_use]
    pub fn of(error: &wgpu::Error) -> Self {
        let kind = match error {
            wgpu::Error::Validation { .. } => "Validation",
            wgpu::Error::OutOfMemory { .. } => "OutOfMemory",
            wgpu::Error::Internal { .. } => "Internal",
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

/// The first uncaptured error on the window's device, shared between
/// wgpu's handler and the frame loop.
#[derive(Debug, Clone, Default)]
pub struct UncapturedErrors(FirstReport<Uncaptured>);

impl UncapturedErrors {
    /// Make this `device`'s uncaptured-error handler: record each error
    /// here, then call `then` (the platform wakes its loop with it).
    pub fn watch(&self, device: &wgpu::Device, then: impl Fn() + Send + 'static) {
        let state = self.clone();
        // wgpu wants the handler `Sync`; the waker need only be `Send`.
        let then = Mutex::new(then);
        device.on_uncaptured_error(std::sync::Arc::new(move |error| {
            state.record(Uncaptured::of(&error));
            (then
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner))();
        }));
    }

    /// The first uncaptured error, once there has been one.
    #[must_use]
    pub fn get(&self) -> Option<Uncaptured> {
        self.0.get()
    }

    /// Keep the first error: a later one may be its consequence. A
    /// later one is logged as a warning, so it is not lost.
    pub fn record(&self, error: Uncaptured) {
        let (kind, message) = (error.kind, error.message.clone());
        if !self.0.record(error) {
            log::warn!(
                "a later uncaptured GPU error, not the one that stops the loop ({kind}: {message})"
            );
        }
    }
}

/// `a GPU call failed with no error scope to catch it (<kind>: <message>); ...; fix: <fix>`.
#[must_use]
pub fn sentence(error: &Uncaptured, target: Target, app: &str) -> String {
    let fix = match target {
        Target::Web => "reload the page".to_owned(),
        Target::Native => format!("restart {app}"),
    };
    format!(
        "a GPU call failed with no error scope to catch it ({}: {}); the frame it belonged to \
         was not drawn. This is a bug in {app} or vapors, so report this message; fix: {fix}",
        error.kind, error.message
    )
}

/// The error that stops the frame loop, once an uncaptured error is recorded.
#[must_use]
pub fn stop_for(error: Option<Uncaptured>, target: Target, app: &str) -> Option<PlatformError> {
    error.map(|error| PlatformError::UncapturedGpuError(sentence(&error, target, app)))
}

#[cfg(test)]
#[path = "uncaptured_tests.rs"]
mod tests;
