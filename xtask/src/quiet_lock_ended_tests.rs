//! When a hold ends: its deadline, its process for a gate or a command,
//! and only one taker after it.

use super::*;

#[test]
fn a_gate_or_command_hold_whose_process_died_is_released_at_once() {
    for form in [Form::Gate, Form::Run] {
        // Given: its process was killed, its deadline an hour off
        let dir = lock_dir("quiet-dead");
        write(&dir, &gone("/w/killed", LONGEST, form)).expect("held");

        // When
        let started = Instant::now();
        let waited = wait_clear(&dir, ("ci", NOBODY), BRIEF, &mut |_| {});

        // Then
        assert_eq!(waited, Ok(()), "{form:?}");
        assert!(started.elapsed() < BRIEF, "{form:?} waited");
    }
}

#[test]
fn a_hold_by_hand_outlives_its_process_until_its_deadline() {
    // Given
    let dir = lock_dir("quiet-held-gone");
    write(&dir, &other("/w/holder", Duration::from_secs(60))).expect("held");

    // When
    let waited = wait_clear(&dir, ("ci", NOBODY), BRIEF, &mut |_| {});

    // Then
    assert!(waited.is_err(), "a hold ended with its process");
}

#[test]
fn of_eight_takers_at_once_after_an_ended_hold_exactly_one_claims_it() {
    // 50 rounds: the race between reading the ended hold and replacing it
    // is narrow, and one round seldom hits it.
    for round in 0..50 {
        // Given
        let dir = lock_dir(&format!("quiet-race-{round}"));
        write(&dir, &gone("/w/old", LONGEST, Form::Run)).expect("ended");

        // When
        let claimed = claim_at_once(&dir, 8);

        // Then
        let won = claimed.iter().filter(|&&won| won).count();
        assert_eq!(won, 1, "round {round}: {claimed:?}");
    }
}

/// `takers` threads claiming the lock in `dir` together: whether each won.
fn claim_at_once(dir: &std::path::Path, takers: usize) -> Vec<bool> {
    let start = std::sync::Arc::new(std::sync::Barrier::new(takers));
    let threads: Vec<_> = (0..takers)
        .map(|n| {
            let (dir, start) = (dir.to_path_buf(), start.clone());
            let holder = Holder::me(&format!("/w/{n}"), LONGEST, Form::Run, "x");
            std::thread::spawn(move || {
                start.wait();
                file::claim(&dir, &holder)
            })
        })
        .collect();
    threads
        .into_iter()
        .map(|taker| taker.join().expect("joined").expect("claim"))
        .collect()
}

#[test]
fn a_hold_past_its_deadline_or_taken_over_is_no_longer_held() {
    // Given
    let dir = lock_dir("quiet-lapsed");
    let held = take(&dir, Holder::me("/w/perf", BRIEF, Form::Run, "perf"), BRIEF).expect("taken");
    assert!(held.still_held(), "held at first");

    // When
    std::thread::sleep(BRIEF);

    // Then
    assert!(!held.still_held(), "held past its deadline");
}

#[test]
fn a_claim_sweeps_files_staged_by_takers_that_have_gone() {
    // Given: a taker that crashed between staging and linking
    let dir = lock_dir("quiet-staged");
    std::fs::create_dir_all(&*dir).expect("dir");
    let left = dir.join(format!("{FILE}.{GONE}.0"));
    std::fs::write(&left, "pid 1 in /w since 1 until 1 as run running x\n").expect("staged");

    // When
    let claimed = file::claim(&dir, &Holder::me("/w/x", BRIEF, Form::Run, "x"));

    // Then
    assert_eq!(claimed, Ok(true));
    assert!(!left.exists(), "{} was left", left.display());
}
