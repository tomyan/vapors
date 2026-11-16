//! Fling boost, as Chromium's `FlingBooster` decides it
//! (`ui/events/blink/fling_booster.cc`, read at chromium 1555c8bf): a
//! touch fling started while the previous one was still coasting, in the
//! same direction, starts at its own velocity plus the previous fling's
//! starting velocity. The finger going down stops the coast and opens a
//! 50 ms window, which each drag update in the fling's direction at
//! 150 px/s or more extends; the new fling boosts if it comes inside the
//! window, both it and the stopped coast (at its last frame) move at
//! 350 px/s or more, and it points the same way as the previous start.
//!
//! Not ported: `prevent_boosting` on a fling cancel, and the source
//! device and modifier checks (every boosted fling here is a touch's);
//! another input stopping the fling resets the booster instead. As in
//! Chromium, one booster serves the window: a coast in one scroll
//! container can boost a fling in another.
//!
//! RELAXED: the window and the 150 px/s check run on the times events
//! are handled (`DocumentInput` stamps them), not the touches' own, as
//! Chromium's do. A page that stalls between a finger down and its first
//! move can let the window lapse where Chromium would boost, and moves
//! handled together, under 1 ms apart, skip the speed check. Retired by
//! #403 (real touch times).

use std::time::Duration;

use vapors_platform::Instant;

use crate::document_input::{CURVE_SPEED_LIMIT, curve_carries};

/// `kMinBoostFlingSpeedSquare`'s root, CSS px/s.
const MIN_BOOST_FLING_SPEED: f32 = 350.0;
/// `kMinBoostTouchScrollSpeedSquare`'s root, CSS px/s.
const MIN_BOOST_SCROLL_SPEED: f32 = 150.0;
/// `kFlingBoostTimeoutDelay`.
const TIMEOUT: Duration = Duration::from_millis(50);

/// What the booster is shown of the touch gesture stream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Observed {
    /// A finger went down on a running fling (`GestureFlingCancel`).
    FlingCancel,
    /// The drag passed the slop (`GestureScrollBegin`).
    ScrollBegin,
    /// The drag moved the content by this delta (`GestureScrollUpdate`,
    /// not momentum).
    ScrollUpdate([f32; 2]),
    /// The gesture ended (`GestureScrollEnd`).
    ScrollEnd,
    /// A fling is about to start (`GestureFlingStart`, observed before
    /// [`FlingBooster::start`]).
    FlingStart,
}

/// The booster's state between flings.
#[derive(Debug, Default)]
pub(crate) struct FlingBooster {
    /// While set, the gesture stream is a candidate for a boost until it.
    cutoff: Option<Instant>,
    /// The last fling's (possibly boosted) starting velocity; zero: none.
    previous_start: [f32; 2],
    /// That fling's velocity at its last frame that moved.
    current: [f32; 2],
    /// The last drag update counted toward a boost.
    last_boosting_scroll: Option<Instant>,
}

impl FlingBooster {
    /// The velocity a fling starting at `velocity` at `now` runs at
    /// (`GetVelocityForFlingStart`), and the booster set for the next.
    pub(crate) fn start(&mut self, velocity: [f32; 2], now: Instant) -> [f32; 2] {
        let start = if self.boosts(velocity, now) {
            capped(add(velocity, self.previous_start))
        } else {
            velocity
        };
        *self = Self {
            previous_start: start,
            current: start,
            ..Self::default()
        };
        start
    }

    /// The running fling's velocity at a frame that moved it
    /// (`ObserveProgressFling`).
    pub(crate) fn progress(&mut self, velocity: [f32; 2]) {
        if !is_zero(self.previous_start) {
            self.current = velocity;
        }
    }

    /// One event of the touch stream at `now` (`ObserveGestureEvent`).
    pub(crate) fn observe(&mut self, event: Observed, now: Instant) {
        if is_zero(self.previous_start) {
            return;
        }
        if self.cutoff.is_some_and(|cutoff| now > cutoff) {
            self.reset();
            return;
        }
        match event {
            Observed::FlingCancel => {
                self.last_boosting_scroll = None;
                self.cutoff = Some(now + TIMEOUT);
            }
            Observed::ScrollBegin => self.cutoff = Some(now + TIMEOUT),
            Observed::ScrollUpdate(delta) => self.scroll_update(delta, now),
            Observed::ScrollEnd => self.last_boosting_scroll = None,
            Observed::FlingStart => {}
        }
    }

    /// The last fling's starting velocity, zero when there is nothing to
    /// boost from.
    #[cfg(test)]
    pub(crate) fn previous_start(&self) -> [f32; 2] {
        self.previous_start
    }

    /// Nothing to boost from (`Reset`).
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    fn scroll_update(&mut self, delta: [f32; 2], now: Instant) {
        if self.cutoff.is_none() {
            return;
        }
        if dot(self.previous_start, delta) <= 0.0 || self.too_slow(delta, now) {
            self.reset();
            return;
        }
        self.last_boosting_scroll = Some(now);
        self.cutoff = Some(now + TIMEOUT);
    }

    /// Whether the drag since the last boosting update was slower than
    /// 150 px/s; updates under 1 ms apart are not judged.
    fn too_slow(&self, delta: [f32; 2], now: Instant) -> bool {
        let Some(since) = self
            .last_boosting_scroll
            .map(|last| now.saturating_duration_since(last))
        else {
            return false;
        };
        let seconds = since.as_secs_f32();
        seconds >= 0.001
            && length_squared(delta) / (seconds * seconds) < MIN_BOOST_SCROLL_SPEED.powi(2)
    }

    /// `ShouldBoostFling`.
    fn boosts(&self, velocity: [f32; 2], now: Instant) -> bool {
        let fast = |v: [f32; 2]| length_squared(v) >= MIN_BOOST_FLING_SPEED.powi(2);
        !is_zero(self.previous_start)
            && self.cutoff.is_some_and(|cutoff| now <= cutoff)
            && dot(self.previous_start, velocity) > 0.0
            && fast(self.current)
            && fast(velocity)
    }
}

/// `velocity` scaled along its direction down to [`CURVE_SPEED_LIMIT`]
/// when faster. A deviation from Chromium, whose boosted sums are not
/// capped (its curve keeps its precision at any speed): a chain of a
/// dozen boosts at the maximum would pass the speed past which this
/// curve loses its 0.1 px steps, so the chain's fling runs at the limit
/// rather than being refused.
/// Scaling in `f32` can round a hair past the limit; each further step
/// shrinks it by one part in 2^23 until the start's own test
/// (`curve_carries`) passes.
fn capped(velocity: [f32; 2]) -> [f32; 2] {
    if curve_carries(velocity) {
        return velocity;
    }
    let speed = velocity[0].hypot(velocity[1]);
    let mut scaled = velocity.map(|v| v * (CURVE_SPEED_LIMIT / speed));
    while !curve_carries(scaled) {
        scaled = scaled.map(|v| v * (1.0 - f32::EPSILON));
    }
    scaled
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn length_squared(v: [f32; 2]) -> f32 {
    dot(v, v)
}

/// `Vector2dF::IsZero`.
#[allow(clippy::float_cmp, reason = "Chromium's IsZero is an exact test")]
fn is_zero(v: [f32; 2]) -> bool {
    v == [0.0; 2]
}

#[cfg(test)]
#[path = "fling_booster_tests.rs"]
mod tests;
