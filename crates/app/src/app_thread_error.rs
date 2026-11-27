//! Why the application thread refused or stopped. Each message names the
//! fix and the reproducing command.

use std::any::Any;
use std::time::Duration;

const REPRODUCE: &str =
    "reproduce: cargo nextest run -p vapors-app --test tests -E 'test(/^app_thread/)'";

/// Why the application thread refused or stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppThreadError {
    /// The thread has ended; [`AppThread::stop`](super::AppThread::stop) says why.
    Stopped,
    /// A handler, a task or the app side panicked, with its message.
    Panicked(String),
    /// The thread's Tokio runtime could not be built.
    // RELAXED: no test reaches it; building a current_thread runtime
    // fails only when the process is out of file descriptors or threads.
    // Retired by a test that lowers RLIMIT_NOFILE in a child process.
    Runtime(String),
    /// The operating system refused the thread.
    // RELAXED: no test reaches it, for the same reason (RLIMIT_NPROC);
    // retired the same way.
    Spawn(String),
    /// The thread did not finish a step within its deadline.
    Deadline {
        /// The step: building the app side, or stopping.
        what: &'static str,
        /// The deadline it missed.
        after: Duration,
    },
}

impl std::fmt::Display for AppThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(
                f,
                "the application thread has stopped; fix: call AppThread::stop, whose error \
                 says why; reproduce: send to the thread again after it stops"
            ),
            Self::Panicked(message) => write!(
                f,
                "the application thread panicked: {message}; fix: the handler, task or app \
                 side that panicked; reproduce: run again with RUST_BACKTRACE=1 to see where"
            ),
            Self::Runtime(message) => write!(
                f,
                "{message}; fix: the thread and file limits of this process; reproduce: \
                 {REPRODUCE}"
            ),
            Self::Spawn(message) => write!(
                f,
                "the application thread could not be started: {message}; fix: the thread \
                 limit of this process; reproduce: {REPRODUCE}"
            ),
            Self::Deadline { what, after } => write!(
                f,
                "the application thread did not {what} within {after:?}: a handler, task or \
                 app side blocked it; fix: move blocking work off the thread; reproduce: run \
                 again under a debugger and read the stack of the thread named vapors-app"
            ),
        }
    }
}

impl std::error::Error for AppThreadError {}

/// A panic payload's text.
pub(super) fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_owned();
    }
    payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| "a panic whose payload is not text".to_owned())
}
