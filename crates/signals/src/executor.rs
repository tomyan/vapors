//! The thread-local task arena and the wake hook.
//! Nothing here runs user code (a poll, a drop, the hook) under a borrow.

use std::cell::RefCell;
use std::future::Future;
use std::panic::Location;
use std::pin::Pin;
use std::sync::Arc;

use crate::node::{Arena, OwnerId};
use crate::runtime::with_runtime;
use crate::task::{Deadline, TaskId};
use crate::wake::{Shared, lock, notify};

pub(crate) type Fut = Pin<Box<dyn Future<Output = Option<String>>>>;

pub(crate) struct TaskSlot {
    /// `None` while the task is being polled.
    pub future: Option<Fut>,
    pub owner: Option<OwnerId>,
    pub deadline: Deadline,
}

#[derive(Default)]
pub(crate) struct Executor {
    pub(crate) tasks: Arena<TaskSlot>,
    pub(crate) shared: Arc<Shared>,
}

thread_local! {
    static EXECUTOR: RefCell<Executor> = RefCell::new(Executor::default());
}

pub(crate) fn with_executor<R>(f: impl FnOnce(&mut Executor) -> R) -> R {
    EXECUTOR.with(|executor| f(&mut executor.borrow_mut()))
}

pub(crate) fn shared() -> Arc<Shared> {
    with_executor(|executor| Arc::clone(&executor.shared))
}

/// Installs the function every wake and every spawn calls, once per
/// thread: it asks the loop to call [`run_ready`].
///
/// # Panics
/// On a second call: a hook replaced silently would strand its waiters.
pub fn set_wake_hook(hook: impl Fn() + Send + Sync + 'static) {
    let shared = shared();
    let mut slot = lock(&shared.hook);
    assert!(
        slot.is_none(),
        "set_wake_hook called twice: install the hook once at startup"
    );
    *slot = Some(Arc::new(hook));
}

/// Refuses a spawn with no hook, on the spawning (owning) thread.
pub(crate) fn require_hook(site: &Location<'_>) {
    let installed = lock(&shared().hook).is_some();
    assert!(
        installed,
        "spawn at {site} with no wake hook installed, so nothing would run the task: \
         call set_wake_hook once at startup (reproduce: CARGO_INCREMENTAL=0 \
         cargo test -p vapors-signals --test tasks_refusals a_spawn_with_no_hook)"
    );
}

pub(crate) fn insert(slot: TaskSlot) -> TaskId {
    with_executor(|executor| {
        let (index, generation) = executor.tasks.insert(slot);
        TaskId { index, generation }
    })
}

pub(crate) fn wake(id: TaskId) {
    notify(&shared(), id);
}

pub(crate) fn deadline(id: TaskId) -> Option<Deadline> {
    with_executor(|executor| Some(executor.tasks.get(id.index, id.generation)?.deadline))
}

pub(crate) fn remove_all(ids: &[TaskId]) -> Vec<TaskSlot> {
    with_executor(|executor| {
        let found = ids
            .iter()
            .filter_map(|id| executor.tasks.remove(id.index, id.generation));
        found.collect()
    })
}

pub(crate) fn cancel(id: TaskId) {
    let owner = remove_all(&[id]).first().and_then(|slot| slot.owner);
    forget(id, owner);
}

fn forget(id: TaskId, owner: Option<OwnerId>) {
    with_runtime(|runtime| {
        if let Some(owner) = owner.and_then(|owner| runtime.owner_mut(owner)) {
            owner.tasks.retain(|task| *task != id);
        }
    });
}
