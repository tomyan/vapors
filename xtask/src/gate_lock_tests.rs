//! One gate at a time on the machine.

use std::time::Duration;

use super::*;

/// A test's lock directory, removed when the guard drops: /tmp is a
/// RAM-backed tmpfs here, and every run left one per test behind.
struct Scratch(std::path::PathBuf);

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;
    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dir(test: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("xtask-gate-lock-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Scratch(dir)
}

#[test]
fn a_test_s_lock_directory_is_removed_when_its_guard_drops() {
    // Given
    let dir = dir("scratch");
    let held = acquire(&dir, "/w/touche", Duration::from_millis(200)).expect("taken");
    let path = dir.to_path_buf();
    assert!(path.exists(), "the lock made it");

    // When
    drop(held);
    drop(dir);

    // Then
    assert!(!path.exists(), "{} is left behind", path.display());
}

#[test]
fn a_free_lock_is_taken_and_names_its_holder() {
    // Given
    let dir = dir("free");

    // When
    let held = acquire(&dir, "/w/touche", Duration::from_millis(200)).expect("taken");

    // Then
    let holder = std::fs::read_to_string(dir.join(HOLDER)).expect("holder");
    assert!(
        holder.contains(&format!("pid {}", std::process::id())),
        "{holder}"
    );
    assert!(holder.contains("/w/touche"), "{holder}");
    drop(held);
}

#[test]
fn a_held_lock_makes_the_next_gate_wait_then_fail_naming_the_holder() {
    // Given
    let dir = dir("held");
    let _first = acquire(&dir, "/w/first", Duration::from_millis(200)).expect("first");

    // When
    let started = std::time::Instant::now();
    let refusal = acquire(&dir, "/w/second", Duration::from_millis(400)).expect_err("waits");

    // Then
    assert!(started.elapsed() >= Duration::from_millis(400), "it waited");
    assert!(refusal.contains("/w/first"), "{refusal}");
    assert!(refusal.contains("held"), "{refusal}");
}

#[test]
fn a_released_lock_is_taken_by_the_next_gate() {
    // Given
    let dir = dir("released");
    let first = acquire(&dir, "/w/first", Duration::from_millis(200)).expect("first");

    // When
    drop(first);
    let second = acquire(&dir, "/w/second", Duration::from_millis(200));

    // Then
    assert!(second.is_ok());
}

#[test]
fn a_home_that_is_empty_or_relative_is_refused() {
    for home in ["", "relative/home"] {
        let refusal = dir_in(Some(home.into())).expect_err(home);
        assert!(refusal.contains("HOME"), "{refusal}");
    }
}
