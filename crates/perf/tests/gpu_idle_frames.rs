//! GPU tier: the idle measurement (property 6). The rows page left alone
//! asks for no frame and does no update; an app that never settles is
//! refused; an app that asks for a frame after settling is counted, one
//! frame per 16 ms of the interval, and fails the verdict naming it.

use std::time::Duration;

use vapors_app::FrameTiming;
use vapors_gpu_harness::gpu;
use vapors_perf::idle::{IdleCounts, verdict};
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::{Counts, Workload};
use vapors_perf::sampler::Sampler;
use vapors_platform::{App, Event, Frame, Next, Platform};

const TEN_SECONDS: Duration = Duration::from_secs(10);
const ONE_SECOND: Duration = Duration::from_secs(1);

#[test]
fn gpu_the_rows_page_left_alone_asks_for_no_frame_and_does_no_update() {
    // Given
    let gpu = gpu();
    let mut rows = Rows::new(1000, RowChange::Idle);

    // When
    let counts = Sampler::new(&gpu)
        .idle(&mut rows, TEN_SECONDS)
        .expect("the page settles and runs");

    // Then: the mount's frame changes the document (its first update),
    // the second draws it and waits
    assert_eq!(
        counts,
        IdleCounts {
            settled_after: 2,
            frames: 0,
            updates: 0
        }
    );
    assert!(verdict(TEN_SECONDS, &[(1000, counts)]).is_ok());
}

#[test]
fn gpu_an_app_that_never_settles_is_refused_naming_its_last_answer() {
    // Given
    let gpu = gpu();
    let mut spinning = Fake(Answers::Always(Next::Continue));

    // When
    let error = Sampler::new(&gpu)
        .idle(&mut spinning, ONE_SECOND)
        .expect_err("never settles");

    // Then
    assert!(
        error.starts_with("idle: the app had not settled after 5 frames"),
        "{error}"
    );
    assert!(
        error.contains("Continue") && error.contains("reproduce:"),
        "{error}"
    );
}

#[test]
fn gpu_an_app_that_asks_for_frames_after_settling_is_counted_one_per_interval_and_fails() {
    // Given: settled by its first frame with a deadline 100 ms on, then
    // asking at once; the platform runs the deadline's frame and then one
    // per 16 ms, so 1 s holds 1 + 56
    let gpu = gpu();
    let mut settles_then_spins = Fake(Answers::DeadlineThen(Next::Continue));

    // When
    let counts = Sampler::new(&gpu)
        .idle(&mut settles_then_spins, ONE_SECOND)
        .expect("settles, then is driven");

    // Then
    assert_eq!(
        counts,
        IdleCounts {
            settled_after: 1,
            frames: 57,
            updates: 0
        }
    );
    let error = verdict(ONE_SECOND, &[(1000, counts)]).expect_err("fails");
    assert!(
        error.contains("1000 rows: 57 frames requested and 0 updates in 1 s"),
        "{error}"
    );
}

/// What a fake app answers after each frame, drawing nothing.
enum Answers {
    Always(Next),
    /// A deadline 100 ms on from the first frame, then `Always`.
    DeadlineThen(Next),
}

impl App for Answers {
    fn event(&mut self, _: Event, _: &Platform) {}

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match *self {
            Self::Always(next) => next,
            Self::DeadlineThen(next) => {
                *self = Self::Always(next);
                Next::WaitUntil(frame.now + Duration::from_millis(100))
            }
        }
    }
}

/// A workload around a fake app: no change, no counts, no GPU time.
struct Fake(Answers);

impl Workload for Fake {
    fn size(&self) -> (u32, u32) {
        (64, 64)
    }

    fn change(&mut self, _: usize) {}

    fn app(&mut self) -> &mut dyn App {
        &mut self.0
    }

    fn check(&self, _: Next) -> Result<(), String> {
        Ok(())
    }

    fn counts(&self) -> Counts {
        Counts::default()
    }

    fn timing(&self) -> FrameTiming {
        unreachable!("the idle driver reads no frame timing")
    }

    fn times_gpu(&self) -> bool {
        false
    }
}
