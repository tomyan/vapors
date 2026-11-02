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
    assert!(refusal.contains("hide_gpu_device = \"false\""), "{refusal}");
}

#[test]
fn this_machine_hides_dev_dri_inside_the_sandbox() {
    // When
    let checked = check_with("bwrap");

    // Then
    assert_eq!(checked, Ok(()));
}

#[test]
fn the_off_switch_runs_steps_as_they_are_and_another_value_is_refused() {
    // Given
    let off = Config::parse("[gate]\nhide_gpu_device = \"false\"\n").expect("off");
    let odd = Config::parse("[gate]\nhide_gpu_device = \"no\"\n").expect("odd");

    // Then
    assert_eq!(wrap(&off), Ok(Wrap::AsIs));
    let refusal = wrap(&odd).expect_err("refused");
    assert!(refusal.contains("fix: .config/factory.toml"), "{refusal}");
}

#[test]
fn unset_the_steps_hide_the_device() {
    assert_eq!(
        wrap(&Config::parse("").expect("empty")),
        Ok(Wrap::HideGpuDevice)
    );
}
