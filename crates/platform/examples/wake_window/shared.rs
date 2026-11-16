//! What `wake_window`'s app and script share: when each frame was
//! drawn, what the next frame returns, and the outcome.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vapors_platform::Next;

#[derive(Default)]
pub struct Shared {
    drawn: Mutex<Vec<Instant>>,
    exit: AtomicBool,
    outcome: Mutex<Option<Result<String, String>>>,
    until: Mutex<Option<Instant>>,
}

impl Shared {
    pub fn drew(&self) {
        self.drawn().push(Instant::now());
    }

    /// What the app returns from this frame: `WaitUntil` the time the
    /// script set for this one frame, else `Wait`.
    pub fn next(&self) -> Next {
        let until = self.until.lock().expect("not poisoned").take();
        until.map_or(Next::Wait, Next::WaitUntil)
    }

    /// Make the next frame return `WaitUntil(at)`.
    pub fn wait_until(&self, at: Instant) {
        *self.until.lock().expect("not poisoned") = Some(at);
    }

    /// When the last frame came.
    pub fn last_at(&self) -> Option<Instant> {
        self.drawn().last().copied()
    }

    /// When each frame from the `at`th on came.
    pub fn drawn_since(&self, at: u64) -> Vec<Instant> {
        let drawn = self.drawn();
        let at = usize::try_from(at).unwrap_or(usize::MAX).min(drawn.len());
        drawn[at..].to_vec()
    }

    pub fn exit_asked(&self) -> bool {
        self.exit.load(Ordering::SeqCst)
    }

    pub fn count(&self) -> u64 {
        self.drawn().len() as u64
    }

    /// The outcome; the loop has ended, so the script has finished.
    pub fn outcome(&self) -> Result<String, String> {
        self.outcome
            .lock()
            .expect("not poisoned")
            .clone()
            .unwrap_or_else(|| Err("the script did not finish".into()))
    }

    /// Record `outcome` and ask the app to end the loop at its next
    /// frame (`run` wakes it).
    pub fn finish(&self, outcome: Result<String, String>) {
        *self.outcome.lock().expect("not poisoned") = Some(outcome);
        self.exit.store(true, Ordering::SeqCst);
    }

    /// When the last start-up frame came, after the first.
    pub fn startup_span(&self) -> Duration {
        let drawn = self.drawn();
        drawn
            .first()
            .zip(drawn.last())
            .map_or(Duration::ZERO, |(first, last)| *last - *first)
    }

    fn drawn(&self) -> std::sync::MutexGuard<'_, Vec<Instant>> {
        self.drawn.lock().expect("not poisoned")
    }
}

/// Frames drawn while `during` runs, or its error.
pub fn frames_over(
    shared: &Shared,
    during: impl FnOnce() -> Result<(), String>,
) -> Result<u64, String> {
    let at = shared.count();
    during()?;
    Ok(shared.count() - at)
}
