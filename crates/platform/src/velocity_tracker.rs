//! A finger's velocity as Chromium's gesture detector measures it: its
//! default strategy, LSQ2 (`ui/events/velocity_tracker/
//! velocity_tracker.cc`, read at chromium 1555c8bf): a second-degree
//! least-squares fit, unweighted, over the last 20 samples within 100 ms
//! of the newest, its slope at the newest sample. Samples 40 ms apart
//! mean the finger stopped between them, and an up 80 ms after the last
//! move means it stopped before lifting: either clears the history.
//!
//! One finger; Chromium's tracker holds every pointer in one history.
//! The history is the same ring: after a clear it holds 19 samples until
//! it wraps, as Chromium's does (`Clear` empties slot 0 only).
//!
//! RELAXED: samples carry the time the platform handled the touch, not
//! the touch's own time (winit 0.30 gives touches none; `Input::track`
//! stamps them). The 40 ms and 80 ms rules then also fire when the page
//! did not handle input for that long, and moves handled together share
//! one time. A clear followed by moves handled together is that stall,
//! not a stopped finger, and same-time samples can leave no fit: both
//! answer [`Velocity::Unresolvable`]. An up handled 80 ms late after a
//! stall alone cannot be told from a finger held still, and answers
//! stopped. Retired by #403 (real touch times; upstream
//! rust-windowing/winit#1194).

use std::time::Duration;

/// `kHistorySize`.
const HISTORY: usize = 20;
/// `kHorizonMS`.
const HORIZON: Duration = Duration::from_millis(100);
/// `kAssumePointerMoveStoppedTimeMs`.
const MOVE_STOPPED: Duration = Duration::from_millis(40);
/// `kAssumePointerUpStoppedTimeMs`.
const UP_STOPPED: Duration = Duration::from_millis(80);
/// `Estimator` degree for LSQ2.
const DEGREE: usize = 2;
/// Samples closer than this were handled together, not sampled apart.
const TOGETHER: Duration = Duration::from_millis(1);

#[derive(Clone, Copy, Debug)]
struct Sample {
    at: Duration,
    position: [f32; 2],
}

/// What the tracker can say of the finger's velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Velocity {
    /// Moving at this velocity, pixels per second.
    Moving([f32; 2]),
    /// Stopped: one sample or none (a stop rule cleared the history).
    Stopped,
    /// Not measurable from these samples.
    Unresolvable(Unresolvable),
}

/// Why the velocity could not be measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unresolvable {
    /// A 40 ms gap cleared the history and the moves after it were
    /// handled together: the page stalled, the finger may not have.
    Stalled,
    /// The samples' times leave the fit no solution (moves handled
    /// together share one time).
    Degenerate,
}

/// One finger's recent positions.
#[derive(Debug, Default)]
pub(crate) struct VelocityTracker {
    ring: [Option<Sample>; HISTORY],
    index: usize,
    last: Option<Duration>,
    /// The history was last cleared by the 40 ms move rule.
    gap_cleared: bool,
}

impl VelocityTracker {
    /// The finger went down at `position`: a new history (`Action::DOWN`).
    pub(crate) fn down(&mut self, position: [f32; 2], now: Duration) {
        *self = Self::default();
        self.add(position, now);
    }

    /// The finger moved (`Action::MOVE`); after 40 ms without a move it
    /// had stopped, and the history starts again.
    pub(crate) fn moved(&mut self, position: [f32; 2], now: Duration) {
        if self.since_last(now) >= Some(MOVE_STOPPED) {
            self.clear();
            self.gap_cleared = true;
        }
        self.add(position, now);
    }

    /// The finger lifted (`Action::UP`, whose position is not added);
    /// 80 ms or more after the last move it had stopped.
    pub(crate) fn up(&mut self, now: Duration) {
        if self.since_last(now) >= Some(UP_STOPPED) {
            self.clear();
        }
    }

    /// The velocity at the newest sample, each axis clamped to `max`
    /// (`ComputeCurrentVelocity(1000, max)`).
    pub(crate) fn velocity(&self, max: f32) -> Velocity {
        let samples = self.recent();
        if samples.len() < 2 {
            return Velocity::Stopped;
        }
        if self.gap_cleared && handled_together(&samples) {
            return Velocity::Unresolvable(Unresolvable::Stalled);
        }
        match fit(&samples) {
            Some([x, y]) => Velocity::Moving([x.clamp(-max, max), y.clamp(-max, max)]),
            None => Velocity::Unresolvable(Unresolvable::Degenerate),
        }
    }

    fn since_last(&self, now: Duration) -> Option<Duration> {
        self.last.map(|last| now.saturating_sub(last))
    }

    /// The samples the fit uses, newest first: back from the newest
    /// until an empty slot, `HISTORY` of them, or one older than the
    /// horizon.
    fn recent(&self) -> Vec<Sample> {
        let Some(newest) = self.ring[self.index] else {
            return Vec::new();
        };
        let mut found = Vec::with_capacity(HISTORY);
        let mut index = self.index;
        while let Some(sample) = self.ring[index].filter(|_| found.len() < HISTORY) {
            if newest.at.saturating_sub(sample.at) > HORIZON {
                break;
            }
            found.push(sample);
            index = (index + HISTORY - 1) % HISTORY;
        }
        found
    }

    fn add(&mut self, position: [f32; 2], now: Duration) {
        self.index = (self.index + 1) % HISTORY;
        self.ring[self.index] = Some(Sample { at: now, position });
        self.last = Some(now);
    }

    fn clear(&mut self) {
        self.index = 0;
        self.ring[0] = None;
    }
}

/// Whether any two of `samples` (newest first) were handled together.
fn handled_together(samples: &[Sample]) -> bool {
    samples
        .windows(2)
        .any(|pair| pair[0].at.saturating_sub(pair[1].at) < TOGETHER)
}

/// The fitted slope at the newest sample on each axis: degree 2, or one
/// less than the samples; `None` when the fit has no solution.
fn fit(samples: &[Sample]) -> Option<[f32; 2]> {
    let newest = samples[0].at;
    // ages before the newest, negative seconds, as Chromium's
    let times: Vec<f32> = samples
        .iter()
        .map(|s| -newest.saturating_sub(s.at).as_secs_f32())
        .collect();
    let count = DEGREE.min(samples.len() - 1) + 1;
    let axis = |a: usize| {
        let values: Vec<f32> = samples.iter().map(|s| s.position[a]).collect();
        crate::least_squares::slope_at_zero(&times, &values, count)
    };
    Some([axis(0)?, axis(1)?])
}

#[cfg(test)]
#[path = "velocity_tracker_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "velocity_tracker_rules_tests.rs"]
mod rules_tests;
