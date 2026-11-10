//! A renderer's own GPU errors, caught by a validation error scope
//! around each call into it and refused as its failure, naming the
//! surface, rather than panicking unnamed in wgpu's uncaptured-error
//! handler. Native wgpu resolves a scope when it is popped. On the web
//! the result arrives later: an unresolved scope is kept and checked on
//! the surface's next frame, which it refuses instead.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use super::super::errors::Refusal;
use crate::surface::SurfaceError;

/// A scope's result, once the GPU reports it.
pub(super) type Caught = Pin<Box<dyn Future<Output = Option<wgpu::Error>>>>;

/// Frames of the surface a scope may stay unresolved before the
/// surface is refused.
///
/// RELAXED guard: the 10 frames (about 170 ms at 60 Hz) is stated, not
/// measured. The expectation, unmeasured, is that a WebGPU
/// `popErrorScope` resolves within a frame or two of the GPU process
/// handling the commands before it, and the rest is slack for a slow
/// frame. Retired when popErrorScope latency in headless Chromium
/// WebGPU is measured and this bound is set from the measurement.
/// Native wgpu resolves in the frame, so it never waits.
const DEADLINE_FRAMES: u32 = 10;

/// The error scopes of one surface not yet seen resolved, and the first
/// error one caught: once caught, every frame of the surface is refused
/// until it is remounted.
#[derive(Default)]
pub(super) struct Scopes {
    pending: Vec<(Caught, u32)>,
    caught: Option<String>,
}

impl Scopes {
    /// Runs `call` inside a validation error scope on `device`: refused
    /// when it fails, or when it (or an earlier call) made a GPU error.
    pub(super) fn run<T>(
        &mut self,
        device: &wgpu::Device,
        call: impl FnOnce() -> Result<T, SurfaceError>,
    ) -> Result<T, Refusal> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = call();
        self.wait_for(Box::pin(scope.pop()));
        let checked = self.poll();
        let value = result.map_err(Refusal::Renderer)?;
        checked.map(|()| value)
    }

    /// Keeps `scope` to poll on each frame until it resolves.
    pub(super) fn wait_for(&mut self, scope: Caught) {
        self.pending.push((scope, 0));
    }

    /// True while a scope is unresolved: the surface wants a frame.
    pub(super) fn waiting(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Once per frame of the surface: refused when a scope has caught an
    /// error (now or before), or has been unresolved for
    /// `DEADLINE_FRAMES` frames.
    pub(super) fn check(&mut self) -> Result<(), Refusal> {
        self.pending.iter_mut().for_each(|(_, frames)| *frames += 1);
        if self
            .pending
            .iter()
            .any(|(_, frames)| *frames >= DEADLINE_FRAMES)
        {
            self.pending.clear();
            self.caught.get_or_insert_with(|| {
                format!(
                    "the GPU did not report this surface's errors within {DEADLINE_FRAMES} frames"
                )
            });
        }
        self.poll()
    }

    /// Polls each scope once, keeping the unresolved ones.
    fn poll(&mut self) -> Result<(), Refusal> {
        let mut context = Context::from_waker(Waker::noop());
        let caught = &mut self.caught;
        self.pending
            .retain_mut(|(scope, _)| match scope.as_mut().poll(&mut context) {
                Poll::Ready(error) => {
                    if let (None, Some(error)) = (&caught, error) {
                        *caught = Some(error.to_string());
                    }
                    false
                }
                Poll::Pending => true,
            });
        match &self.caught {
            None => Ok(()),
            Some(error) => Err(Refusal::Renderer(
                format!("{error} (refused until it is remounted)").into(),
            )),
        }
    }
}

#[cfg(test)]
#[path = "painter_surface_scope_tests.rs"]
mod tests;
