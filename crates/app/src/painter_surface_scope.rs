//! A renderer's own GPU errors, caught by a validation error scope
//! around each call into it and refused as its failure, naming the
//! surface, rather than panicking unnamed in wgpu's uncaptured-error
//! handler. Native wgpu resolves a scope when it is popped. On the web
//! the result arrives later: an unresolved scope is kept, polled with
//! the app's waker so that its result wakes the loop for a frame, and
//! checked on the surface's next frame, which it refuses instead. It
//! asks for no frame while it waits; its deadline is the time the loop
//! waits until, so a scope that never resolves still refuses.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use vapors_platform::Instant;

use super::super::errors::Refusal;
use crate::surface::SurfaceError;

/// A scope's result, once the GPU reports it.
pub(super) type Caught = Pin<Box<dyn Future<Output = Option<wgpu::Error>>>>;

/// How long a scope may stay unresolved before its surface is refused.
///
/// Measured 2026-11-10: headless Chromium 155.0.8059.79, WebGPU on
/// `SwiftShader` (Vulkan), load average 5 to 16 on 12 cores.
/// `popErrorScope` around a render pass and submit, once per animation
/// frame, resolved in p50 0.4 to 0.9 ms, p99 6 to 12 ms, max 37.6 ms
/// over 6,500 renders; around a call that makes a validation error, max
/// 8 ms over 650. Rerun: `PLAYWRIGHT_MODULE=<playwright-core/index.mjs>
/// node scripts/web-scope-latency.mjs [renders]`.
///
/// 1000 ms is about 26 times the largest. 100 ms would be under 3
/// times it, and the largest came from a loaded machine, where a stall
/// of the browser's GPU process can take longer still; a slow phone or
/// a heavier render adds more. Refusing a working surface is the worse
/// error, and a scope that never resolves is refused within a second
/// either way. Native wgpu resolves in the frame, so it never waits.
pub(crate) const DEADLINE: Duration = Duration::from_millis(1000);

/// What a scope is polled with: the waker its result wakes, and the
/// time now, against which deadlines are kept.
#[derive(Clone, Copy)]
pub(super) struct Turn<'a> {
    pub(super) waker: &'a Waker,
    pub(super) now: Instant,
}

/// The error scopes of one surface not yet seen resolved, each with its
/// deadline, and the first error one caught: once caught, every frame
/// of the surface is refused until it is remounted.
#[derive(Default)]
pub(super) struct Scopes {
    pending: Vec<(Caught, Instant)>,
    caught: Option<String>,
}

impl Scopes {
    /// Runs `call` inside a validation error scope on `device`: refused
    /// when it fails, or when it (or an earlier call) made a GPU error.
    pub(super) fn run<T>(
        &mut self,
        device: &wgpu::Device,
        turn: Turn<'_>,
        call: impl FnOnce() -> Result<T, SurfaceError>,
    ) -> Result<T, Refusal> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = call();
        self.wait_for(Box::pin(scope.pop()), turn);
        let checked = self.poll(turn.waker);
        let value = result.map_err(Refusal::Renderer)?;
        checked.map(|()| value)
    }

    /// Keeps `scope` until it resolves, refusing at `DEADLINE` from now.
    pub(super) fn wait_for(&mut self, scope: Caught, turn: Turn<'_>) {
        self.pending.push((scope, turn.now + DEADLINE));
    }

    /// The earliest deadline of an unresolved scope: the loop waits no
    /// later than this for a frame that checks it.
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.pending.iter().map(|(_, deadline)| *deadline).min()
    }

    /// On each frame of the surface: refused when a scope has caught an
    /// error (now or before), or is unresolved at its deadline.
    pub(super) fn check(&mut self, turn: Turn<'_>) -> Result<(), Refusal> {
        let late = self.pending.iter().any(|(_, at)| turn.now >= *at);
        if late {
            self.pending.clear();
            self.caught.get_or_insert_with(|| {
                format!(
                    "the GPU did not report this surface's errors within {} ms",
                    DEADLINE.as_millis()
                )
            });
        }
        self.poll(turn.waker)
    }

    /// Polls each scope once with `waker`, keeping the unresolved ones.
    fn poll(&mut self, waker: &Waker) -> Result<(), Refusal> {
        let mut context = Context::from_waker(waker);
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
