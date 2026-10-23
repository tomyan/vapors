//! The gate's plan from a workflow: which steps run, in what order, with
//! what environment, directory and deadline.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use super::*;

pub(super) const WORKFLOW: &str = r#"
env:
  RUSTFLAGS: -D warnings
  NEXTEST_PROFILE: ci
defaults:
  run:
    working-directory: touche
    shell: bash
jobs:
  ci:
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v7
        with:
          path: touche
      - name: install libdbus
        run: sudo apt-get update -q && sudo apt-get install -y -q libdbus-1-dev
      - name: free disk
        run: |
          df -h / | tail -1
          sudo rm -rf /usr/share/dotnet
      - name: greet
        uses: ./touche/.github/actions/greet
        timeout-minutes: 2
        with:
          who: world
      - name: lint
        run: cargo xtask lint && cargo xtask wasm
      - name: report a timeout
        if: cancelled()
        env:
          TIMEOUT_MINUTES: 10
        run: .github/scripts/report-timeout.sh
  gpu:
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    env:
      VAPORS_GPU_ADAPTER: llvmpipe
      NEXTEST_PROFILE: gpu-override
    steps:
      - run: cargo xtask test --tier gpu
        env:
          EXTRA: "1"
      - uses: actions/upload-artifact@v7
        if: failure()
      - name: report a timeout
        if: cancelled()
        env:
          TIMEOUT_MINUTES: 20
        run: .github/scripts/report-timeout.sh
"#;

/// The composite action the workflow calls, at its path from the gate root.
pub(super) const GREET_PATH: &str = "../touche/.github/actions/greet/action.yml";

pub(super) const GREET: &str = r#"
name: greet
inputs:
  who:
    description: whom
  greeting:
    description: how
    default: hello
runs:
  using: composite
  steps:
    - name: say it
      shell: bash
      run: echo "${{ inputs.greeting }} ${{ inputs.who }}" >> "$GITHUB_OUTPUT"
    - name: say it again
      shell: bash
      env:
        WHO: ${{ inputs.who }}
      run: echo "again $WHO"
"#;

pub(super) fn setup() -> BTreeSet<String> {
    [
        "sudo apt-get update -q && sudo apt-get install -y -q libdbus-1-dev",
        "df -h / | tail -1",
        "sudo rm -rf /usr/share/dotnet",
    ]
    .map(String::from)
    .into()
}

pub(super) fn actions() -> BTreeMap<String, String> {
    [
        ("actions/checkout", "the gate runs in a checkout"),
        ("actions/upload-artifact", "nothing to upload"),
    ]
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .into()
}

/// The plan of `workflow` with `greet` as the action's text.
pub(super) fn plan_with(
    workflow: &str,
    greet: &str,
    setup: &BTreeSet<String>,
    actions: &BTreeMap<String, String>,
) -> Result<Vec<PlannedStep>, String> {
    let read = |path: &str| {
        if path == GREET_PATH {
            Ok(greet.to_owned())
        } else {
            Err(format!("{path}: no such file"))
        }
    };
    plan(
        workflow,
        &Sources {
            setup,
            actions,
            read: &read,
        },
    )
}

pub(super) fn plan_of(workflow: &str) -> Result<Vec<PlannedStep>, String> {
    plan_with(workflow, GREET, &setup(), &actions())
}

#[test]
fn the_plan_is_every_run_step_less_setup_in_file_order_actions_expanded() {
    // When
    let planned = plan_of(WORKFLOW).expect("a plan");

    // Then
    let names: Vec<&str> = planned.iter().map(|step| step.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "ci: greet: say it",
            "ci: greet: say it again",
            "ci: lint",
            "gpu: cargo xtask test --tier gpu"
        ],
        "a `- run:` step is named by its first line"
    );
    assert_eq!(planned[2].run, "cargo xtask lint && cargo xtask wasm");
}

#[test]
fn each_step_gets_workflow_then_job_then_step_env_and_ci_true() {
    // When
    let planned = plan_of(WORKFLOW).expect("a plan");

    // Then
    let gpu = &planned[3].env;
    assert_eq!(gpu["RUSTFLAGS"], "-D warnings");
    assert_eq!(
        gpu["NEXTEST_PROFILE"], "gpu-override",
        "the job's value wins"
    );
    assert_eq!(gpu["VAPORS_GPU_ADAPTER"], "llvmpipe");
    assert_eq!(gpu["EXTRA"], "1");
    assert_eq!(gpu["CI"], "true");
    assert_eq!(planned[2].env["NEXTEST_PROFILE"], "ci");
}

#[test]
fn a_composite_actions_step_runs_from_the_workspace_with_its_inputs() {
    // When
    let planned = plan_of(WORKFLOW).expect("a plan");

    // Then: `with:` over the default, and the workspace root is the checkout's parent
    assert_eq!(planned[0].run, r#"echo "hello world" >> "$GITHUB_OUTPUT""#);
    assert_eq!(planned[0].dir, "..");
    assert_eq!(planned[2].dir, ".");
}

#[test]
fn each_step_carries_its_jobs_timeout_and_its_own() {
    // When
    let planned = plan_of(WORKFLOW).expect("a plan");

    // Then
    assert_eq!(planned[0].job_timeout, Duration::from_mins(10));
    assert_eq!(
        planned[0].step_timeout,
        Some(Duration::from_mins(2)),
        "the calling step's, for the action's steps"
    );
    assert_eq!(planned[2].step_timeout, None);
    assert_eq!(planned[3].job, "gpu");
    assert_eq!(planned[3].job_timeout, Duration::from_mins(20));
}

#[test]
fn a_step_with_a_line_not_listed_exactly_is_run() {
    // Given: free disk gains a setup-like line; a second step keeps the exact lines in use
    let workflow = WORKFLOW.replace(
        "sudo rm -rf /usr/share/dotnet\n",
        "sudo rm -rf /usr/share/dotnet\n          sudo rm -rf /usr/share/dotnet /opt/ghc\n      - name: free disk again\n        run: |\n          df -h / | tail -1\n          sudo rm -rf /usr/share/dotnet\n",
    );

    // When
    let planned = plan_of(&workflow).expect("a plan");

    // Then
    let names: Vec<&str> = planned.iter().map(|step| step.name.as_str()).collect();
    assert!(names.contains(&"ci: free disk"), "{names:?}");
    assert!(!names.contains(&"ci: free disk again"), "{names:?}");
}
