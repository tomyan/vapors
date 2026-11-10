//! Watching one GPU error scope until its result arrives or its
//! deadline passes, waking the loop only for an error or the deadline:
//! a scope that caught nothing needs no frame, so an idle app draws
//! nothing (docs/gpu-surface.md).
//!
//! Native wgpu resolves a scope when it is popped, so its result is
//! read at once. On the web it arrives later: the scope races a page
//! timer (`vapors_platform::Timer`) in a task on the page's queue
//! (`spawn_local`), off the frame loop.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};

use super::Caught;

/// The loop's waker once the owner gives it (`ScenePainter::wake_with`),
/// shared with every watched scope: an error wakes whichever waker is
/// current when it arrives.
pub(in super::super) type LoopWaker = Rc<RefCell<Option<Waker>>>;

/// A scope's deadline: resolves, with the refusal to record, if it
/// passes before the scope's result. Dropping it cancels it.
type Deadline = Pin<Box<dyn Future<Output = String>>>;

/// What a watched scope has reported so far.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) enum Outcome {
    /// No result yet.
    #[default]
    Waiting,
    /// Resolved with no error.
    Clean,
    /// Resolved with this error, or refused at its deadline.
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
    /// Reads `scope`'s result if it has one now; otherwise watches it
    /// until it does or its deadline passes, waking `waker` for an
    /// error or the deadline.
    pub(super) fn new(mut scope: Caught, waker: &LoopWaker) -> Self {
        let outcome = Rc::new(RefCell::new(Outcome::Waiting));
        let mut context = Context::from_waker(Waker::noop());
        let unresolved = match scope.as_mut().poll(&mut context) {
            Poll::Ready(error) => {
                *outcome.borrow_mut() = outcome_of(error);
                None
            }
            Poll::Pending => Some(Watch::new(scope, deadline(), &outcome, waker)),
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
    /// given up by the surface's first frame after its deadline
    /// (`Scopes::settle`).
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

/// The page's timer at `DEADLINE`, counting visible time only
/// (`visible_deadline`).
#[cfg(target_family = "wasm")]
fn deadline() -> Deadline {
    Box::pin(super::deadline::visible_deadline(super::deadline::WebPage))
}

/// Natively a scope resolves when popped; frames judge one that did not.
#[cfg(not(target_family = "wasm"))]
fn deadline() -> Deadline {
    Box::pin(std::future::pending())
}

/// Drives a scope to its result or its deadline, records which, and
/// wakes the loop for an error or the deadline, for a frame that
/// refuses the surface. A clean result drops, and so cancels, the
/// deadline. The outcome is held weakly: once the surface's scopes are
/// dropped (unmount or remount), nothing reads it, so the watch ends on
/// its next poll, dropping its scope and deadline and waking nothing.
struct Watch {
    scope: Caught,
    deadline: Deadline,
    outcome: Weak<RefCell<Outcome>>,
    waker: LoopWaker,
}

impl Watch {
    fn new(
        scope: Caught,
        deadline: Deadline,
        outcome: &Rc<RefCell<Outcome>>,
        waker: &LoopWaker,
    ) -> Self {
        let (outcome, waker) = (Rc::downgrade(outcome), Rc::clone(waker));
        Self {
            scope,
            deadline,
            outcome,
            waker,
        }
    }

    fn record(&self, outcome: Outcome) {
        let Some(slot) = self.outcome.upgrade() else {
            return;
        };
        let caught = matches!(outcome, Outcome::Caught(_));
        *slot.borrow_mut() = outcome;
        if let Some(waker) = self.waker.borrow().as_ref().filter(|_| caught) {
            waker.wake_by_ref();
        }
    }
}

impl Future for Watch {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.outcome.strong_count() == 0 {
            return Poll::Ready(());
        }
        if let Poll::Ready(error) = self.scope.as_mut().poll(context) {
            self.record(outcome_of(error));
            return Poll::Ready(());
        }
        let Poll::Ready(refusal) = self.deadline.as_mut().poll(context) else {
            return Poll::Pending;
        };
        self.record(Outcome::Caught(refusal));
        Poll::Ready(())
    }
}

fn outcome_of(error: Option<wgpu::Error>) -> Outcome {
    error.map_or(Outcome::Clean, |error| Outcome::Caught(error.to_string()))
}

#[cfg(test)]
#[path = "painter_surface_scope_watch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "painter_surface_scope_watch_hidden_tests.rs"]
mod hidden_tests;
