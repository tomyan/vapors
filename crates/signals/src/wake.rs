//! The part of a task a waker may touch from any thread: the ready
//! list, the wake hook and the waker itself.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::Wake;

use crate::task::TaskId;

pub(crate) type Hook = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
pub(crate) struct Shared {
    pub ready: Mutex<Vec<TaskId>>,
    pub hook: Mutex<Option<Hook>>,
}

/// Locks, continuing after a poisoned lock. Safe because each guarded
/// value (a `Vec` of ids, an `Option` of a hook) is changed by one
/// push, take or assignment, so a panic never leaves it half-updated.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn notify(shared: &Shared, id: TaskId) {
    requeue(shared, [id]);
}

/// Queues `ids` in order and asks the loop to run them.
pub(crate) fn requeue(shared: &Shared, ids: impl IntoIterator<Item = TaskId>) {
    lock(&shared.ready).extend(ids);
    let hook = lock(&shared.hook).clone();
    let Some(hook) = hook else {
        unreachable!("a task exists only after spawn saw a wake hook")
    };
    hook();
}

pub(crate) struct TaskWaker {
    pub id: TaskId,
    pub shared: Arc<Shared>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        notify(&self.shared, self.id);
    }
}
