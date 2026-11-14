//! `cargo xtask scaling`: a clean tree only, and its two steps.

use super::*;

#[test]
fn a_clean_tree_gives_its_revision() {
    assert_eq!(clean_revision("abc123", ""), Ok("abc123".to_owned()));
}

#[test]
fn a_tree_with_changes_is_refused_naming_the_revision_and_the_fix() {
    // Given
    let changes = " M crates/perf/src/scaling_judged.rs\n";

    // When
    let error = clean_revision("abc123", changes).expect_err("dirty");

    // Then
    assert!(error.contains("the tree at abc123 has changes"), "{error}");
    assert!(error.contains("fix: commit or stash"), "{error}");
}

#[test]
fn the_judge_runs_every_change_after_the_release_build_each_with_a_deadline() {
    // When
    let steps = steps();

    // Then
    let lines: Vec<String> = steps.iter().map(Step::reproduce).collect();
    assert_eq!(
        lines,
        [
            "cargo build --release -p vapors-perf",
            "target/release/vapors-perf 60 10 --scaling all"
        ]
    );
    assert!(steps.iter().all(|step| step.deadline.is_some()));
}
