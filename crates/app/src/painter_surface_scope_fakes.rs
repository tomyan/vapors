//! Test fakes for GPU error scopes: a scope whose result the test
//! gives, and a loop waker that counts its wakes.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use super::LoopWaker;

/// Counts its wakes, as the app's waker would ask the loop for frames.
#[derive(Default)]
pub(crate) struct Wakes(AtomicUsize);

impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// A waker that counts its wakes, as the app's loop waker would.
pub(crate) fn counting_waker() -> (Arc<Wakes>, Waker) {
    let wakes = Arc::new(Wakes::default());
    (Arc::clone(&wakes), Waker::from(wakes))
}

/// A loop waker that counts its wakes, as given by `wake_with`.
pub(crate) fn counting() -> (Arc<Wakes>, LoopWaker) {
    let (count, waker) = counting_waker();
    (count, Rc::new(RefCell::new(Some(waker))))
}

pub(crate) fn woken(wakes: &Wakes) -> usize {
    wakes.0.load(Ordering::SeqCst)
}

/// A scope whose result the test gives, waking whoever polled it, as a
/// WebGPU popErrorScope promise resolving does.
#[derive(Clone, Default)]
pub(crate) struct Late(Rc<RefCell<LateState>>);

#[derive(Default)]
struct LateState {
    /// `Some` once resolved: `Some(None)` is a scope that caught nothing.
    #[allow(clippy::option_option)] // a scope's own output, once it has one
    result: Option<Option<wgpu::Error>>,
    polled_by: Option<Waker>,
}

impl Late {
    pub(crate) fn resolve(&self, error: Option<&str>) {
        let mut state = self.0.borrow_mut();
        state.result = Some(error.map(|e| wgpu::Error::Validation {
            source: e.into(),
            description: e.to_owned(),
        }));
        state.polled_by.take().into_iter().for_each(Waker::wake);
    }
}

impl Future for Late {
    type Output = Option<wgpu::Error>;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.borrow_mut();
        if let Some(result) = state.result.take() {
            return Poll::Ready(result);
        }
        state.polled_by = Some(context.waker().clone());
        Poll::Pending
    }
}
