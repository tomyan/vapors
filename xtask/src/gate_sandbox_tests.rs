//! The gate's steps see no GPU device, as GitHub's runners have none.

use super::*;

#[test]
fn every_step_runs_with_dev_dri_hidden() {
    // Then
    assert_eq!(
        HIDE_GPU_DEVICE,
        [
            "--dev-bind",
            "/",
            "/",
            "--tmpfs",
            "/dev/dri",
            "--die-with-parent"
        ]
    );
}

#[test]
fn a_machine_without_bwrap_is_refused_naming_the_fix() {
    // When
    let refusal = check_with("no-such-bwrap-for-this-test").expect_err("refused");

    // Then
    assert!(refusal.contains("no-such-bwrap-for-this-test"), "{refusal}");
    assert!(refusal.contains("fix: install bubblewrap"), "{refusal}");
}

#[test]
fn this_machine_hides_dev_dri_inside_the_sandbox() {
    // When
    let checked = check_with("bwrap");

    // Then
    assert_eq!(checked, Ok(()));
}
