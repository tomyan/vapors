//! Watching one GPU error scope until its result arrives, waking the
//! loop only when the result is an error: a scope that caught nothing
//! needs no frame, and a surface that has stopped rendering then draws
//! no more frames than its last render did (docs/gpu-surface.md).
//!
//! Native wgpu resolves a scope when it is popped, so its result is
//! read at once. On the web it arrives later: the scope is driven by
//! the page's own task queue (`spawn_local`), off the frame loop.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use super::Caught;

/// The loop's waker once the owner gives it (`ScenePainter::wake_with`),
/// shared with every watched scope: an error wakes whichever waker is
/// current when it arrives.
pub(in super::super) type LoopWaker = Rc<RefCell<Option<Waker>>>;

/// What a watched scope has reported so far.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) enum Outcome {
    /// No result yet.
    #[default]
    Waiting,
    /// Resolved with no error.
    Clean,
    /// Resolved with this error.
    Caught(String),
}

/// A scope being watched: where its result lands, and natively the
/// scope itself if it was not resolved when popped.
pub(super) struct Watched {
    outcome: Rc<RefCell<Outcome>>,
    #[cfg(not(target_family = "wasm"))]
    unresolved: Option<Watch>,
}

impl Watched {
    /// Reads `scope`'s result if it has one now; otherwise drives it
    /// until it does, waking `waker` if the result is an error.
    pub(super) fn new(mut scope: Caught, waker: &LoopWaker) -> Self {
        let outcome = Rc::new(RefCell::new(Outcome::Waiting));
        let mut context = Context::from_waker(Waker::noop());
        let unresolved = match scope.as_mut().poll(&mut context) {
            Poll::Ready(error) => {
                *outcome.borrow_mut() = outcome_of(error);
                None
            }
            Poll::Pending => Some(Watch::new(scope, &outcome, waker)),
        };
        Self::keeping(outcome, unresolved)
    }

    /// The scope's result so far.
    pub(super) fn outcome(&mut self) -> Outcome {
        #[cfg(not(target_family = "wasm"))]
        self.poll_unresolved();
        self.outcome.borrow().clone()
    }

    #[cfg(target_family = "wasm")]
    fn keeping(outcome: Rc<RefCell<Outcome>>, unresolved: Option<Watch>) -> Self {
        if let Some(watch) = unresolved {
            wasm_bindgen_futures::spawn_local(watch);
        }
        Self { outcome }
    }

    /// Natively there is no task queue to drive it: a scope wgpu did not
    /// resolve when popped (it always does) is polled on each read, and
    /// its result is seen on the surface's next frame or at its deadline.
    #[cfg(not(target_family = "wasm"))]
    fn keeping(outcome: Rc<RefCell<Outcome>>, unresolved: Option<Watch>) -> Self {
        Self {
            outcome,
            unresolved,
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn poll_unresolved(&mut self) {
        let Some(watch) = self.unresolved.as_mut() else {
            return;
        };
        let mut context = Context::from_waker(Waker::noop());
        if Pin::new(watch).poll(&mut context).is_ready() {
            self.unresolved = None;
        }
    }
}

/// Drives a scope to its result, records it, and wakes the loop if it
/// is an error, for a frame that refuses the surface.
struct Watch {
    scope: Caught,
    outcome: Rc<RefCell<Outcome>>,
    waker: LoopWaker,
}

impl Watch {
    fn new(scope: Caught, outcome: &Rc<RefCell<Outcome>>, waker: &LoopWaker) -> Self {
        Self {
            scope,
            outcome: Rc::clone(outcome),
            waker: Rc::clone(waker),
        }
    }
}

impl Future for Watch {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let Poll::Ready(error) = self.scope.as_mut().poll(context) else {
            return Poll::Pending;
        };
        let outcome = outcome_of(error);
        let caught = matches!(outcome, Outcome::Caught(_));
        *self.outcome.borrow_mut() = outcome;
        if let Some(waker) = self.waker.borrow().as_ref().filter(|_| caught) {
            waker.wake_by_ref();
        }
        Poll::Ready(())
    }
}

fn outcome_of(error: Option<wgpu::Error>) -> Outcome {
    error.map_or(Outcome::Clean, |error| Outcome::Caught(error.to_string()))
}

#[cfg(test)]
#[path = "painter_surface_scope_watch_tests.rs"]
mod tests;
