//! `spawn`, `Task` and `Deadline`: a future owned by the current
//! owner, polled by [`crate::run_ready`] only when woken.

use std::fmt::Display;
use std::future::Future;
use std::panic::Location;
use std::time::Duration;

use crate::executor::{self, TaskSlot};
use crate::runtime::with_runtime;

/// Index plus generation, as for nodes. Plain integers, so a waker
/// carrying one is `Send`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaskId {
    pub index: u32,
    pub generation: u32,
}

/// How long a task may wait. Carried by T1 and judged by T3
/// (`docs/plans/async-components.md` section 4): until then a task is
/// never refused for its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    /// Refused at spawn plus the duration; the duration comes from a
    /// measurement named where the constant is declared.
    Measured(Duration),
    /// The user decides when it ends.
    User,
}

/// A handle to a spawned task. Dropping it does not stop the task; its
/// owner's disposal does (`Task::cancel` comes with the next slice).
#[derive(Debug, Clone, Copy)]
pub struct Task {
    id: TaskId,
}

impl Task {
    /// The deadline given to `spawn`; `None` once the task is gone.
    #[must_use]
    pub fn deadline(&self) -> Option<Deadline> {
        executor::deadline(self.id)
    }
}

/// Spawns `future` under the current owner (or none). It is queued for
/// its first poll, which `run_ready` makes; `spawn` does not poll. The
/// future is not `Send`: it is polled on the thread that spawned it.
/// An `Err` it returns comes back in [`crate::Ran::failures`], naming
/// this call site.
#[track_caller]
pub fn spawn<E: Display>(
    deadline: Deadline,
    future: impl Future<Output = Result<(), E>> + 'static,
) -> Task {
    let site = Location::caller();
    let owner = with_runtime(|runtime| runtime.current_owner);
    let future = Box::pin(async move {
        let error = future.await.err()?;
        Some(format!("task spawned at {site} returned an error: {error}"))
    });
    let id = executor::insert(TaskSlot {
        future: Some(future),
        owner,
        deadline,
    });
    with_runtime(|runtime| {
        if let Some(owner) = owner.and_then(|owner| runtime.owner_mut(owner)) {
            owner.tasks.push(id);
        }
    });
    Task { id }
}
