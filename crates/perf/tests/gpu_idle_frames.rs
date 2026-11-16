//! GPU tier: the idle measurement (property 6). The rows page left alone
//! asks for no frame and does no update; an app that never settles is
//! refused; an app that asks for a frame after settling is counted, one
//! frame per 16 ms of the interval, and fails the verdict naming it; an
//! app asking for a passed deadline again is refused at the cap; the
//! updates count reads each frame's own update, not the last one's.

use std::time::Duration;

use vapors_gpu_harness::gpu;
use vapors_perf::idle::{IdleCounts, verdict};
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::sampler::Sampler;
use vapors_platform::Next;

#[path = "support/idle_fakes.rs"]
mod idle_fakes;
use idle_fakes::{Answers, Deadlines, Fake};

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
