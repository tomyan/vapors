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

#[derive(Clone, Copy, Debug)]
struct Sample {
    at: Duration,
    position: [f32; 2],
}

/// One finger's recent positions.
#[derive(Debug, Default)]
pub(crate) struct VelocityTracker {
    ring: [Option<Sample>; HISTORY],
    index: usize,
    last: Option<Duration>,
}

impl VelocityTracker {
    /// The finger went down at `position`: a new history (`Action::DOWN`).
    pub(crate) fn down(&mut self, position: [f32; 2], now: Duration) {
        self.clear();
        self.last = None;
        self.add(position, now);
    }

    /// The finger moved (`Action::MOVE`); after 40 ms without a move it
    /// had stopped, and the history starts again.
    pub(crate) fn moved(&mut self, position: [f32; 2], now: Duration) {
        if self
            .last
            .is_some_and(|last| now.saturating_sub(last) >= MOVE_STOPPED)
        {
            self.clear();
        }
        self.add(position, now);
    }

    /// The finger lifted (`Action::UP`, whose position is not added);
    /// 80 ms or more after the last move it had stopped.
    pub(crate) fn up(&mut self, now: Duration) {
        if self
            .last
            .is_some_and(|last| now.saturating_sub(last) >= UP_STOPPED)
        {
            self.clear();
        }
    }

    /// The velocity at the newest sample, pixels per second, each axis
    /// clamped to `max` (`ComputeCurrentVelocity(1000, max)`); zero with
    /// one sample (a fit of degree 0) or no fit.
    pub(crate) fn velocity(&self, max: f32) -> [f32; 2] {
        let samples = self.recent();
        if samples.is_empty() {
            return [0.0; 2];
        }
        let newest = samples[0].at;
        // ages before the newest, negative seconds, as Chromium's
        let times: Vec<f32> = samples
            .iter()
            .map(|s| -newest.saturating_sub(s.at).as_secs_f32())
            .collect();
        let degree = DEGREE.min(samples.len() - 1);
        let axis = |a: usize| {
            let values: Vec<f32> = samples.iter().map(|s| s.position[a]).collect();
            crate::least_squares::slope_at_zero(&times, &values, degree + 1)
        };
        match (axis(0), axis(1)) {
            (Some(x), Some(y)) => [x.clamp(-max, max), y.clamp(-max, max)],
            _ => [0.0; 2],
        }
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

#[cfg(test)]
#[path = "velocity_tracker_tests.rs"]
mod tests;
