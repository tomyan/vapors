//! Chromium's own tests of `FlingBooster`
//! (`ui/events/blink/fling_booster_unittest.cc`), on the same event
//! streams; the prevent-boosting, modifier and source-device tests are
//! left out with what they test (`fling_booster.rs`).

use std::time::Duration;

use vapors_platform::Instant;

use super::{FlingBooster, Observed};

/// `kEventDelta`.
const STEP: Duration = Duration::from_millis(10);
/// `kFlingBoostTimeoutDelay`.
const TIMEOUT: Duration = Duration::from_millis(50);
const MIN_SCROLL_SPEED: f32 = 150.0;
const MIN_FLING_SPEED: f32 = 350.0;

/// The booster and the stream's clock.
struct Stream {
    booster: FlingBooster,
    now: Instant,
}

impl Stream {
    fn new() -> Self {
        Self {
            booster: FlingBooster::default(),
            now: Instant::now(),
        }
    }

    fn observe(&mut self, event: Observed) {
        self.booster.observe(event, self.now);
    }

    /// `SendFlingStart`: observed, then started.
    fn fling(&mut self, velocity: [f32; 2]) -> [f32; 2] {
        self.observe(Observed::FlingStart);
        self.booster.start(velocity, self.now)
    }

    /// `SimulateBoostingScroll`. The second update's delta is Chromium's
    /// `DeltaFromVelocity`, which divides the velocity by the step
    /// (15 000 px over 10 ms), well above the minimum speed.
    fn boosting_scroll(&mut self) {
        self.now += STEP;
        self.observe(Observed::FlingCancel);
        self.observe(Observed::ScrollEnd);
        self.observe(Observed::ScrollBegin);
        self.now += STEP;
        self.observe(Observed::ScrollUpdate([0.0, 1.0]));
        self.now += STEP;
        self.observe(Observed::ScrollUpdate([
            0.0,
            MIN_SCROLL_SPEED / STEP.as_secs_f32(),
        ]));
    }

    /// `ProgressFling`.
    fn progress(&mut self, velocity: [f32; 2], delta: [f32; 2]) {
        self.observe(Observed::ScrollUpdate(delta));
        self.booster.progress(velocity);
    }
}

#[test]
fn a_fling_during_a_boosting_scroll_adds_the_last_start() {
    // Given: FlingBoostBasic
    let mut stream = Stream::new();
    assert_eq!(
        stream.fling([0.0, 1000.0]),
        [0.0, 1000.0],
        "the first fling is not boosted"
    );

    // When
    stream.boosting_scroll();

    // Then
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 3000.0]);
}

#[test]
fn a_fling_still_at_350_px_a_second_boosts() {
    // Given: FlingBoostProgressedFling
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);
    stream.now += STEP;
    stream.progress([0.0, MIN_FLING_SPEED], [0.0, 20.0]);

    // When
    stream.boosting_scroll();

    // Then
    assert_eq!(stream.fling([0.0, 1100.0]), [0.0, 2100.0]);
}

#[test]
fn an_update_past_the_50_ms_window_ends_the_boost() {
    // Given: NoFlingBoostIfScrollDelayed
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);
    stream.boosting_scroll();

    // When
    stream.now += TIMEOUT + Duration::from_millis(1);
    stream.observe(Observed::ScrollUpdate([0.0, 10_000.0]));

    // Then
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 2000.0]);
}

#[test]
fn a_new_fling_under_350_px_a_second_is_not_boosted() {
    // Given: NoFlingBoostIfBoostTooSlow
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);
    stream.boosting_scroll();

    // When
    let slow = [0.0, MIN_FLING_SPEED - 1.0];

    // Then
    assert_eq!(stream.fling(slow), slow);
}

#[test]
fn a_previous_start_under_350_px_a_second_boosts_nothing() {
    // Given: NoFlingBoostIfPreviousStartVelocityTooSlow
    let mut stream = Stream::new();
    stream.fling([0.0, MIN_FLING_SPEED - 1.0]);

    // When
    stream.boosting_scroll();

    // Then
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 2000.0]);
}

#[test]
fn a_coast_slowed_under_350_px_a_second_boosts_nothing() {
    // Given: NoFlingBoostIfCurrentVelocityTooSlow
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);

    // When
    stream.now += STEP;
    stream.progress([0.0, MIN_FLING_SPEED - 1.0], [0.0, 20.0]);

    // Then
    assert_eq!(stream.fling([0.0, 1100.0]), [0.0, 1100.0]);
}

#[test]
fn a_fling_another_way_is_not_boosted() {
    // Given: NoFlingBoostIfFlingInDifferentDirection
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);
    stream.boosting_scroll();

    // Then
    assert_eq!(stream.fling([1000.0, 0.0]), [1000.0, 0.0]);
}

#[test]
fn a_drag_another_way_ends_the_boost() {
    // Given: NoFlingBoostIfScrollInDifferentDirection
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);
    stream.boosting_scroll();

    // When
    stream.now += STEP;
    stream.observe(Observed::ScrollUpdate([1000.0, 0.0]));

    // Then
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 2000.0]);
}

#[test]
fn a_drag_beginning_after_the_window_does_not_reopen_it() {
    // Given: NoFlingBoostIfScrollBeginPastCutoffTime
    let mut stream = Stream::new();
    assert_eq!(stream.fling([0.0, 1000.0]), [0.0, 1000.0]);

    // When
    stream.now += STEP;
    stream.observe(Observed::FlingCancel);
    stream.observe(Observed::ScrollEnd);
    stream.now += TIMEOUT + Duration::from_millis(1);
    stream.observe(Observed::ScrollBegin);
    stream.now += STEP;
    stream.observe(Observed::ScrollUpdate([0.0, 1.0]));
    stream.now += STEP;
    stream.observe(Observed::ScrollUpdate([
        0.0,
        MIN_SCROLL_SPEED / STEP.as_secs_f32(),
    ]));

    // Then
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 2000.0]);
}

#[test]
fn a_drag_slower_than_150_px_a_second_ends_the_boost() {
    // Given a boosting scroll's start, then updates 10 ms apart moving
    // 1 px (100 px/s)
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);
    stream.now += STEP;
    stream.observe(Observed::FlingCancel);
    stream.observe(Observed::ScrollBegin);
    stream.now += STEP;
    stream.observe(Observed::ScrollUpdate([0.0, 1.0]));

    // When
    stream.now += STEP;
    stream.observe(Observed::ScrollUpdate([0.0, 1.0]));

    // Then
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 2000.0]);
}

#[test]
fn a_fling_with_no_touch_between_is_not_boosted() {
    // Given a fling, and no finger down or drag after it
    let mut stream = Stream::new();
    stream.fling([0.0, 1000.0]);

    // When
    stream.now += STEP;

    // Then: no window was opened (Chromium's "CutoffTimeUnset")
    assert_eq!(stream.fling([0.0, 2000.0]), [0.0, 2000.0]);
}

#[test]
fn a_chain_of_boosts_past_the_curves_limit_runs_at_the_limit() {
    // Given a start of 95 000 px/s (a long chain at the maximum)
    let mut stream = Stream::new();
    stream.fling([0.0, 95_000.0]);
    stream.boosting_scroll();

    // When the next boosts it by 8000 more
    let started = stream.fling([0.0, 8000.0]);

    // Then: capped along its direction, not refused (a deviation from
    // Chromium, declared at the cap)
    assert_eq!(started, [0.0, crate::document_input::CURVE_SPEED_LIMIT]);
}
