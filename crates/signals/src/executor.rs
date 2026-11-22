//! The thread-local task arena, the ready list and `run_ready`.
//! Nothing here runs user code (a poll, a drop, the hook) under a borrow.

use std::cell::RefCell;
use std::collections::HashSet;
use std::future::Future;
use std::panic::Location;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::node::{Arena, OwnerId};
use crate::runtime::with_runtime;
use crate::task::{Deadline, TaskId};
use crate::track::untrack;
use crate::wake::{Shared, TaskWaker, lock, notify};

type Fut = Pin<Box<dyn Future<Output = Option<String>>>>;

pub(crate) struct TaskSlot {
    /// `None` while the task is being polled.
    pub future: Option<Fut>,
    pub owner: Option<OwnerId>,
    pub deadline: Deadline,
}

#[derive(Default)]
struct Executor {
    tasks: Arena<TaskSlot>,
    shared: Arc<Shared>,
}

thread_local! {
    static EXECUTOR: RefCell<Executor> = RefCell::new(Executor::default());
}

fn with_executor<R>(f: impl FnOnce(&mut Executor) -> R) -> R {
    EXECUTOR.with(|executor| f(&mut executor.borrow_mut()))
}

fn shared() -> Arc<Shared> {
    with_executor(|executor| Arc::clone(&executor.shared))
}

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

/// Polls every woken task once, with nothing tracking reads.
///
/// # Panics
/// Inside a batch or an effect flush: a task would write signals
/// mid-flush. Reproduce: `CARGO_INCREMENTAL=0 cargo test -p
/// vapors-signals --test tasks_refusals run_ready_inside`.
pub fn run_ready() -> Ran {
    let (batching, flushing) = with_runtime(|runtime| (runtime.batch_depth > 0, runtime.flushing));
    assert!(
        !batching,
        "run_ready called inside a batch: call it from the loop's wake handler \
         (reproduce: CARGO_INCREMENTAL=0 cargo test -p vapors-signals --test tasks_refusals \
         run_ready_inside_a_batch_refuses)"
    );
    assert!(
        !flushing,
        "run_ready called inside an effect flush: call it from the loop's wake handler \
         (reproduce: CARGO_INCREMENTAL=0 cargo test -p vapors-signals --test tasks_refusals \
         run_ready_inside_an_effect_flush_refuses)"
    );
    let mut ran = Ran::default();
    for id in drain() {
        poll_one(id, &mut ran);
    }
    ran
}

fn drain() -> Vec<TaskId> {
    let mut ids = std::mem::take(&mut *lock(&shared().ready));
    let mut seen = HashSet::new();
    ids.retain(|id| seen.insert(*id));
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
