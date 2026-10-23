//! `wake_window`'s script, on its own thread: settle, count idle frames,
//! wake singly, wake in a burst, then end the loop with a last wake.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use vapors_platform::Waker;

/// After the first frame, before the idle count. The window's own
/// start-up frames (mapped, sized, exposed) land in this; 2 s is far
/// past the last seen, which `run` prints, so a late one cannot make
/// the idle count flaky. Frames from the window system are not told
/// apart from wakes: the app never sees an expose.
const SETTLE: Duration = Duration::from_secs(2);
/// How long the app is left alone, expecting no frame.
const IDLE: Duration = Duration::from_secs(1);
/// Between wakes: llvmpipe draws this window in well under 50 ms.
const BETWEEN: Duration = Duration::from_millis(300);
const SINGLE_WAKES: usize = 3;
const BURST: usize = 5;

/// What the app and the script share.
#[derive(Default)]
pub struct Shared {
    frames: AtomicU64,
    exit: AtomicBool,
    outcome: Mutex<Option<Result<String, String>>>,
    first_at: Mutex<Option<Instant>>,
    last_at: Mutex<Option<Instant>>,
}

impl Shared {
    pub fn drew(&self) {
        self.frames.fetch_add(1, Ordering::SeqCst);
        let now = Instant::now();
        self.first_at
            .lock()
            .expect("not poisoned")
            .get_or_insert(now);
        *self.last_at.lock().expect("not poisoned") = Some(now);
    }

    pub fn exit_asked(&self) -> bool {
        self.exit.load(Ordering::SeqCst)
    }

    fn count(&self) -> u64 {
        self.frames.load(Ordering::SeqCst)
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
    fn startup_span(&self) -> Duration {
        let first = *self.first_at.lock().expect("not poisoned");
        let last = *self.last_at.lock().expect("not poisoned");
        first.zip(last).map_or(Duration::ZERO, |(f, l)| l - f)
    }
}

/// The whole script, then a wake that ends the loop.
pub fn run(waker: &Waker, shared: &Shared) {
    let outcome = measure(waker, shared);
    shared.finish(outcome);
    if let Err(closed) = waker.wake() {
        eprintln!("wake_window: the last wake failed ({closed}) though the loop should be running");
        std::process::exit(1);
    }
}

fn measure(waker: &Waker, shared: &Shared) -> Result<String, String> {
    std::thread::sleep(SETTLE);
    let startup = (shared.count(), shared.startup_span());
    let idle = frames_over(shared, || {
        std::thread::sleep(IDLE);
        Ok(())
    })?;
    let single = (0..SINGLE_WAKES)
        .map(|_| frames_over(shared, || wake_then_wait(waker, 1)))
        .collect::<Result<Vec<_>, _>>()?;
    let burst = frames_over(shared, || wake_then_wait(waker, BURST))?;
    let line = format!(
        "start-up {} frames over {:?}; idle frames in {IDLE:?}: {idle}; per wake: {single:?}; a burst of {BURST}: {burst}",
        startup.0, startup.1
    );
    let held = idle == 0 && single.iter().all(|n| *n == 1) && (1..=2).contains(&burst);
    if held {
        Ok(line)
    } else {
        Err(format!(
            "{line}; expected 0 idle, 1 per wake and 1 or 2 for the burst"
        ))
    }
}

/// Frames drawn while `during` runs, or its error.
fn frames_over(
    shared: &Shared,
    during: impl FnOnce() -> Result<(), String>,
) -> Result<u64, String> {
    let at = shared.count();
    during()?;
    Ok(shared.count() - at)
}

fn wake_then_wait(waker: &Waker, times: usize) -> Result<(), String> {
    for _ in 0..times {
        waker.wake().map_err(|closed| closed.to_string())?;
    }
    std::thread::sleep(BETWEEN);
    Ok(())
}
