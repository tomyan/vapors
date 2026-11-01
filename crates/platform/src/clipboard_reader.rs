//! A clipboard read on a worker thread with a deadline, for an image or
//! text alike: three answers each, so a read that cannot finish is
//! never taken for an empty clipboard. A read either blocks its caller
//! until its answer or deadline ([`Reader::read`]), or hands its answer
//! to a callback on another thread ([`Reader::start`]), for the event
//! loop, which must not wait on a clipboard's owner.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

/// A read's answer, which can say why there is none.
pub trait Answer: Send + 'static {
    /// The answer for a read that could not be made, and why.
    fn unreadable(why: String) -> Self;
}

/// Where a started read's answer goes, on a thread of its own.
pub type Done<T> = Box<dyn FnOnce(T) + Send>;

/// One read at a time, each on a worker thread with a deadline. A read
/// past its deadline is left running (arboard has no cancel; on Wayland
/// its pipe read has no timeout), and a new read is refused until it
/// ends, so threads never pile up behind one owner.
#[derive(Debug, Default)]
pub struct Reader {
    /// Whether a read's thread has not ended.
    busy: Arc<AtomicBool>,
}

impl Reader {
    /// `read`'s answer within `deadline`, or why there is none; blocks.
    pub fn read<T: Answer>(&mut self, deadline: Duration, read: fn() -> T) -> T {
        let (sender, receiver) = mpsc::channel();
        self.start(
            deadline,
            read,
            Box::new(move |answer| {
                let _ = sender.send(answer);
            }),
        );
        // `done` is always called, by `Once` if not by the read
        receiver
            .recv()
            .unwrap_or_else(|_| T::unreadable("the clipboard read's answer was lost".to_owned()))
    }

    /// Starts `read`; `done` gets its answer, or why there is none, by
    /// `deadline`, on another thread (or at once, when a read is still
    /// in progress or no thread starts).
    pub fn start<T: Answer>(&mut self, deadline: Duration, read: fn() -> T, done: Done<T>) {
        let done = Once(Some(done));
        if self.busy.swap(true, Ordering::SeqCst) {
            return done.give(T::unreadable(
                "a clipboard read is still in progress".to_owned(),
            ));
        }
        let busy = Idle(Arc::clone(&self.busy));
        let started = std::thread::Builder::new()
            .name("clipboard deadline".to_owned())
            .spawn(move || done.give(read_by(deadline, read, busy)));
        if let Err(error) = started {
            // the closure was dropped: `Once` answered `done`, and `Idle`
            // marked the reader idle; this says why
            crate::report_error(&format!("no thread to read the clipboard on: {error}"));
        }
    }
}

/// `read`'s answer from a thread of its own, or why there is none by
/// `deadline`.
fn read_by<T: Answer>(deadline: Duration, read: fn() -> T, busy: Idle) -> T {
    let (sender, receiver) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("clipboard".to_owned())
        .spawn(move || {
            let answer = read();
            // idle before the answer, so the next read is not refused
            drop(busy);
            let _ = sender.send(answer);
        });
    if let Err(error) = spawned {
        return T::unreadable(format!("no thread to read the clipboard on: {error}"));
    }
    match receiver.recv_timeout(deadline) {
        Ok(answer) => answer,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            T::unreadable(format!("timed out after {} ms", deadline.as_millis()))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            T::unreadable("the clipboard read failed (its thread panicked)".to_owned())
        }
    }
}

/// Marks the reader idle when the read's thread ends, by its answer or
/// a panic.
struct Idle(Arc<AtomicBool>);

impl Drop for Idle {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// A `done` that is called exactly once: with the answer, or, when it
/// is dropped unanswered (a thread that did not start, or panicked),
/// with why.
struct Once<T: Answer>(Option<Done<T>>);

impl<T: Answer> Once<T> {
    fn give(mut self, answer: T) {
        if let Some(done) = self.0.take() {
            done(answer);
        }
    }
}

impl<T: Answer> Drop for Once<T> {
    fn drop(&mut self) {
        if let Some(done) = self.0.take() {
            done(T::unreadable(
                "the clipboard read ended without an answer".to_owned(),
            ));
        }
    }
}
