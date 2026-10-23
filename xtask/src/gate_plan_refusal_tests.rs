//! What the gate's plan refuses, each by name.

use super::tests::{WORKFLOW, plan_of, setup};
use super::*;

/// The refusal for `workflow`, which must name `needle`.
fn refused(workflow: &str, needle: &str) {
    let refusal = plan_of(workflow).expect_err("refused");
    assert!(refusal.contains(needle), "wanted `{needle}` in: {refusal}");
}

#[test]
fn steps_the_gate_does_not_model_are_refused_naming_job_and_step() {
    refused(
        &WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            "        run: cargo xtask lint\n        shell: bash",
        ),
        "ci: lint: `shell:`",
    );
    refused(
        &WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            "        run: cargo xtask lint\n        working-directory: x",
        ),
        "ci: lint: `working-directory:`",
    );
    refused(
        &WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            "        run: cargo xtask lint\n        if: github.event_name == 'push'",
        ),
        "ci: lint: `if: github.event_name == 'push'` on a run step",
    );
    refused(
        &WORKFLOW.replace("        if: failure()", "        if: github.ref == 'main'"),
        "gpu: actions/upload-artifact@v7: `if: github.ref == 'main'`",
    );
    refused(
        &WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            "        run: |\n          cargo xtask lint \\\n            --verbose",
        ),
        "ci: lint: a `\\` continuation",
    );
    refused(
        &WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            "        run: echo ${{ github.sha }}",
        ),
        "ci: lint: an expression",
    );
    refused(
        &WORKFLOW.replace(
            "        run: cargo xtask lint && cargo xtask wasm",
            "        run: [a, b]",
        ),
        "ci: lint: `run` is not a string",
    );
    refused(
        &WORKFLOW.replace(
            "    runs-on: ubuntu-24.04\n    env:",
            "    runs-on: ubuntu-24.04\n    strategy:\n      matrix:\n        os: [a]\n    env:",
        ),
        "gpu: a matrix",
    );
}

#[test]
fn a_working_directory_default_other_than_the_checkout_path_is_refused() {
    refused(
        &WORKFLOW.replace(
            "    working-directory: touche",
            "    working-directory: elsewhere",
        ),
        "defaults.run.working-directory `elsewhere`",
    );
}

#[test]
fn a_setup_line_that_matches_no_run_step_is_refused_naming_it() {
    // Given: a step that was removed from the workflow, still listed as setup
    let mut setup = setup();
    setup.insert("sudo apt-get install -y -q gone".to_owned());

    // When
    let error = plan(WORKFLOW, &setup).expect_err("a stale setup line");

    // Then
    assert!(
        error.contains("`sudo apt-get install -y -q gone` matches no line of a run step"),
        "{error}"
    );
}
