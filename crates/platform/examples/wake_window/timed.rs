//! `wake_window`'s timed waits: a frame that returns
//! `Next::WaitUntil(at)` gets one more frame at `at`, and none there if a
//! later frame returned `Wait` before `at`; a time already past gets
//! exactly one more frame.

use std::fmt;
use std::thread::sleep;
use std::time::{Duration, Instant};

use vapors_platform::Waker;

use crate::script::{Shared, frames_over};

/// From the wake to the time asked for: past the woken frame (llvmpipe
/// draws this window in well under 50 ms) and past `REPLACE_AFTER`.
const DELAY: Duration = Duration::from_millis(600);
/// From the first wake to the second, which replaces the time.
const REPLACE_AFTER: Duration = Duration::from_millis(100);
/// After the time, before counting: as `script::BETWEEN`.
const AFTER: Duration = Duration::from_millis(300);

/// How far back the already-passed time is.
const PAST_BY: Duration = Duration::from_millis(500);

pub const EXPECTED: &str = "2 frames for a timed wait (the woken one, then one at or after the time) and 2 for a replaced one, 2 for a time already past (the woken one, then one at once)";

/// The frames counted, and when the timed wait's last frame came
/// relative to the time it asked for.
pub struct Timed {
    reached: u64,
    late_by: Option<Duration>,
    replaced: u64,
    past: u64,
}

impl Timed {
    pub fn held(&self) -> bool {
        self.reached == 2 && self.late_by.is_some() && self.replaced == 2 && self.past == 2
    }
}

impl fmt::Display for Timed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let late = self
            .late_by
            .map_or_else(|| "before the time".into(), |by| format!("{by:?} after it"));
        write!(
            f,
            "a timed wait of {DELAY:?}: {} frames, the last {late}; replaced after {REPLACE_AFTER:?}: {} frames; a time already past: {} frames",
            self.reached, self.replaced, self.past
        )
    }
}

pub fn measure(waker: &Waker, shared: &Shared) -> Result<Timed, String> {
    let mut at = Instant::now();
    let reached = frames_over(shared, || {
        at = Instant::now() + DELAY;
        shared.wait_until(at);
        wake(waker)?;
        sleep(DELAY + AFTER);
        Ok(())
    })?;
    let late_by = shared
        .last_at()
        .and_then(|last| last.checked_duration_since(at));
    let replaced = frames_over(shared, || replace(waker, shared))?;
    let past = frames_over(shared, || past(waker, shared))?;
    Ok(Timed {
        reached,
        late_by,
        replaced,
        past,
    })
}

/// Ask for a time, then wake again before it with nothing asked, so the
/// second frame returns `Wait`; wait past the first time.
fn replace(waker: &Waker, shared: &Shared) -> Result<(), String> {
    shared.wait_until(Instant::now() + DELAY);
    wake(waker)?;
    sleep(REPLACE_AFTER);
    wake(waker)?;
    sleep(DELAY + AFTER);
    Ok(())
}

/// Ask for a time already gone: the woken frame returns it, which asks
/// for another frame at once; that frame returns `Wait`, so no more come.
fn past(waker: &Waker, shared: &Shared) -> Result<(), String> {
    let now = Instant::now();
    shared.wait_until(now.checked_sub(PAST_BY).unwrap_or(now));
    wake(waker)?;
    sleep(AFTER);
    Ok(())
}

fn wake(waker: &Waker) -> Result<(), String> {
    waker.wake().map_err(|closed| closed.to_string())
}
