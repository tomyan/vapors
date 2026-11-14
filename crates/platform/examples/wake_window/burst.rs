//! `wake_window`'s burst: five wakes from one thread draw fewer frames
//! than wakes, because wakes before a frame draw it once. How many
//! frames depends on how far apart the wakes land, which the example
//! measures rather than assumes: a loaded machine can deschedule the
//! waking thread between wakes (#366).
//!
//! A frame after the first needs a wake that came after the frame
//! before it started, so with the wakes spread over `gap` and frames
//! at least `interval` apart, `n` frames need `(n - 2) * interval <
//! gap`: at most `1 + ceil(gap / interval)` frames.

use std::fmt;
use std::thread::sleep;
use std::time::{Duration, Instant};

use vapors_platform::Waker;

use crate::script::{Shared, frames_over};

pub const WAKES: usize = 5;
/// After the last wake, before counting: as `script::BETWEEN`.
const AFTER: Duration = Duration::from_millis(300);
/// Back-to-back pairs measured for the interval; the shortest is kept,
/// so a pair slowed by load cannot widen the bound.
const PAIRS: usize = 3;

/// The burst's frames, the gap from before its first wake to after its
/// last, and the shortest interval between two frames drawn back to back.
pub struct Burst {
    frames: u64,
    gap: Duration,
    interval: Duration,
}

impl Burst {
    /// The most frames the burst may draw, from its gap and interval.
    pub fn most(&self) -> u64 {
        let nanos = self.interval.as_nanos().max(1);
        let spans = self.gap.as_nanos().div_ceil(nanos);
        1 + u64::try_from(spans).unwrap_or(u64::MAX - 1)
    }

    /// `Err` names the gap when the wakes were so far apart that the
    /// bound allows a frame per wake, so the burst shows no coalescing.
    pub fn held(&self) -> Result<bool, String> {
        if self.most() >= WAKES as u64 {
            return Err(format!(
                "refused: the {WAKES} wakes spanned {:?} against a frame interval of {:?}, so they may draw up to {} frames and the burst cannot show wakes coalescing; the waking thread was held up between wakes (machine load?)",
                self.gap,
                self.interval,
                self.most()
            ));
        }
        Ok((1..=self.most()).contains(&self.frames))
    }
}

impl fmt::Display for Burst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a burst of {WAKES} over {:?} (frame interval {:?}): {} frames, at most {}",
            self.gap,
            self.interval,
            self.frames,
            self.most()
        )
    }
}

pub fn measure(waker: &Waker, shared: &Shared) -> Result<Burst, String> {
    let interval = (0..PAIRS)
        .map(|_| back_to_back(waker, shared))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .min()
        .ok_or("no back-to-back pair was measured")?;
    let mut gap = Duration::ZERO;
    let frames = frames_over(shared, || {
        gap = wake_all(waker)?;
        sleep(AFTER);
        Ok(())
    })?;
    Ok(Burst {
        frames,
        gap,
        interval,
    })
}

/// The time between two frames drawn back to back: the woken frame
/// returns a time already past, which asks for the next frame at once.
fn back_to_back(waker: &Waker, shared: &Shared) -> Result<Duration, String> {
    let drawn = frames_over(shared, || {
        shared.wait_until(Instant::now());
        waker.wake().map_err(|closed| closed.to_string())?;
        sleep(AFTER);
        Ok(())
    })?;
    match (drawn, shared.last_two()) {
        (2, Some((before, last))) => Ok(last - before),
        _ => Err(format!(
            "a frame returning a time already past drew {drawn} frames, not 2, so the frame interval could not be measured"
        )),
    }
}

/// Wake `WAKES` times; the time from before the first to after the last.
fn wake_all(waker: &Waker) -> Result<Duration, String> {
    let start = Instant::now();
    for _ in 0..WAKES {
        waker.wake().map_err(|closed| closed.to_string())?;
    }
    Ok(start.elapsed())
}
