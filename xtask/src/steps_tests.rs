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
            "host",
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
    let step = super::deny_with(None);

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
fn the_deny_step_runs_every_check_offline_when_the_sandbox_says_so() {
    // When
    let step = super::deny_with(Some("1"));

    // Then
    assert_eq!(step.reproduce(), "cargo deny --offline check");
}

#[test]
fn the_deny_step_refuses_a_value_of_its_variable_it_does_not_know() {
    // When
    let step = super::deny_with(Some("yes"));

    // Then: a step that fails, naming the value
    assert_eq!(step.program, "false");
    assert!(
        step.fix.contains("VAPORS_DENY_OFFLINE=\"yes\""),
        "{}",
        step.fix
    );
}
