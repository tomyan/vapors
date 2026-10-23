//! What the gate's plan refuses in jobs and steps, each by name.

use super::tests::{WORKFLOW, plan_of};

/// The refusal for `workflow`, which must name `needle`.
pub(super) fn refused(workflow: &str, needle: &str) {
    let refusal = plan_of(workflow).expect_err("refused");
    assert!(refusal.contains(needle), "wanted `{needle}` in: {refusal}");
}

const LINT: &str = "        run: cargo xtask lint && cargo xtask wasm";

#[test]
fn step_keys_and_scripts_the_gate_does_not_model_are_refused_naming_job_and_step() {
    let cases = [
        (format!("{LINT}\n        shell: bash"), "ci: lint: `shell:`"),
        (
            format!("{LINT}\n        working-directory: x"),
            "ci: lint: `working-directory:`",
        ),
        (
            format!("{LINT}\n        continue-on-error: true"),
            "ci: lint: `continue-on-error:`",
        ),
        (
            format!("{LINT}\n        if: github.event_name == 'push'"),
            "ci: lint: `if: github.event_name == 'push'`",
        ),
        (
            "        run: |\n          cargo xtask lint \\\n            --verbose".to_owned(),
            "ci: lint: a `\\` continuation",
        ),
        (
            "        run: echo ${{ github.sha }}".to_owned(),
            "ci: lint: an expression",
        ),
        (
            "        run: echo X=1 >> \"$GITHUB_ENV\"".to_owned(),
            "ci: lint: writes GITHUB_ENV",
        ),
        (
            "        run: [a, b]".to_owned(),
            "ci: lint: `run` is not a string",
        ),
        (
            format!("{LINT}\n        timeout-minutes: soon"),
            "ci: lint: `timeout-minutes:",
        ),
    ];
    for (replacement, needle) in cases {
        refused(&WORKFLOW.replace(LINT, &replacement), needle);
    }
    refused(
        &WORKFLOW.replace("        if: failure()", "        if: github.ref == 'main'"),
        "gpu: actions/upload-artifact@v7: `if: github.ref == 'main'`",
    );
}

#[test]
fn job_keys_the_gate_does_not_model_are_refused_naming_the_job() {
    let gpu = "    timeout-minutes: 20\n";
    for (key, needle) in [
        ("    needs: ci\n", "gpu: `needs:`"),
        (
            "    strategy:\n      matrix:\n        os: [a]\n",
            "gpu: `strategy:`",
        ),
        ("    container: rust\n", "gpu: `container:`"),
        (
            "    services:\n      db:\n        image: x\n",
            "gpu: `services:`",
        ),
    ] {
        refused(&WORKFLOW.replace(gpu, &format!("{gpu}{key}")), needle);
    }
}

#[test]
fn a_job_without_timeout_minutes_is_refused() {
    refused(
        &WORKFLOW.replace("    timeout-minutes: 20\n", ""),
        "gpu: no `timeout-minutes:`",
    );
}

#[test]
fn an_env_that_is_not_a_mapping_is_refused() {
    refused(
        &WORKFLOW.replace(
            "      VAPORS_GPU_ADAPTER: llvmpipe\n      NEXTEST_PROFILE: gpu-override\n",
            "      - llvmpipe\n",
        ),
        "gpu: `env:` is not a mapping",
    );
}

#[test]
fn a_working_directory_default_other_than_the_checkout_path_is_refused() {
    refused(
        &WORKFLOW.replace(
            "    working-directory: touche",
            "    working-directory: elsewhere",
        ),
        "ci: run steps run in `elsewhere`",
    );
    refused(
        &WORKFLOW.replace("    timeout-minutes: 20\n", "    timeout-minutes: 20\n    defaults:\n      run:\n        working-directory: other\n"),
        "gpu: run steps run in `other`",
    );
    refused(
        &WORKFLOW.replace("    shell: bash\n", "    shell: sh\n"),
        "ci: run steps run under `sh`",
    );
}

#[test]
fn a_job_whose_run_steps_are_not_bash_is_refused() {
    // Given: no shell, so CI runs `bash -e` without pipefail
    let workflow = WORKFLOW.replace("    shell: bash\n", "");

    // Then
    refused(
        &workflow,
        "ci: run steps run under CI's default `bash -e`, without pipefail",
    );
}

#[test]
fn a_jobs_own_bash_default_is_enough() {
    // Given: the workflow names no shell, each job names bash
    let workflow = WORKFLOW
        .replace("    shell: bash\n", "")
        .replace(
            "    timeout-minutes: 10\n",
            "    timeout-minutes: 10\n    defaults:\n      run:\n        shell: bash\n",
        )
        .replace(
            "    timeout-minutes: 20\n",
            "    timeout-minutes: 20\n    defaults:\n      run:\n        shell: bash\n",
        );

    // When / Then
    assert!(plan_of(&workflow).is_ok(), "{:?}", plan_of(&workflow));
}
