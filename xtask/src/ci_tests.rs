//! A run gives its steps a TMPDIR of their own and fails when a step
//! leaves anything there.

use super::*;
use crate::scratch;

fn shell(name: &str, script: &str) -> Step {
    Step::new(name, "sh", &["-c", script], "a test step")
}

#[test]
fn a_passing_step_that_leaves_a_file_in_its_tmpdir_fails_the_run_naming_it() {
    // Given: a step that passes but leaks, as a test that forgets its file
    let base = scratch::dir("ci-leak");
    let leaking = shell("leak", r#"touch "$TMPDIR/leaked-by-a-test""#);

    // When
    let refusal = crate::run_tmp::around("ci", &base, |tmp| {
        run_in(&[leaking], &Reporter::new(false), tmp)
    })
    .expect_err("leaked");

    // Then
    assert!(refusal.contains("leaked-by-a-test"), "{refusal}");
    assert_eq!(
        std::fs::read_dir(&*base).expect("base").count(),
        0,
        "removed"
    );
}

#[test]
fn steps_that_leave_nothing_pass_and_each_had_the_run_directory_as_tmpdir() {
    // Given: a step that fails unless TMPDIR is this run's directory
    let base = scratch::dir("ci-clean");
    let expected = base.join(format!("xtask-run-{}", std::process::id()));
    let script = format!(
        r#"[ "$TMPDIR" = "{}" ] && touch "$TMPDIR/x" && rm "$TMPDIR/x""#,
        expected.display()
    );

    // When
    let code = crate::run_tmp::around("ci", &base, |tmp| {
        run_in(&[shell("tmpdir", &script)], &Reporter::new(false), tmp)
    });

    // Then
    assert_eq!(code, Ok(ExitCode::SUCCESS));
    assert!(!expected.exists(), "removed");
}
