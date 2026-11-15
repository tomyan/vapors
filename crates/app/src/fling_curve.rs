//! Chromium's fling curve on Android: `MobileScroller` in fling mode
//! (`ui/events/mobile_scroller.cc`, ported from Android's
//! `OverScroller` spline; docs/plans/touch-momentum.md has the lines
//! and the worked values). A fling at speed `v` runs for
//! `exp(l / (rate − 1))` seconds over
//! `friction × tuning × exp(rate / (rate − 1) × l)` CSS pixels, where
//! `l = ln(inflexion × v / (friction × tuning))`, along the direction of
//! its velocity, and its offset at a time is the spline's position at
//! that fraction of the duration, times the distance.

use std::sync::LazyLock;
use std::time::Duration;

/// `kDefaultFriction`: Android's `ViewConfiguration` scroll friction.
const FRICTION: f64 = 0.015;
/// `kDecelerationRate`: ln(0.78) / ln(0.9).
const DECELERATION_RATE: f64 = 2.358_201_8;
/// `kInflexion`: where the spline's tension lines cross.
const INFLEXION: f64 = 0.35;
/// `ComputeDeceleration(0.84)`: g × 39.37 in/m × 160 px/in × 0.84.
const TUNING: f64 = 9.806_65 * 39.37 * 160.0 * 0.84;
/// `kThresholdForFlingEnd`: closer than this to the end on both axes,
/// the fling ends, CSS px.
const END_THRESHOLD: f64 = 0.1;
/// `NUM_SAMPLES`: samples in the spline's position table.
const SAMPLES: usize = 100;
/// `SAMPLES` as a float.
const SAMPLES_F64: f64 = 100.0;
const START_TENSION: f64 = 0.5;
const END_TENSION: f64 = 1.0;

/// The spline's position at each of `SAMPLES + 1` evenly spaced times
/// (`SplineConstants`): a cubic Bézier in time inverted by bisection,
/// each search starting where the last one ended, as Chromium's does.
static POSITIONS: LazyLock<[f64; SAMPLES + 1]> = LazyLock::new(|| {
    let mut positions = [1.0; SAMPLES + 1];
    let mut x_min = 0.0;
    for (i, position) in positions.iter_mut().take(SAMPLES).enumerate() {
        let x = time_inverse(sample_time(i), &mut x_min);
        *position = 3.0 * x * (1.0 - x) * ((1.0 - x) * START_TENSION + x) + x * x * x;
    }
    positions
});

/// The Bézier parameter at which the spline's time is `alpha`, by
/// bisection from `x_min` up, which it raises as it goes, until within
/// 1e-5 (Chromium's `ApproxEquals`; in f64 this takes about 17 steps).
fn time_inverse(alpha: f64, x_min: &mut f64) -> f64 {
    let (p1, p2) = (
        START_TENSION * INFLEXION,
        1.0 - END_TENSION * (1.0 - INFLEXION),
    );
    let mut x_max = 1.0;
    loop {
        let x = *x_min + (x_max - *x_min) / 2.0;
        let time = 3.0 * x * (1.0 - x) * ((1.0 - x) * p1 + x * p2) + x * x * x;
        if (time - alpha).abs() < 1e-5 {
            return x;
        }
        if time > alpha { x_max = x } else { *x_min = x }
    }
}

#[allow(clippy::cast_precision_loss, reason = "sample indices are at most 100")]
fn sample_time(i: usize) -> f64 {
    i as f64 / SAMPLES_F64
}

/// The fraction of the distance covered at fraction `u` of the
/// duration (`CalculateCoefficients`): the table interpolated linearly.
pub(crate) fn spline_position(u: f64) -> f64 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "u is clamped to 0..1 first, so the index is 0..=100"
    )]
    let index = (u.clamp(0.0, 1.0) * SAMPLES_F64).floor() as usize;
    if index >= SAMPLES {
        return 1.0;
    }
    let (low, high) = (POSITIONS[index], POSITIONS[index + 1]);
    low + (u - sample_time(index)) * (high - low) * SAMPLES_F64
}

/// One fling's curve, from its starting velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FlingCurve {
    duration: f64,
    distance: f64,
    direction: [f64; 2],
}

impl FlingCurve {
    /// The curve for a fling starting at `velocity` (CSS px/s, as a
    /// scroll offset moves); `None` when it is zero or not finite.
    pub(crate) fn new(velocity: [f32; 2]) -> Option<Self> {
        let [x, y] = velocity.map(f64::from);
        let speed = x.hypot(y);
        if !speed.is_finite() || speed == 0.0 {
            return None;
        }
        let l = (INFLEXION * speed / (FRICTION * TUNING)).ln();
        Some(Self {
            duration: (l / (DECELERATION_RATE - 1.0)).exp(),
            distance: FRICTION * TUNING * (DECELERATION_RATE / (DECELERATION_RATE - 1.0) * l).exp(),
            direction: [x / speed, y / speed],
        })
    }

    /// How long the fling runs.
    #[cfg(test)]
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.duration)
    }

    /// How far the fling moves when it runs to its end, CSS px.
    #[cfg(test)]
    pub(crate) fn distance(&self) -> f64 {
        self.distance
    }

    /// The offset from the fling's start at `elapsed`, CSS px; `None`
    /// once the curve has ended (at its duration, or within
    /// `END_THRESHOLD` of its end on both axes).
    pub(crate) fn offset_at(&self, elapsed: Duration) -> Option<[f32; 2]> {
        // from the duration on the spline is 1, so the threshold ends it
        let covered = spline_position(elapsed.as_secs_f64() / self.duration) * self.distance;
        let left = self.distance - covered;
        let ended = self
            .direction
            .iter()
            .all(|d| (left * d).abs() < END_THRESHOLD);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "offsets are page pixels, far inside f32"
        )]
        (!ended).then(|| self.direction.map(|d| (covered * d) as f32))
    }
}

#[cfg(test)]
#[path = "fling_curve_tests.rs"]
mod tests;
