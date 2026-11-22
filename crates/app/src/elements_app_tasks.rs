//! Tasks on a wake (`docs/plans/async-components.md` section 3.3, T2):
//! the executor's wake hook posts a task wake to the loop
//! (`Platform::task_waker`); the wake polls the woken tasks
//! (`App::tasks_woken`) before any frame, and asks for a frame only when
//! the poll changed the document or a task ended. A pending task that is
//! not woken costs no poll and no frame. Every other wake (a surface's,
//! a paste's, a GPU error scope's) still asks for a frame and polls no
//! task.
//!
//! The hook is installed by `ElementsApp::new`, once per thread (the
//! executor is per thread), before the app has seen the loop's
//! platform. A wake before then is kept and posted once the app sees it.
//! A component that spawns while it mounts must be mounted after
//! `ElementsApp::new`: before it there is no hook, and `spawn` refuses.

use std::cell::OnceCell;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vapors_platform::{Instant, Platform, report_error};
use vapors_signals::{run_ready, set_wake_hook};

use super::ElementsApp;
use super::surfaces::scope_wake::{Wrapped, wrapped};

/// Where the hook finds the loop's waker.
enum Loop {
    /// The app has seen no platform yet; `missed` when a wake came.
    Unknown { missed: bool },
    /// The loop's task waker, from the first platform the app saw.
    Known(Wrapped),
}

type Slot = Arc<Mutex<Loop>>;

thread_local! {
    /// The slot of this thread's hook. Every app on the thread shares
    /// it, as they share the thread's executor. It keeps the first
    /// platform's task waker and ignores later ones: one loop runs per
    /// thread (natively `run` once per process, one page on the web),
    /// and `run_ready` polls every task on the thread whichever app
    /// spawned it, so a second loop on the thread could not tell its
    /// tasks apart anyway. A platform that only reads the app's frame
    /// requests, as the tests' second detached one does, is ignored.
    static SLOT: OnceCell<Slot> = const { OnceCell::new() };
}

/// Installs the executor's wake hook on this thread unless an app
/// already has.
///
/// # Panics
/// When something other than an `ElementsApp` installed a hook on this
/// thread first (`set_wake_hook` refuses a second).
pub(super) fn install_hook() {
    SLOT.with(|cell| {
        cell.get_or_init(|| {
            let slot = Arc::new(Mutex::new(Loop::Unknown { missed: false }));
            let hooked = Arc::clone(&slot);
            set_wake_hook(move || wake(&hooked));
            slot
        });
    });
}

/// The hook's body: posts a wake to the loop, or keeps it until the
/// loop is known.
fn wake(slot: &Mutex<Loop>) {
    match &mut *lock(slot) {
        Loop::Known(waker) => post(waker),
        Loop::Unknown { missed } => *missed = true,
    }
}

/// Keeps the loop's task waker from `platform`, the first time the app sees
/// one, and posts the wake kept until then.
pub(super) fn keep_waker(platform: &Platform) {
    let slot = SLOT.with(|cell| cell.get().cloned()).expect(NOT_INSTALLED);
    let mut state = lock(&slot);
    if let Loop::Unknown { missed } = *state {
        let waker = wrapped(platform.task_waker());
        if missed {
            post(&waker);
        }
        *state = Loop::Known(waker);
    }
}

/// Unreachable: only an `ElementsApp`, made after `install_hook`, sees
/// a platform.
const NOT_INSTALLED: &str =
    "ElementsApp::new installs the wake hook before any platform reaches it";

fn post(waker: &Wrapped) {
    // LoopClosed: the window is gone and the loop with it, so no task
    // can be polled again; the process is ending. Nothing to report.
    let _ = waker.wake();
}

/// Locks, continuing after a poisoned lock: the slot changes by one
/// assignment, so a panic never leaves it half-written.
fn lock(slot: &Mutex<Loop>) -> MutexGuard<'_, Loop> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ElementsApp {
    /// `App::tasks_woken`: polls the woken tasks and asks for a frame when
    /// one is owed. A task's `Err` goes to the error report.
    pub(super) fn poll_on_wake(&mut self, platform: &Platform) {
        keep_waker(platform);
        if self.poll_tasks(&mut report_error) {
            platform.request_redraw();
        }
    }

    /// Polls every woken task once, timed as the Tasks phase, and hands
    /// each failure to `report`. True when a frame is owed: the poll
    /// changed the document, or a task ended (completed or failed).
    pub(super) fn poll_tasks(&mut self, report: &mut dyn FnMut(&str)) -> bool {
        let started = Instant::now();
        let ran = run_ready();
        self.tasks_spent += started.elapsed();
        for failure in &ran.failures {
            report(failure);
        }
        let changed = self.document.borrow_mut().take_changed();
        changed || ran.completed > 0
    }
}

#[cfg(test)]
#[path = "elements_app_tasks_tests.rs"]
mod tests;
