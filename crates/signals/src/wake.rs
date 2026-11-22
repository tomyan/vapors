//! The part of a task a waker may touch from any thread: the ready
//! list, the wake hook and the waker itself.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::Wake;

use crate::task::TaskId;

pub(crate) type Hook = Arc<dyn Fn() + Send + Sync>;

/// What a waker may touch from any thread.
#[derive(Default)]
pub(crate) struct Shared {
    pub ready: Mutex<Vec<TaskId>>,
    pub hook: Mutex<Option<Hook>>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
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
        let hook = lock(&self.shared.hook).clone();
        let Some(hook) = hook else {
            panic!(
                "a task was woken with no wake hook installed, so nothing would run it: \
                 call set_wake_hook once at startup (reproduce: \
                 CARGO_INCREMENTAL=0 cargo test -p vapors-signals --test tasks a_wake_with_no_hook)"
            );
        };
        lock(&self.shared.ready).push(self.id);
        hook();
    }
}
