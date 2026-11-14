//! The burst's judgement from measured times, apart from the window so
//! tests can give it any timing (`tests/wake_burst_judge.rs`).
//!
//! A frame after the first needs a wake that came after the frame
//! before it started, so with the wakes spread over `gap` and frames
//! at least `interval` apart, `n` frames need `(n - 2) * interval <
//! gap`: at most `1 + ceil(gap / interval)` frames. When that reaches
//! `WAKES`, the burst cannot show wakes coalescing; a loaded machine
//! can deschedule the waking thread between wakes (#366), so the burst
//! is tried again, up to `ATTEMPTS` times.

use std::time::Duration;

pub const WAKES: usize = 5;
pub const ATTEMPTS: usize = 3;

/// One back-to-back pair: the frames it drew and, when there were two
/// or more, the time between the last two.
#[derive(Clone, Copy, Debug)]
pub struct Pair {
    pub drawn: u64,
    pub apart: Option<Duration>,
}

/// One burst: the frames it drew, the gap from before its first wake to
/// after its last, and the shortest time between two of its frames.
#[derive(Clone, Copy, Debug)]
pub struct Attempt {
    pub frames: u64,
    pub gap: Duration,
    pub closest: Option<Duration>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Held,
    /// Frames outside the bound: the waker is wrong. Never retried.
    Count(String),
    /// The burst could not be judged; the message names why and the fix.
    Refused(String),
}

/// The frame interval: the shortest pair, so a pair slowed by load
/// cannot narrow the bound. `Err` refuses a pair that did not draw 2
/// frames, and a zero interval.
pub fn interval(pairs: &[Pair]) -> Result<Duration, String> {
    let mut shortest: Option<Duration> = None;
    for pair in pairs {
        let apart = pair.apart.filter(|_| pair.drawn == 2).ok_or_else(|| {
            format!("refused: the burst was not run: a frame returning a time already past drew {} frames, not 2, so the frame interval could not be measured; Next::WaitUntil with a past time must draw one more frame at once (timed.rs, 'a time already past')", pair.drawn)
        })?;
        shortest = Some(shortest.map_or(apart, |s| s.min(apart)));
    }
    match shortest {
        None => Err("refused: the burst was not run: no back-to-back pair was measured".into()),
        Some(zero) if zero.is_zero() => Err("refused: the burst was not run: two frames drawn back to back were 0ns apart, so the frame interval cannot bound the burst; check the frame timestamps (Shared::drew)".into()),
        Some(shortest) => Ok(shortest),
    }
}

/// The most frames a burst over `gap` may draw; `interval` is not zero.
pub fn most(gap: Duration, interval: Duration) -> u64 {
    let spans = gap.as_nanos().div_ceil(interval.as_nanos());
    1 + u64::try_from(spans).unwrap_or(u64::MAX - 1)
}

/// Bursts from `next` until one can be judged, or `ATTEMPTS` spread
/// out. Returns the attempts made and the outcome.
pub fn judge<E>(
    interval: Duration,
    mut next: impl FnMut() -> Result<Attempt, E>,
) -> Result<(Vec<Attempt>, Outcome), E> {
    let mut attempts = Vec::new();
    while attempts.len() < ATTEMPTS {
        let attempt = next()?;
        attempts.push(attempt);
        if let Some(outcome) = judge_one(attempt, interval) {
            return Ok((attempts, outcome));
        }
    }
    let refused = spread_out(&attempts, interval);
    Ok((attempts, Outcome::Refused(refused)))
}

/// `None` when the attempt spread out and may be retried.
fn judge_one(attempt: Attempt, interval: Duration) -> Option<Outcome> {
    if let Some(closest) = attempt.closest.filter(|c| *c < interval) {
        return Some(Outcome::Refused(format!(
            "refused: two of the burst's frames were {closest:?} apart, under the frame interval of {interval:?} measured on the WaitUntil path, so that interval does not bound frames on the wake path; measure the interval from woken frames"
        )));
    }
    let most = most(attempt.gap, interval);
    if most >= WAKES as u64 {
        return None;
    }
    if (1..=most).contains(&attempt.frames) {
        Some(Outcome::Held)
    } else {
        Some(Outcome::Count(format!(
            "expected 1 to {most} frames for the burst, got {}",
            attempt.frames
        )))
    }
}

fn spread_out(attempts: &[Attempt], interval: Duration) -> String {
    let gaps: Vec<Duration> = attempts.iter().map(|a| a.gap).collect();
    format!(
        "refused: each of {} bursts of {WAKES} wakes spread over more than {} frame intervals of {interval:?} (gaps {gaps:?}), so up to a frame per wake was allowed and the burst could not show wakes coalescing; the waking thread was held up between wakes: re-run on a quieter machine",
        attempts.len(),
        WAKES - 2
    )
}
