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
    Runtime(String),
    /// The operating system refused the thread.
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
                "the application thread has stopped; fix: call AppThread::stop to learn why"
            ),
            Self::Panicked(message) => write!(
                f,
                "the application thread panicked: {message}; fix: the panicking handler or task ({REPRODUCE})"
            ),
            Self::Runtime(message) => write!(
                f,
                "{message}; fix: the thread's resources (open files, threads) on this machine"
            ),
            Self::Spawn(message) => write!(
                f,
                "the application thread could not be started: {message}; fix: the process's thread limit"
            ),
            Self::Deadline { what, after } => write!(
                f,
                "the application thread did not {what} within {after:?}: a handler, task or app side blocked it; fix: move blocking work off the thread ({REPRODUCE})"
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
