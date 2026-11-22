//! A controlled clock whose `sleep` futures wake through the task's
//! own waker, so every wake reaches the hook the host installed.
#![allow(dead_code)]

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

struct Timer {
    id: u32,
    at: Duration,
    waker: Option<Waker>,
}

#[derive(Default)]
struct State {
    now: Duration,
    next: u32,
    timers: Vec<Timer>,
}

#[derive(Clone, Default)]
pub struct Clock(Rc<RefCell<State>>);

impl Clock {
    pub fn now(&self) -> Duration {
        self.0.borrow().now
    }

    /// Timers scheduled and not yet dropped: what a host would wait on.
    pub fn scheduled(&self) -> usize {
        self.0.borrow().timers.len()
    }

    /// The earliest deadline a host would arm, if any.
    pub fn earliest(&self) -> Option<Duration> {
        self.0.borrow().timers.iter().map(|t| t.at).min()
    }

    /// Moves time to `now` and wakes every timer due by then.
    pub fn advance_to(&self, now: Duration) {
        let due: Vec<Waker> = {
            let mut state = self.0.borrow_mut();
            state.now = now;
            let due = state.timers.iter_mut().filter(|t| t.at <= now);
            due.filter_map(|t| t.waker.take()).collect()
        };
        due.into_iter().for_each(Waker::wake);
    }

    pub fn sleep_until(&self, at: Duration) -> Sleep {
        let mut state = self.0.borrow_mut();
        state.next += 1;
        let id = state.next;
        state.timers.push(Timer {
            id,
            at,
            waker: None,
        });
        Sleep {
            clock: self.clone(),
            id,
            at,
        }
    }
}

pub struct Sleep {
    clock: Clock,
    id: u32,
    at: Duration,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.clock.0.borrow_mut();
        if state.now >= self.at {
            return Poll::Ready(());
        }
        let id = self.id;
        if let Some(timer) = state.timers.iter_mut().find(|t| t.id == id) {
            timer.waker = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        let id = self.id;
        self.clock.0.borrow_mut().timers.retain(|t| t.id != id);
    }
}
