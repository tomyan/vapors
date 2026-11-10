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

type Caught = Pin<Box<dyn Future<Output = Option<wgpu::Error>>>>;

/// The error scopes of one surface not yet seen resolved.
#[derive(Default)]
pub(super) struct Scopes {
    pending: Vec<Caught>,
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
        self.pending.push(Box::pin(scope.pop()));
        let checked = self.check();
        let value = result.map_err(Refusal::Renderer)?;
        checked.map(|()| value)
    }

    /// Refuses with the first error a resolved scope caught; keeps the
    /// unresolved ones for the next check.
    pub(super) fn check(&mut self) -> Result<(), Refusal> {
        let mut context = Context::from_waker(Waker::noop());
        let mut caught = None;
        self.pending
            .retain_mut(|scope| match scope.as_mut().poll(&mut context) {
                Poll::Ready(error) => {
                    caught = caught.take().or(error);
                    false
                }
                Poll::Pending => true,
            });
        caught.map_or(Ok(()), |error| Err(Refusal::Renderer(Box::new(error))))
    }
}
