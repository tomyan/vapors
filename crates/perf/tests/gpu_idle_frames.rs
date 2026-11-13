//! GPU tier: the idle measurement (property 6). The rows page left alone
//! asks for no frame and does no update; an app that never settles is
//! refused; an app that asks for a frame after settling is counted, one
//! frame per 16 ms of the interval, and fails the verdict naming it; an
//! app asking for a passed deadline again is refused at the cap; the
//! updates count reads each frame's own update, not the last one's.

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
const REPRODUCE: &str = "vapors-perf 60 10 --scaling Idle";

#[test]
fn gpu_the_rows_page_left_alone_asks_for_no_frame_and_does_no_update() {
    // Given
    let gpu = gpu();
    let mut rows = Rows::new(1000, RowChange::Idle);

    // When
    let counts = Sampler::new(&gpu)
        .idle(&mut rows, TEN_SECONDS, REPRODUCE)
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
    assert!(verdict(TEN_SECONDS, &[(1000, counts)], REPRODUCE).is_ok());
}

#[test]
fn gpu_an_app_that_never_settles_is_refused_naming_its_last_answer() {
    // Given
    let gpu = gpu();
    let mut spinning = Fake(Answers::Always(Next::Continue));

    // When
    let error = Sampler::new(&gpu)
        .idle(&mut spinning, ONE_SECOND, REPRODUCE)
        .expect_err("never settles");

    // Then
    assert!(
        error.starts_with("idle: the app had not settled after 5 frames"),
        "{error}"
    );
    assert!(
        error.contains("Continue")
            && error.ends_with("reproduce: vapors-perf 60 10 --scaling Idle"),
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
        .idle(&mut settles_then_spins, ONE_SECOND, REPRODUCE)
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
    let error = verdict(ONE_SECOND, &[(1000, counts)], REPRODUCE).expect_err("fails");
    assert!(
        error.contains("1000 rows: 57 frames requested and 0 updates in 1 s"),
        "{error}"
    );
}

#[test]
fn gpu_an_app_asking_for_a_passed_deadline_again_is_refused_at_the_cap() {
    // Given: every frame asks for a frame at its own time, which never
    // moves the clock; the cap for 1 s is 62 frames
    let gpu = gpu();
    let mut stuck = Fake(Answers::DeadlineNow);

    // When
    let error = Sampler::new(&gpu)
        .idle(&mut stuck, ONE_SECOND, REPRODUCE)
        .expect_err("refused at the cap");

    // Then
    assert!(
        error.starts_with("idle: the app asked for more than 62 frames in 1 s"),
        "{error}"
    );
    assert!(
        error.contains("WaitUntil")
            && error.ends_with("reproduce: vapors-perf 60 10 --scaling Idle"),
        "{error}"
    );
}

#[test]
fn gpu_a_frame_whose_update_changed_a_row_counts_as_an_update() {
    // Given: the rows page settles, then is edited before each frame
    // (Text); the first deadline's frame edits, and an edited document
    // asks for the next frame at once, so 1 s holds 1 + 56 frames, each
    // an update
    let gpu = gpu();
    let mut edited = Deadlines::new(Rows::new(1000, RowChange::Text));

    // When
    let counts = Sampler::new(&gpu)
        .idle(&mut edited, ONE_SECOND, REPRODUCE)
        .expect("settles, then is driven");

    // Then
    assert_eq!(
        counts,
        IdleCounts {
            settled_after: 2,
            frames: 57,
            updates: 57
        }
    );
}

#[test]
fn gpu_a_frame_that_changed_nothing_counts_no_update_whatever_the_last_update_did() {
    // Given: the same deadlines on the unchanged page; the settling
    // updates did work, the ten frames after do not
    let gpu = gpu();
    let mut unchanged = Deadlines::new(Rows::new(1000, RowChange::Idle));

    // When
    let counts = Sampler::new(&gpu)
        .idle(&mut unchanged, ONE_SECOND, REPRODUCE)
        .expect("settles, then is driven");

    // Then
    assert_eq!(
        counts,
        IdleCounts {
            settled_after: 2,
            frames: 10,
            updates: 0
        }
    );
}

/// What a fake app answers after each frame, drawing nothing.
enum Answers {
    Always(Next),
    /// A deadline 100 ms on from the first frame, then `Always`.
    DeadlineThen(Next),
    /// A deadline at the frame's own time, every frame.
    DeadlineNow,
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
            Self::DeadlineNow => Next::WaitUntil(frame.now),
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

/// The rows page whose app asks for a frame 100 ms after any it would
/// have waited on, so the driver runs frames after settling; its change
/// is made only from the third frame, after the page has settled.
struct Deadlines(Rows);

impl Deadlines {
    fn new(rows: Rows) -> Self {
        Self(rows)
    }
}

/// Frames before this are the settling ones; no change is made before them.
const EDIT_FROM: usize = 2;

impl App for Deadlines {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.0.app().event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match self.0.app().frame(frame) {
            Next::Wait => Next::WaitUntil(frame.now + Duration::from_millis(100)),
            other => other,
        }
    }
}

impl Workload for Deadlines {
    fn size(&self) -> (u32, u32) {
        self.0.size()
    }

    fn change(&mut self, sample: usize) {
        if sample >= EDIT_FROM {
            self.0.change(sample);
        } else {
            self.0.take_ax_update();
        }
    }

    fn app(&mut self) -> &mut dyn App {
        self
    }

    fn check(&self, next: Next) -> Result<(), String> {
        self.0.check(next)
    }

    fn counts(&self) -> Counts {
        self.0.counts()
    }

    fn timing(&self) -> FrameTiming {
        self.0.timing()
    }
}
