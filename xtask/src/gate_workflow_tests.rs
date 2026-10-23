//! Reading the commands a workflow runs, and holding the gate's list to
//! them.

use super::*;

const WORKFLOW: &str = "\
jobs:
  ci:
    steps:
      - name: install libdbus
        run: sudo apt-get update -q && sudo apt-get install -y -q libdbus-1-dev
      - name: toolchain
        run: rustup toolchain install && rustup show active-toolchain
      - name: free disk
        run: |
          df -h / | tail -1
          sudo rm -rf /usr/share/dotnet
      - uses: actions/cache@abc
        with:
          path: x
      - name: lint
        run: cargo xtask lint && cargo xtask test --tier unit
  web:
    steps:
      - name: check
        run: scripts/check.sh target/web \"$(mktemp -d)\" # the page contract
";

fn setup() -> Vec<String> {
    ["sudo ", "rustup ", "df "].map(String::from).to_vec()
}

#[test]
fn the_runs_are_split_on_and_and_on_lines_with_comments_dropped() {
    // When
    let commands = run_commands(WORKFLOW);

    // Then
    assert_eq!(
        commands,
        [
            "sudo apt-get update -q",
            "sudo apt-get install -y -q libdbus-1-dev",
            "rustup toolchain install",
            "rustup show active-toolchain",
            "df -h / | tail -1",
            "sudo rm -rf /usr/share/dotnet",
            "cargo xtask lint",
            "cargo xtask test --tier unit",
            "scripts/check.sh target/web \"$(mktemp -d)\"",
        ]
    );
}

#[test]
fn setup_commands_are_left_out_of_what_the_gate_must_run() {
    // When
    let checked = checked_commands(WORKFLOW, &setup());

    // Then
    assert_eq!(
        checked,
        [
            "cargo xtask lint",
            "cargo xtask test --tier unit",
            "scripts/check.sh target/web \"$(mktemp -d)\""
        ]
    );
}

#[test]
fn a_gate_that_runs_what_the_workflow_runs_agrees() {
    // Given: in its own order, with an environment assignment the job sets
    let gate = [
        "scripts/check.sh target/web \"$(mktemp -d)\"",
        "cargo xtask lint",
        "VAPORS_GPU_ADAPTER=llvmpipe cargo xtask test --tier unit",
    ]
    .map(String::from);

    // When / Then
    assert_eq!(agree(&gate, WORKFLOW, &setup()), Ok(()));
}

#[test]
fn a_workflow_step_the_gate_does_not_run_is_named() {
    // Given
    let gate = ["cargo xtask lint", "cargo xtask test --tier unit"].map(String::from);

    // When
    let refusal = agree(&gate, WORKFLOW, &setup()).expect_err("check.sh missing");

    // Then
    assert!(
        refusal.contains(
            "ci.yml runs `scripts/check.sh target/web \"$(mktemp -d)\"`, the gate does not"
        ),
        "{refusal}"
    );
}

#[test]
fn a_gate_command_the_workflow_does_not_run_is_named() {
    // Given
    let gate = [
        "cargo xtask lint",
        "cargo xtask test --tier unit",
        "scripts/check.sh target/web \"$(mktemp -d)\"",
        "cargo xtask stray",
    ]
    .map(String::from);

    // When
    let refusal = agree(&gate, WORKFLOW, &setup()).expect_err("stray");

    // Then
    assert!(
        refusal.contains("the gate runs `cargo xtask stray`, ci.yml does not"),
        "{refusal}"
    );
}
