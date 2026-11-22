//! The thread-local task arena, the ready list and `run_ready`.
//! Nothing here runs user code (a poll, a drop) under a borrow.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::node::{Arena, OwnerId};
use crate::runtime::with_runtime;
use crate::task::{Deadline, TaskId};
use crate::track::untrack;
use crate::wake::{Shared, TaskWaker, lock};

/// A task's future ends in `Some(message)` when the user's future
/// returned an `Err`.
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

/// What one `run_ready` did. Failures must be shown by the caller.
#[derive(Debug, Default)]
#[must_use = "a task's Err is only reported here"]
pub struct Ran {
    /// Tasks polled.
    pub polled: usize,
    /// Tasks that finished, with `Ok` or `Err`.
    pub completed: usize,
    /// One message per `Err`, naming the task's spawn site.
    pub failures: Vec<String>,
}

/// Installs the function every wake calls, once per thread: it asks the
/// loop to call [`run_ready`].
///
/// # Panics
/// On a second call: a hook replaced silently would strand its waiters.
pub fn set_wake_hook(hook: impl Fn() + Send + Sync + 'static) {
    let shared = with_executor(|executor| Arc::clone(&executor.shared));
    let mut slot = lock(&shared.hook);
    assert!(
        slot.is_none(),
        "set_wake_hook called twice: install the hook once at startup"
    );
    *slot = Some(Arc::new(hook));
}

pub(crate) fn insert(slot: TaskSlot) -> TaskId {
    with_executor(|executor| {
        let (index, generation) = executor.tasks.insert(slot);
        let id = TaskId { index, generation };
        lock(&executor.shared.ready).push(id);
        id
    })
}

pub(crate) fn deadline(id: TaskId) -> Option<Deadline> {
    with_executor(|executor| Some(executor.tasks.get(id.index, id.generation)?.deadline))
}

/// Removes tasks, returning them to be dropped after the borrow.
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
/// Inside a batch or a flush: a task would write signals mid-flush.
pub fn run_ready() -> Ran {
    let refused = with_runtime(|runtime| runtime.batch_depth > 0 || runtime.flushing);
    assert!(
        !refused,
        "run_ready called inside a batch or an effect flush: call it from the loop's wake \
         handler (reproduce: CARGO_INCREMENTAL=0 cargo test -p vapors-signals --test tasks refuses)"
    );
    let mut ran = Ran::default();
    for id in drain() {
        poll_one(id, &mut ran);
    }
    ran
}

fn drain() -> Vec<TaskId> {
    let shared = with_executor(|executor| Arc::clone(&executor.shared));
    let mut ids = std::mem::take(&mut *lock(&shared.ready));
    let mut seen = Vec::new();
    ids.retain(|id| {
        let fresh = !seen.contains(&(id.index, id.generation));
        seen.push((id.index, id.generation));
        fresh
    });
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

/// Returns the future to its slot, or drops it when its owner was
/// disposed during the poll.
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
