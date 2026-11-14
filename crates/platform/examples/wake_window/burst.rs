//! `wake_window`'s burst in the window: five wakes from one thread draw
//! fewer frames than wakes, because wakes before a frame draw it once.
//! This measures the times; `burst_judge.rs` judges them.

use std::fmt;
use std::thread::sleep;
use std::time::{Duration, Instant};

use vapors_platform::Waker;

use crate::burst_judge::{self, Attempt, Outcome, Pair, WAKES};
use crate::shared::{Shared, frames_over};

/// After the last wake, before counting: as `script::BETWEEN`.
const AFTER: Duration = Duration::from_millis(300);
/// Back-to-back pairs measured for the frame interval.
const PAIRS: usize = 3;

/// The frame interval, when it was measured, each attempt made, and the
/// outcome.
pub struct Burst {
    interval: Option<Duration>,
    attempts: Vec<Attempt>,
    pub outcome: Outcome,
}

pub fn measure(waker: &Waker, shared: &Shared) -> Result<Burst, String> {
    let pairs = (0..PAIRS)
        .map(|_| back_to_back(waker, shared))
        .collect::<Result<Vec<_>, _>>()?;
    let interval = match burst_judge::interval(&pairs) {
        Ok(interval) => interval,
        Err(refused) => return Ok(Burst::not_run(Outcome::Refused(refused))),
    };
    let (attempts, outcome) = burst_judge::judge(interval, || attempt(waker, shared))?;
    Ok(Burst {
        interval: Some(interval),
        attempts,
        outcome,
    })
}

impl Burst {
    fn not_run(outcome: Outcome) -> Self {
        Self {
            interval: None,
            attempts: Vec::new(),
            outcome,
        }
    }
}

impl fmt::Display for Burst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(interval) = self.interval else {
            return write!(f, "a burst of {WAKES}: not run;");
        };
        write!(f, "a burst of {WAKES}:")?;
        for (n, attempt) in self.attempts.iter().enumerate() {
            write!(
                f,
                " attempt {} over {:?} (frame interval {interval:?}): {} frames, at most {};",
                n + 1,
                attempt.gap,
                attempt.frames,
                burst_judge::most(attempt.gap, interval)
            )?;
        }
        Ok(())
    }
}

/// Two frames drawn back to back: the woken frame returns a time
/// already past, which asks for the next frame at once.
fn back_to_back(waker: &Waker, shared: &Shared) -> Result<Pair, String> {
    let at = shared.count();
    let drawn = frames_over(shared, || {
        shared.wait_until(Instant::now());
        waker.wake().map_err(|closed| closed.to_string())?;
        sleep(AFTER);
        Ok(())
    })?;
    let apart = closest(&shared.drawn_since(at));
    Ok(Pair { drawn, apart })
}

/// One burst: `WAKES` wakes, the gap from before the first to after the
/// last, then the frames it drew.
fn attempt(waker: &Waker, shared: &Shared) -> Result<Attempt, String> {
    let at = shared.count();
    let start = Instant::now();
    for _ in 0..WAKES {
        waker.wake().map_err(|closed| closed.to_string())?;
    }
    let gap = start.elapsed();
    sleep(AFTER);
    let drawn = shared.drawn_since(at);
    Ok(Attempt {
        frames: drawn.len() as u64,
        gap,
        closest: closest(&drawn),
    })
}

/// The shortest time between two consecutive frames, if two were drawn.
fn closest(drawn: &[Instant]) -> Option<Duration> {
    drawn.windows(2).map(|w| w[1] - w[0]).min()
}
