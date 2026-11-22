//! Shared helpers of the task tests.
#![allow(dead_code)]

mod clock;
#[allow(unused_imports)] // not every test binary needs the clock
pub use clock::Clock;

use std::cell::{Cell, RefCell};
use std::future::{Future, poll_fn};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Poll, Waker};

use vapors_signals::set_wake_hook;

pub type Slot = Rc<RefCell<Option<Waker>>>;

pub fn hook() -> Arc<AtomicUsize> {
    let wakes = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&wakes);
    set_wake_hook(move || {
        seen.fetch_add(1, Ordering::SeqCst);
    });
    wakes
}

/// A future that pends, storing its waker, until `done` is set.
pub fn gate(
    slot: &Slot,
    done: &Rc<Cell<bool>>,
    polls: &Rc<Cell<u32>>,
) -> impl Future<Output = Result<(), String>> + use<> {
    let (slot, done, polls) = (Rc::clone(slot), Rc::clone(done), Rc::clone(polls));
    poll_fn(move |cx| {
        polls.set(polls.get() + 1);
        if done.get() {
            return Poll::Ready(Ok(()));
        }
        *slot.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    })
}

pub fn fresh() -> (Slot, Rc<Cell<bool>>, Rc<Cell<u32>>) {
    (
        Slot::default(),
        Rc::new(Cell::new(false)),
        Rc::new(Cell::new(0)),
    )
}

pub struct Guard(pub Rc<Cell<bool>>);

impl Drop for Guard {
    fn drop(&mut self) {
        self.0.set(true);
    }
}
