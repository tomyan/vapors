//! The factory's steps: their order and their command lines.

use super::*;

#[test]
fn ci_runs_fastest_checks_first() {
    // Given
    let steps = ci("vapors-web-demo");

    // When
    let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();

    // Then
    assert_eq!(
        names,
        [
            "tools",
            "fmt",
            "files",
            "layers",
            "builds",
            "imports",
            "nightly",
            "nightly issues",
            "deny",
            "clippy",
            "wasm",
            "wasm release",
            "unit",
            "gpu"
        ]
    );
}

#[test]
fn reproduce_line_is_the_pasteable_command() {
    // Given
    let step = fmt();

    // When
    let line = step.reproduce();

    // Then
    assert_eq!(line, "cargo fmt --all --check");
}

#[test]
fn wasm_step_lints_the_configured_package_for_the_browser_target() {
    // Given
    let step = wasm("vapors-web-demo");

    // When
    let line = step.reproduce();

    // Then
    assert_eq!(
        line,
        "RUSTFLAGS='-D warnings' cargo clippy --target wasm32-unknown-unknown -p vapors-web-demo -- -D warnings"
    );
}

#[test]
fn the_wasm_release_step_lints_the_package_as_trunk_builds_it() {
    // Given: trunk builds the page in release, where debug-only code is gone
    let step = wasm_release("touche-web");

    // When
    let line = step.reproduce();

    // Then
    assert_eq!(
        line,
        "RUSTFLAGS='-D warnings' cargo clippy --target wasm32-unknown-unknown -p touche-web --release -- -D warnings"
    );
    assert_eq!(
        wasm_lint("touche-web").len(),
        2,
        "`cargo xtask wasm` runs both"
    );
}

#[test]
fn the_deny_step_checks_every_section_within_its_deadline() {
    // When
    let step = deny();

    // Then
    assert_eq!(step.reproduce(), "cargo deny check");
    assert!(
        step.deadline.is_some(),
        "the advisory fetch needs a deadline"
    );
    assert!(step.fix.contains("offline"), "{}", step.fix);
}

#[test]
fn lint_checks_layers_builds_and_imports_after_files_and_before_clippy() {
    // When
    let names: Vec<String> = lint().into_iter().map(|s| s.name).collect();

    // Then
    assert_eq!(
        names,
        [
            "fmt",
            "files",
            "layers",
            "builds",
            "imports",
            "nightly",
            "nightly issues",
            "clippy"
        ]
    );
}

#[test]
fn ci_without_a_named_step_runs_every_other() {
    // Given
    let all = super::ci("web");

    // When
    let names: Vec<String> = super::without(all, &["gpu".to_owned()])
        .expect("known")
        .into_iter()
        .map(|step| step.name)
        .collect();

    // Then
    assert!(!names.contains(&"gpu".to_owned()), "{names:?}");
    assert_eq!(names.len(), super::ci("web").len() - 1);
}

#[test]
fn ci_without_a_step_it_does_not_have_is_refused_naming_its_steps() {
    let refused = super::without(super::ci("web"), &["gpus".to_owned()])
        .err()
        .expect("refused");
    assert!(
        refused.starts_with("--except gpus: ci has no such step (tools, fmt"),
        "{refused}"
    );
}
