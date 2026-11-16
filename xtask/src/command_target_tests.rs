//! A step builds into the directory the cap measured: `spawn` hands the
//! step's own `CARGO_TARGET_DIR` to `sccache::apply`, which sets it on
//! the command and measures it, so the two cannot differ.

use crate::steps::Step;

fn printing_target(dir: &std::path::Path, value: &str) -> Step {
    let mut step = Step::new(
        "target",
        "sh",
        &["-c", r#"printf %s "$CARGO_TARGET_DIR""#],
        "a test step",
    );
    step.dir = Some(dir.display().to_string());
    step.env
        .push(("CARGO_TARGET_DIR".to_owned(), value.to_owned()));
    step
}

#[test]
fn a_step_builds_into_its_own_cargo_target_dir_as_measured() {
    // Given
    let dir = crate::scratch::dir("a_step_builds_into_its_own_cargo_target_dir_as_measured");
    let target = dir.join("built");

    // When
    let outcome = super::run(&printing_target(&dir, &target.display().to_string()));

    // Then
    assert!(outcome.passed, "{}", outcome.output);
    assert_eq!(outcome.output, target.display().to_string());
}

#[test]
fn a_relative_cargo_target_dir_reaches_the_step_resolved_from_its_directory() {
    // Given
    let dir = crate::scratch::dir("a_relative_cargo_target_dir_reaches_the_step_resolved");

    // When
    let outcome = super::run(&printing_target(&dir, "target"));

    // Then
    assert!(outcome.passed, "{}", outcome.output);
    assert_eq!(outcome.output, dir.join("target").display().to_string());
}
