//! `spawn`, `Task` and `Deadline`: a future owned by the current
//! owner, polled by [`crate::run_ready`] only when woken.

use std::fmt::Display;
use std::future::Future;
use std::panic::Location;
use std::time::Duration;

use crate::executor::{self, TaskSlot};
use crate::node::OwnerId;
use crate::runtime::with_runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TaskId {
    pub index: u32,
    pub generation: u32,
}

/// How long a task may wait. Carried by T1 and judged by T3
/// (`docs/plans/async-components.md` section 4). Until T3 lands NO
/// deadline is enforced: a task that pends forever hangs silently,
/// whatever its variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    /// To be refused by T3 once the duration passes; the duration must
    /// come from a measurement named where the constant is declared.
    Measured(Duration),
    /// The user's own action ends the wait (a dialog, a file picker),
    /// so no duration can be measured. T3 will bound it by the owner's
    /// disposal only; nothing else ends it.
    User,
}

/// A handle to a spawned task; its owner's disposal stops it.
#[derive(Debug, Clone, Copy)]
pub struct Task {
    id: crate::task::TaskId,
}

impl Task {
    /// The deadline given to `spawn`; `None` once the task is gone.
    #[must_use]
    pub fn deadline(self) -> Option<Deadline> {
        executor::deadline(self.id)
    }
}

/// Spawns `future` under the current owner (or none). It is queued for
/// its first poll and the wake hook is called, so the loop will call
/// `run_ready`; `spawn` does not poll. The future is not `Send`: it is
/// polled on the thread that spawned it. An `Err` it returns comes
/// back in [`crate::Ran::failures`], naming this call site.
///
/// `deadline` is carried only: nothing enforces it until T3, so a
/// future that never completes hangs without a report.
/// # Panics
/// When the current owner is already disposed (the task would outlive
/// it). Reproduce: `CARGO_INCREMENTAL=0 cargo test -p vapors-signals
/// --test tasks_refusals a_spawn_`.
///
/// With no wake hook installed (call `set_wake_hook` once at startup):
/// nothing would ever run the task. Reproduce: `CARGO_INCREMENTAL=0
/// cargo test -p vapors-signals --test tasks_refusals a_spawn_with_no_hook`.
#[track_caller]
pub fn spawn<E: Display>(
    deadline: Deadline,
    future: impl Future<Output = Result<(), E>> + 'static,
) -> Task {
    let site = Location::caller();
    executor::require_hook(site);

    let owner = with_runtime(|runtime| runtime.current_owner);
    assert!(live(owner), "{}", disposed_owner(site));
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

    executor::wake(id);
    Task { id }
}

fn live(owner: Option<OwnerId>) -> bool {
    let Some(owner) = owner else { return true };
    with_runtime(|runtime| runtime.owner_mut(owner).is_some())
}

fn disposed_owner(site: &Location<'_>) -> String {
    format!(
        "spawn at {site} under an owner that is already disposed: the task would outlive \
         it; spawn under a live owner, or inside `unowned` (reproduce: CARGO_INCREMENTAL=0 \
         cargo test -p vapors-signals --test tasks_refusals a_spawn_under_a_disposed_owner)"
    )
}
