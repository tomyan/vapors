//! A clipboard read on a worker thread with a deadline, for an image or
//! text alike: three answers each, so a read that cannot finish is
//! never taken for an empty clipboard.

use std::sync::mpsc;
use std::time::Duration;

/// A read's answer, which can say why there is none.
pub trait Answer: Send + 'static {
    /// The answer for a read that could not be made, and why.
    fn unreadable(why: String) -> Self;
}

/// One read at a time, each on a worker thread with a deadline. A read
/// past its deadline is left running (arboard has no cancel; on Wayland
/// its pipe read has no timeout), and a new read is refused until it
/// ends, so threads never pile up behind one owner.
#[derive(Debug)]
pub struct Reader<T> {
    pending: Option<mpsc::Receiver<T>>,
}

impl<T> Default for Reader<T> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<T: Answer> Reader<T> {
    /// `read`'s answer within `deadline`, or why there is none.
    pub fn read(&mut self, deadline: Duration, read: fn() -> T) -> T {
        if self.still_reading() {
            return T::unreadable("a clipboard read is still in progress".to_owned());
        }
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("clipboard".to_owned())
            .spawn(move || {
                let _ = sender.send(read());
            });
        if let Err(error) = spawned {
            return T::unreadable(format!("no thread to read the clipboard on: {error}"));
        }
        self.wait(receiver, deadline)
    }

    /// Whether an earlier read has not ended; one that has is dropped
    /// (its answer was for an earlier paste).
    fn still_reading(&mut self) -> bool {
        let running = self
            .pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.try_recv(), Err(mpsc::TryRecvError::Empty)));
        if !running {
            self.pending = None;
        }
        running
    }

    fn wait(&mut self, receiver: mpsc::Receiver<T>, deadline: Duration) -> T {
        match receiver.recv_timeout(deadline) {
            Ok(image) => image,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.pending = Some(receiver);
                T::unreadable(format!("timed out after {} ms", deadline.as_millis()))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                T::unreadable("the clipboard read failed (its thread panicked)".to_owned())
            }
        }
    }
}
