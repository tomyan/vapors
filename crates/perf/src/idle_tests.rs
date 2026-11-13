//! The platform's step for each answer, the update test, the table and
//! the verdict's two shapes.

use super::*;

fn counts(settled_after: usize, frames: usize, updates: usize) -> IdleCounts {
    IdleCounts {
        settled_after,
        frames,
        updates,
    }
}

#[test]
fn continue_gets_a_frame_one_interval_later_until_the_end() {
    // Given
    let now = Instant::now();
    let end = now + Duration::from_millis(40);

    // When / Then
    assert_eq!(
        step(Next::Continue, now, end),
        Ok(Step::Frame(now + FRAME_INTERVAL))
    );
    assert_eq!(
        step(Next::Continue, now + Duration::from_millis(32), end),
        Ok(Step::Done)
    );
}

#[test]
fn wait_until_gets_a_frame_at_its_time_within_the_interval_and_never_in_the_past() {
    // Given
    let past = Instant::now();
    let now = past + Duration::from_secs(1);
    let end = now + Duration::from_secs(10);
    let soon = now + Duration::from_secs(2);
    let late = now + Duration::from_secs(11);

    // When / Then
    assert_eq!(step(Next::WaitUntil(soon), now, end), Ok(Step::Frame(soon)));
    assert_eq!(step(Next::WaitUntil(late), now, end), Ok(Step::Done));
    assert_eq!(step(Next::WaitUntil(past), now, end), Ok(Step::Frame(now)));
}

#[test]
fn wait_is_done_and_exit_is_refused() {
    let now = Instant::now();
    assert_eq!(step(Next::Wait, now, now + INTERVAL), Ok(Step::Done));
    assert_eq!(
        step(Next::Exit, now, now + INTERVAL),
        Err("the app exited while idle".to_owned())
    );
}

#[test]
fn an_update_did_work_when_any_count_is_not_zero() {
    // Given
    let mut worked = UpdateCounts::default();
    worked.nodes_checked = 1;

    // When / Then
    assert!(!did_update(UpdateCounts::default()));
    assert!(did_update(worked));
    assert!(is_settled(Next::Wait));
    assert!(is_settled(Next::WaitUntil(Instant::now())));
    assert!(!is_settled(Next::Continue));
}

#[test]
fn the_section_tables_each_count_per_size() {
    // Given
    let measured = [(1000, counts(1, 0, 0)), (3000, counts(1, 2, 1))];

    // When
    let section = section(INTERVAL, &measured);

    // Then
    assert_eq!(
        section,
        "### Idle, 10 s with no input (frames on the app's clock, 16 ms apart)\n\n\
         | | 1000 rows | 3000 rows |\n|---|---|---|\n\
         | frames to settle | 1 | 1 |\n| frames requested | 0 | 2 |\n| updates | 0 | 1 |\n"
    );
}

#[test]
fn a_run_with_no_frames_and_no_updates_passes_naming_the_sizes() {
    let measured = [(1000, counts(1, 0, 0)), (3000, counts(2, 0, 0))];
    assert_eq!(
        verdict(INTERVAL, &measured),
        Ok("idle: pass, no frames requested and no updates in 10 s with no input at 1000, 3000 rows \
            (settled after 1, 2 frames)"
            .to_owned())
    );
}

#[test]
fn a_frame_or_an_update_fails_naming_the_size_the_counts_the_command_and_the_fix() {
    // Given
    let measured = [(1000, counts(1, 0, 0)), (3000, counts(1, 625, 625))];

    // When
    let error = verdict(INTERVAL, &measured).expect_err("fails");

    // Then
    assert_eq!(
        error,
        "idle: the app does work with no input:\n\
         - 3000 rows: 625 frames requested and 625 updates in 10 s after settling in 1 frames\n\
         reproduce: vapors-perf 60 10 --scaling Idle; the fix is to return Next::Wait from a frame \
         that changed nothing and to skip the update when nothing changed \
         (docs/plans/perf-finish-line.md, property 6)"
    );
}

#[test]
fn no_sizes_is_refused() {
    assert_eq!(
        verdict(INTERVAL, &[]),
        Err("idle: no size was measured, so nothing can be judged".to_owned())
    );
}
