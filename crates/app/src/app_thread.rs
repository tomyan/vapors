//! The desktop application thread (`docs/plans/desktop-app-thread.md`,
//! L1a). App-authored code runs here, off winit's thread: every handler,
//! and every task continuation after an `.await`. The thread runs a
//! `current_thread` Tokio runtime with its timer, and drives the
//! `vapors_signals` executor inside it, so a task can await a Tokio
//! timer. It takes a turn per message and per coalesced task wake, and
//! none while nothing is pending.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use body::{Body, Ready, Turn};
pub use error::AppThreadError;
use exit::Exit;

/// How long `start` waits for the thread's runtime and the caller's app
/// side. The floor is measured: with an empty app side, 50 starts took
/// 26 µs median and 123 µs at most (debug build, this machine,
/// 2026-11-22, a probe test not kept). The 5 s ceiling is a judgement: a
/// real app side builds more, and no measured app side is near it yet.
/// Re-measured with the first app side that builds a document (L1b).
const START_DEADLINE: Duration = Duration::from_secs(5);

/// How long `stop` waits for the thread to finish its turn, drop its
/// pending tasks and exit. The floor: the same 50 idle threads stopped
/// in 16 µs median and 119 µs at most; with the end learned through a
/// joining thread (`app_thread_exit.rs`), 18 µs and 155 µs, and starts
/// 28 µs and 155 µs (two runs of 50, 2026-11-26, a probe test not kept).
/// The 5 s ceiling is a judgement, as for `START_DEADLINE`.
const STOP_DEADLINE: Duration = Duration::from_secs(5);

/// The two waits. Apart from the constants so the unit tests can prove
/// each deadline without waiting 5 s.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Deadlines {
    pub(crate) start: Duration,
    pub(crate) stop: Duration,
}

const DEADLINES: Deadlines = Deadlines {
    start: START_DEADLINE,
    stop: STOP_DEADLINE,
};

/// The application thread: send it messages, stop it to learn why it
/// ended.
///
/// RELAXED: dropped without `stop`, it is asked to stop and neither
/// waited for nor checked, so a panic it had is seen only on stderr, from
/// the default panic hook. `Drop` cannot return the error, and blocking
/// in a drop would need a deadline nobody hears about. The same holds for
/// a thread whose app side missed `START_DEADLINE`: it is asked to stop
/// and left. Retired when the desktop runner (L1b) owns the thread and
/// calls `stop` when its loop ends, reporting the result.
pub struct AppThread<M: Send + 'static> {
    sender: UnboundedSender<Turn<M>>,
    exit: Option<Exit>,
    turns: Arc<AtomicU64>,
    stop_deadline: Duration,
}

impl<M: Send + 'static> AppThread<M> {
    /// Starts the thread and builds the app side on it with `make`, so
    /// state that is not `Send` never crosses a thread. Each message is
    /// handed to the handler `make` returns; a task's `Err` goes to
    /// `report`, naming its spawn site.
    ///
    /// # Errors
    /// The thread or its runtime could not be made, `make` panicked, or
    /// it did not return within `START_DEADLINE`.
    pub fn start<H: FnMut(M) + 'static>(
        make: impl FnOnce() -> H + Send + 'static,
        report: fn(&str),
    ) -> Result<Self, AppThreadError> {
        Self::start_within(make, report, DEADLINES)
    }

    /// `start`, with `deadlines` in place of the constants.
    pub(crate) fn start_within<H: FnMut(M) + 'static>(
        make: impl FnOnce() -> H + Send + 'static,
        report: fn(&str),
        deadlines: Deadlines,
    ) -> Result<Self, AppThreadError> {
        let (sender, receiver) = unbounded_channel();
        let (ready, readied) = mpsc::channel();
        let turns = Arc::new(AtomicU64::new(0));
        let body = Body::new(receiver, sender.clone(), Arc::clone(&turns), report);
        let thread = std::thread::Builder::new()
            .name("vapors-app".to_owned())
            .spawn(move || body.run(make, &ready))
            .map_err(|error| AppThreadError::Spawn(error.to_string()))?;
        let exit = Exit::watch(thread).inspect_err(|_| {
            // Err: the thread already ended; there is nothing to stop
            let _ = sender.send(Turn::Stop);
        })?;
        let app = AppThread {
            sender,
            exit: Some(exit),
            turns,
            stop_deadline: deadlines.stop,
        };
        app.wait_ready(&readied, deadlines.start)
    }

    fn wait_ready(
        mut self,
        readied: &mpsc::Receiver<Ready>,
        deadline: Duration,
    ) -> Result<Self, AppThreadError> {
        let started = Instant::now();
        match readied.recv_timeout(deadline) {
            Ok(Ok(())) => Ok(self),
            Ok(Err(runtime)) => Err(AppThreadError::Runtime(runtime)),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(self.ended_within(
                "build the app side",
                deadline.saturating_sub(started.elapsed()),
            )),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(AppThreadError::Deadline {
                what: "build the app side",
                after: deadline,
            }),
        }
    }

    /// Hands `message` to the handler, on the application thread.
    ///
    /// # Errors
    /// [`AppThreadError::Stopped`] once the thread has ended; `stop` says why.
    pub fn send(&self, message: M) -> Result<(), AppThreadError> {
        self.sender
            .send(Turn::Message(message))
            .map_err(|_| AppThreadError::Stopped)
    }

    /// The application thread's id.
    ///
    /// # Panics
    /// Never: the handle is taken only by `stop` and by a refused
    /// `start`, and each consumes `self`.
    #[must_use]
    pub fn thread_id(&self) -> ThreadId {
        self.exit.as_ref().expect("held until stop").id()
    }

    /// The turns the thread has taken: one per message and one per
    /// coalesced task wake. Constant while nothing is pending.
    #[must_use]
    pub fn turns(&self) -> u64 {
        self.turns.load(Ordering::SeqCst)
    }

    /// Asks the thread to stop after its current turn and waits for it.
    /// Messages sent before it are handled first. Tasks still pending are
    /// dropped with the thread, unpolled and unreported, as a disposed
    /// owner's tasks are.
    ///
    /// # Errors
    /// [`AppThreadError::Panicked`] with the panic's message when a
    /// handler, a task or the app side panicked; a deadline error when it
    /// did not end within `STOP_DEADLINE`, its pending tasks dropped.
    pub fn stop(mut self) -> Result<(), AppThreadError> {
        // Err: the thread already ended; its join says why
        let _ = self.sender.send(Turn::Stop);
        match self.ended_within("stop", self.stop_deadline) {
            AppThreadError::Stopped => Ok(()),
            error => Err(error),
        }
    }

    /// How the thread ended, once it has, within `deadline`: `Stopped`, its
    /// panic, or a deadline error naming `what` it was doing.
    fn ended_within(&mut self, what: &'static str, deadline: Duration) -> AppThreadError {
        let exit = self.exit.take().expect("waited for once");
        exit.within(deadline).unwrap_or(AppThreadError::Deadline {
            what,
            after: deadline,
        })
    }
}

impl<M: Send + 'static> Drop for AppThread<M> {
    fn drop(&mut self) {
        if self.exit.is_some() {
            // Err: already ended; nothing to ask
            let _ = self.sender.send(Turn::Stop);
        }
    }
}

#[path = "app_thread_body.rs"]
mod body;
#[path = "app_thread_error.rs"]
mod error;
#[path = "app_thread_exit.rs"]
mod exit;
#[cfg(test)]
#[path = "app_thread_tests.rs"]
mod tests;
