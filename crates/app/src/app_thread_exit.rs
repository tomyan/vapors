//! The application thread's end, learned from a second thread that joins
//! it, so `stop` can wait for the end with a deadline. The thread's last
//! turn is not its end: after it, the thread drops its thread-locals,
//! among them the executor's pending tasks and their owners, which is
//! app-authored code, and only a join sees that finish.

use std::sync::mpsc;
use std::thread::{JoinHandle, ThreadId};
use std::time::Duration;

use super::AppThreadError;
use super::error::panic_message;

pub(super) struct Exit {
    id: ThreadId,
    joined: mpsc::Receiver<std::thread::Result<()>>,
}

impl Exit {
    /// Starts the thread that joins `thread`.
    pub(super) fn watch(thread: JoinHandle<()>) -> Result<Self, AppThreadError> {
        let id = thread.thread().id();
        let (sender, joined) = mpsc::channel();
        std::thread::Builder::new()
            .name("vapors-app-join".to_owned())
            // Err: nobody waits for the end any more
            .spawn(move || drop(sender.send(thread.join())))
            .map_err(|error| AppThreadError::Spawn(error.to_string()))?;
        Ok(Self { id, joined })
    }

    pub(super) fn id(&self) -> ThreadId {
        self.id
    }

    /// How the thread ended, `Stopped` or its panic; `None` when it has
    /// not ended within `deadline`.
    pub(super) fn within(&self, deadline: Duration) -> Option<AppThreadError> {
        match self.joined.recv_timeout(deadline) {
            Ok(Ok(())) => Some(AppThreadError::Stopped),
            Ok(Err(payload)) => Some(AppThreadError::Panicked(panic_message(&*payload))),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => Some(AppThreadError::Panicked(
                "the thread joining the application thread ended without its result".to_owned(),
            )),
        }
    }
}
