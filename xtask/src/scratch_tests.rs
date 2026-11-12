//! Scratch directories go when dropped, after a pass or a panic, and are
//! kept only for a failing test with the switch set.

use super::*;

#[test]
fn a_scratch_directory_is_removed_when_dropped() {
    // Given
    let scratch = dir("scratch-dropped");
    std::fs::write(scratch.join("file"), "x").expect("file");
    let path = scratch.to_path_buf();

    // When
    drop(scratch);

    // Then
    assert!(!path.exists(), "{}", path.display());
}

#[test]
fn a_scratch_directory_is_removed_when_its_test_panics() {
    // Given: a test body that makes a directory, then fails
    let made = std::sync::Mutex::new(None);
    let failed = std::panic::catch_unwind(|| {
        let scratch = dir("scratch-panicked");
        std::fs::write(scratch.join("file"), "x").expect("file");
        *made.lock().expect("lock") = Some(scratch.to_path_buf());
        panic!("the test fails");
    });

    // Then
    assert!(failed.is_err());
    let path = made.lock().expect("lock").clone().expect("made");
    assert!(!path.exists(), "{}", path.display());
}

#[test]
fn only_a_failing_test_with_the_switch_set_keeps_its_directory() {
    assert!(keeps(true, true));
    assert!(!keeps(true, false));
    assert!(!keeps(false, true));
    assert!(!keeps(false, false));
}
