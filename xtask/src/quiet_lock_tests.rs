//! The quiet lock: one holder, a hard deadline, and waiters that say
//! who holds it and when it expires.

use std::time::{Duration, Instant};

use crate::scratch::lock_dir;

use super::*;

const BRIEF: Duration = Duration::from_millis(400);

fn other(worktree: &str, held_for: Duration) -> Holder {
    Holder {
        pid: GONE,
        ..Holder::me(worktree, held_for, "a timed run")
    }
}

/// A pid above Linux's largest `pid_max` (2^22), so never this process.
const GONE: u32 = 4_194_305;

#[test]
fn a_free_quiet_lock_is_taken_and_names_its_holder_until_its_deadline() {
    // Given
    let dir = lock_dir("quiet-free");

    // When
    let held = take(&dir, Holder::me("/w/perf", BRIEF, "perf"), BRIEF).expect("taken");

    // Then
    let holder = read(&dir).expect("readable").expect("held");
    assert_eq!(
        (holder.worktree.as_str(), holder.what.as_str()),
        ("/w/perf", "perf")
    );
    drop(held);
    assert_eq!(read(&dir), Ok(None), "dropping it releases it");
}

#[test]
fn a_waiter_says_who_holds_it_and_when_it_expires_then_goes_on_at_the_deadline() {
    // Given
    let dir = lock_dir("quiet-expire");
    write(&dir, &other("/w/holder", BRIEF)).expect("held");

    // When
    let started = Instant::now();
    let mut said = Vec::new();
    let waited = wait_clear(&dir, ("ci", None), 10 * BRIEF, &mut |line| {
        said.push(line.to_owned());
    });

    // Then
    let elapsed = started.elapsed();
    assert_eq!(waited, Ok(()));
    assert!(
        elapsed >= Duration::from_millis(350) && elapsed < 10 * BRIEF,
        "{elapsed:?}"
    );
    assert!(
        said[0].starts_with("ci: waiting for the quiet lock, held by pid "),
        "{said:?}"
    );
    assert!(
        said[0].contains("in /w/holder running `a timed run`, expires in "),
        "{said:?}"
    );
}

#[test]
fn a_waiter_goes_on_as_soon_as_the_holder_releases_it() {
    // Given
    let dir = lock_dir("quiet-release");
    let held = take(
        &dir,
        Holder::me("/w/holder", Duration::from_secs(60), "perf"),
        BRIEF,
    )
    .expect("taken");
    let waiting_dir = dir.to_path_buf();
    let waiter = std::thread::spawn(move || {
        let started = Instant::now();
        wait_clear(
            &waiting_dir,
            ("ci", None),
            Duration::from_secs(5),
            &mut |_| {},
        )
        .map(|()| started.elapsed())
    });

    // When
    std::thread::sleep(Duration::from_millis(200));
    drop(held);

    // Then
    let took = waiter.join().expect("joined").expect("clear");
    assert!(took < Duration::from_secs(5), "{took:?}");
}

#[test]
fn the_holders_own_commands_do_not_wait_on_it() {
    // Given
    let dir = lock_dir("quiet-own");
    let holder = other("/w/holder", Duration::from_secs(60));
    write(&dir, &holder).expect("held");

    // When
    let started = Instant::now();
    let waited = wait_clear(&dir, ("ci", Some(holder.pid)), BRIEF, &mut |_| {});

    // Then
    assert_eq!(waited, Ok(()));
    assert!(started.elapsed() < BRIEF);
}

#[test]
fn a_second_holder_waits_for_the_first_to_expire_then_takes_it() {
    // Given
    let dir = lock_dir("quiet-second");
    write(&dir, &other("/w/first", BRIEF)).expect("held");

    // When
    let held = take(&dir, Holder::me("/w/second", BRIEF, "perf"), 10 * BRIEF);

    // Then
    assert!(held.is_ok(), "{held:?}");
    let holder = read(&dir).expect("readable").expect("held");
    assert_eq!(holder.worktree, "/w/second");
}

#[test]
fn a_holder_file_of_another_shape_is_refused_naming_it_and_the_fix() {
    // Given
    let dir = lock_dir("quiet-shape");
    std::fs::create_dir_all(&*dir).expect("dir");
    std::fs::write(dir.join(FILE), "somebody\n").expect("written");

    // When
    let refusal = wait_clear(&dir, ("ci", None), 10 * BRIEF, &mut |_| {}).expect_err("refused");

    // Then
    assert!(
        refusal.contains(FILE) && refusal.contains("`somebody`"),
        "{refusal}"
    );
    assert!(refusal.contains("fix: rm "), "{refusal}");
}

#[test]
fn a_deadline_past_the_longest_hold_is_refused_not_waited_on() {
    // Given
    let dir = lock_dir("quiet-far");
    write(&dir, &other("/w/far", LONGEST + Duration::from_secs(3_600))).expect("held");

    // When
    let refusal = wait_clear(&dir, ("ci", None), 10 * BRIEF, &mut |_| {}).expect_err("refused");

    // Then
    assert!(refusal.contains("past the longest hold"), "{refusal}");
}

#[test]
fn a_hold_longer_than_the_longest_is_refused() {
    let dir = lock_dir("quiet-long");
    let refusal = take(
        &dir,
        Holder::me("/w/x", LONGEST + Duration::from_secs(60), "x"),
        BRIEF,
    )
    .expect_err("too long");
    assert!(refusal.contains("longer than"), "{refusal}");
}

#[test]
fn a_waiter_fails_at_its_own_deadline_naming_the_holder() {
    // Given
    let dir = lock_dir("quiet-own-deadline");
    write(&dir, &other("/w/holder", Duration::from_secs(60))).expect("held");

    // When
    let refusal = wait_clear(&dir, ("ci", None), BRIEF, &mut |_| {}).expect_err("still held");

    // Then
    assert!(
        refusal.contains("still held by pid ") && refusal.contains("/w/holder"),
        "{refusal}"
    );
    assert!(refusal.contains("fix: rm "), "{refusal}");
}
