//! The gate's plan from a workflow: which steps run, with what
//! environment, and what is refused by name.

use std::collections::BTreeSet;

use super::*;

pub(super) const WORKFLOW: &str = r#"
env:
  RUSTFLAGS: -D warnings
  NEXTEST_PROFILE: ci
defaults:
  run:
    working-directory: touche
jobs:
  ci:
    runs-on: ubuntu-24.04
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
      - name: lint
        run: cargo xtask lint && cargo xtask wasm
  gpu:
    runs-on: ubuntu-24.04
    env:
      VAPORS_GPU_ADAPTER: llvmpipe
      NEXTEST_PROFILE: gpu-override
    steps:
      - run: cargo xtask test --tier gpu
        env:
          EXTRA: "1"
      - uses: actions/upload-artifact@v7
        if: failure()
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

pub(super) fn plan_of(workflow: &str) -> Result<Vec<PlannedStep>, String> {
    plan(workflow, &setup())
}

#[test]
fn the_plan_is_every_run_step_less_setup_in_file_order() {
    // When
    let planned = plan_of(WORKFLOW).expect("a plan");

    // Then
    let runs: Vec<&str> = planned.iter().map(|step| step.run.as_str()).collect();
    assert_eq!(
        runs,
        [
            "cargo xtask lint && cargo xtask wasm",
            "cargo xtask test --tier gpu"
        ]
    );
    assert_eq!(planned[0].name, "ci: lint");
    assert_eq!(
        planned[1].name, "gpu: cargo xtask test --tier gpu",
        "a `- run:` step is named by its first line"
    );
}

#[test]
fn each_step_gets_workflow_then_job_then_step_env_and_ci_true() {
    // When
    let planned = plan_of(WORKFLOW).expect("a plan");

    // Then
    let gpu = &planned[1].env;
    assert_eq!(gpu["RUSTFLAGS"], "-D warnings");
    assert_eq!(
        gpu["NEXTEST_PROFILE"], "gpu-override",
        "the job's value wins"
    );
    assert_eq!(gpu["VAPORS_GPU_ADAPTER"], "llvmpipe");
    assert_eq!(gpu["EXTRA"], "1");
    assert_eq!(gpu["CI"], "true");
    assert_eq!(planned[0].env["NEXTEST_PROFILE"], "ci");
}

#[test]
fn a_setup_like_line_that_is_not_listed_exactly_is_run() {
    // Given
    let workflow = WORKFLOW.replace(
        "sudo rm -rf /usr/share/dotnet\n",
        "sudo rm -rf /usr/share/dotnet\n          sudo rm -rf /usr/share/dotnet /opt/ghc\n",
    );

    // When
    let planned = plan_of(&workflow).expect("a plan");

    // Then: the free-disk step is no longer all setup, so the gate runs it
    assert!(
        planned.iter().any(|step| step.name == "ci: free disk"),
        "{planned:?}"
    );
}

#[test]
fn a_run_step_ci_runs_on_success_is_planned_and_one_only_on_failure_is_not() {
    // Given
    let with = |condition: &str| {
        WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            &format!("        if: {condition}\n        run: cargo xtask lint && cargo xtask wasm"),
        )
    };
    let lint = |workflow: &str| {
        plan_of(workflow)
            .expect("a plan")
            .iter()
            .any(|step| step.name == "ci: lint")
    };

    // Then
    assert!(lint(&with("always()")), "always() runs on success");
    assert!(lint(&with("success()")), "success() runs on success");
    assert!(
        !lint(&with("failure()")),
        "failure() never runs when the gate passes"
    );
}
