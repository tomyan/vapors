//! The desktop application thread (`docs/plans/desktop-app-thread.md`,
//! L1a). App-authored code runs here, off winit's thread: every handler,
//! and every task continuation after an `.await`. The thread runs a
//! `current_thread` Tokio runtime with its timer, and drives the
//! `vapors_signals` executor inside it, so a task can await a Tokio
//! timer. It takes a turn per message and per coalesced task wake, and
//! none while nothing is pending.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{JoinHandle, ThreadId};
use std::time::Duration;

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use body::{Body, Ready, Turn};
pub use error::AppThreadError;

/// How long `start` waits for the thread's runtime and app side. With an
/// empty app side, 50 starts took 26 µs median and 123 µs at most (debug
/// build, this machine, 2026-11-22, a probe test since removed); 5 s is
/// reached only by an app side that hangs.
const START_DEADLINE: Duration = Duration::from_secs(5);

/// How long `stop` waits for the thread to finish its turn and exit. The
/// same 50 idle threads stopped in 16 µs median and 119 µs at most; 5 s
/// is reached only by a handler or task that blocks the thread.
const STOP_DEADLINE: Duration = Duration::from_secs(5);

/// The application thread: send it messages, stop it to learn why it
/// ended. Dropped without `stop`, it is asked to stop and not waited for.
pub struct AppThread<M: Send + 'static> {
    sender: UnboundedSender<Turn<M>>,
    thread: Option<JoinHandle<()>>,
    turns: Arc<AtomicU64>,
    exited: mpsc::Receiver<()>,
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
        let (sender, receiver) = unbounded_channel();
        let (ready, readied) = mpsc::channel();
        let (exit, exited) = mpsc::channel();
        let turns = Arc::new(AtomicU64::new(0));
        let body = Body::new(receiver, sender.clone(), Arc::clone(&turns), report);
        let thread = std::thread::Builder::new()
            .name("vapors-app".to_owned())
            .spawn(move || body.run(make, &ready, exit))
            .map_err(|error| AppThreadError::Spawn(error.to_string()))?;
        let app = AppThread {
            sender,
            thread: Some(thread),
            turns,
            exited,
        };
        app.wait_ready(&readied)
    }

    fn wait_ready(mut self, readied: &mpsc::Receiver<Ready>) -> Result<Self, AppThreadError> {
        match readied.recv_timeout(START_DEADLINE) {
            Ok(Ok(())) => Ok(self),
            Ok(Err(runtime)) => Err(AppThreadError::Runtime(runtime)),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(self.join()),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(AppThreadError::Deadline {
                what: "build the app side",
                after: START_DEADLINE,
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
    /// Never: the handle is only taken by `stop`, which consumes `self`.
    #[must_use]
    pub fn thread_id(&self) -> ThreadId {
        self.thread.as_ref().expect("held until stop").thread().id()
    }

    /// The turns the thread has taken: one per message and one per
    /// coalesced task wake. Constant while nothing is pending.
    #[must_use]
    pub fn turns(&self) -> u64 {
        self.turns.load(Ordering::SeqCst)
    }

    /// Asks the thread to stop after its current turn and waits for it.
    ///
    /// # Errors
    /// [`AppThreadError::Panicked`] with the panic's message when a
    /// handler, a task or the app side panicked; a deadline error when it
    /// did not exit within `STOP_DEADLINE`.
    pub fn stop(mut self) -> Result<(), AppThreadError> {
        // Err: the thread already ended; its join says why
        let _ = self.sender.send(Turn::Stop);
        match self.exited.recv_timeout(STOP_DEADLINE) {
            Err(mpsc::RecvTimeoutError::Timeout) => Err(AppThreadError::Deadline {
                what: "stop",
                after: STOP_DEADLINE,
            }),
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => match self.join() {
                AppThreadError::Stopped => Ok(()),
                error => Err(error),
            },
        }
    }

    /// Joins the exited thread: `Stopped` when it ended normally, else
    /// its panic.
    fn join(&mut self) -> AppThreadError {
        let thread = self.thread.take().expect("joined once");
        match thread.join() {
            Ok(()) => AppThreadError::Stopped,
            Err(payload) => AppThreadError::Panicked(error::panic_message(&*payload)),
        }
    }
}

impl<M: Send + 'static> Drop for AppThread<M> {
    fn drop(&mut self) {
        if self.thread.is_some() {
            // Err: already ended; nothing to ask
            let _ = self.sender.send(Turn::Stop);
        }
    }
}

#[path = "app_thread_body.rs"]
mod body;
#[path = "app_thread_error.rs"]
mod error;
