//! A run started inside this run's step makes its TMPDIR inside this
//! one's; when it was killed, this run removes it, as the start-of-run
//! sweep does at the top (issue #358). Anything else stays refused.

use super::*;
use crate::scratch;

fn who(running: &dyn Fn(u32) -> bool) -> sweep::Who<'_> {
    sweep::Who {
        pid: 1,
        uid: crate::xvfb_display::current_uid().expect("uid"),
        is_running: running,
    }
}

fn with_inner(test: &str) -> (scratch::Scratch, RunTmp) {
    let base = scratch::dir(test);
    let run = RunTmp::make(&base, 2).expect("made");
    let inner = run.path().join("xtask-run-77/xtask-run-78");
    std::fs::create_dir_all(&inner).expect("inner run dirs");
    std::fs::write(inner.join("child.pid"), "78").expect("file");
    (base, run)
}

#[test]
fn a_dead_inner_runs_directory_is_removed_and_said_and_the_run_passes() {
    // Given
    let (_base, run) = with_inner("run-tmp-inner-dead");

    // When
    let said = run.sweep_inner(&who(&|_| false)).expect("swept");
    let finished = run.finish("ci", false);

    // Then
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("xtask-run-77 (2 entries), left by run 77, which is not going"),
        "{said:?}"
    );
    assert_eq!(finished, Ok(()));
}

#[test]
fn a_live_inner_runs_directory_is_left_for_the_leftover_check_to_refuse() {
    // Given
    let (_base, run) = with_inner("run-tmp-inner-live");

    // When
    let said = run.sweep_inner(&who(&|pid| pid == 77)).expect("swept");
    let refusal = run.finish("ci", false).expect_err("a live run's directory");

    // Then
    assert_eq!(said, Vec::<String>::new());
    assert!(refusal.contains("xtask-run-77"), "{refusal}");
}

#[test]
fn a_step_s_own_leftover_beside_a_dead_inner_run_is_still_refused() {
    // Given
    let (_base, run) = with_inner("run-tmp-inner-beside");
    std::fs::write(run.path().join("leaked.txt"), "x").expect("file");

    // When
    run.sweep_inner(&who(&|_| false)).expect("swept");
    let refusal = run.finish("ci", false).expect_err("left a file");

    // Then
    assert!(refusal.contains("left 1 entry"), "{refusal}");
    assert!(refusal.contains("leaked.txt"), "{refusal}");
}
