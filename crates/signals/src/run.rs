//! `run_ready`: polls the woken tasks. Nothing here runs user code (a
//! poll, a drop, the hook) under a borrow.

use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::executor::{Fut, cancel, shared, with_executor};
use crate::task::TaskId;
use crate::track::untrack;
use crate::wake::{TaskWaker, lock};

/// What one `run_ready` did. Failures must be shown by the caller.
#[derive(Debug, Default)]
#[must_use = "a task's Err is only reported here"]
pub struct Ran {
    /// Tasks polled.
    pub polled: usize,
    /// Tasks finished.
    pub completed: usize,
    /// One message per `Err`, naming the spawn site.
    pub failures: Vec<String>,
}

/// Polls every woken task once, with nothing tracking reads. A panic
/// in a poll goes on to the caller.
pub fn run_ready() -> Ran {
    let mut ran = Ran::default();
    let mut waiting = drain();
    while let Some(id) = waiting.pop() {
        poll_one(id, &mut ran);
    }
    ran
}

fn drain() -> Vec<TaskId> {
    let mut ids = std::mem::take(&mut *lock(&shared().ready));
    ids.reverse();
    ids
}

fn take_future(id: TaskId) -> Option<(Fut, Waker)> {
    with_executor(|executor| {
        let future = executor
            .tasks
            .get_mut(id.index, id.generation)?
            .future
            .take()?;
        let shared = Arc::clone(&executor.shared);
        Some((future, Waker::from(Arc::new(TaskWaker { id, shared }))))
    })
}

fn poll_one(id: TaskId, ran: &mut Ran) {
    let Some((mut future, waker)) = take_future(id) else {
        return;
    };
    ran.polled += 1;
    let poll = untrack(|| future.as_mut().poll(&mut Context::from_waker(&waker)));
    match poll {
        Poll::Pending => put_back(id, future),
        Poll::Ready(failure) => {
            ran.completed += 1;
            ran.failures.extend(failure);
            cancel(id);
        }
    }
}

fn put_back(id: TaskId, future: Fut) {
    let orphan = with_executor(
        |executor| match executor.tasks.get_mut(id.index, id.generation) {
            Some(slot) => {
                slot.future = Some(future);
                None
            }
            None => Some(future),
        },
    );
    drop(orphan);
}
