//! Conditions, composite actions, third-party actions and stale
//! entries: what the plan skips and refuses, each by name.

use super::refusal_tests::refused;
use super::tests::{GREET, WORKFLOW, actions, plan_of, plan_with, setup};

/// The refusal when the action's text is `greet`.
fn refused_action(greet: &str, needle: &str) {
    let refusal = plan_with(WORKFLOW, greet, &setup(), &actions()).expect_err("refused");
    assert!(refusal.contains(needle), "wanted `{needle}` in: {refusal}");
}

#[test]
fn a_composite_step_the_gate_does_not_model_is_refused() {
    refused_action(
        &GREET.replace("shell: bash", "shell: pwsh"),
        "ci: greet: say it: `shell: pwsh`",
    );
    refused_action(
        &GREET.replace("${{ inputs.who }}", "${{ github.actor }}"),
        "ci: greet: say it: an expression",
    );
    refused_action(
        &GREET.replace("using: composite", "using: node20"),
        "not a composite action",
    );
    refused_action(
        &GREET.replace("    default: hello\n", ""),
        "input `greeting` has no value and no default",
    );
}

#[test]
fn an_input_the_action_does_not_declare_is_refused() {
    refused(
        &WORKFLOW.replace(
            "          who: world",
            "          who: world\n          whom: you",
        ),
        "`with: whom`, which the action does not declare",
    );
}

#[test]
fn a_missing_action_file_is_refused_naming_it() {
    refused(
        &WORKFLOW.replace("actions/greet", "actions/gone"),
        "../touche/.github/actions/gone/action.yml",
    );
}

#[test]
fn a_third_party_action_not_listed_is_refused_naming_where_to_list_it() {
    refused(
        &WORKFLOW.replace("actions/upload-artifact@v7", "someone/upload@v1"),
        "`someone/upload@v1`, an action the gate does not run; fix: list \"someone/upload\" in [gate.actions]",
    );
}

#[test]
fn a_listed_action_or_setup_line_nothing_uses_is_refused_naming_it() {
    // Given
    let mut stale_actions = actions();
    stale_actions.insert("gone/action".to_owned(), "was used once".to_owned());
    let mut stale_setup = setup();
    stale_setup.insert("sudo apt-get install -y -q gone".to_owned());

    // When
    let action = plan_with(WORKFLOW, GREET, &setup(), &stale_actions).expect_err("stale action");
    let line = plan_with(WORKFLOW, GREET, &stale_setup, &actions()).expect_err("stale line");

    // Then
    assert!(
        action.contains("[gate.actions] `gone/action` is used by no step"),
        "{action}"
    );
    assert!(
        line.contains("`sudo apt-get install -y -q gone` makes no step of ci.yml all setup"),
        "{line}"
    );
}

#[test]
fn an_action_step_ci_skips_on_success_is_not_expanded() {
    // Given
    let workflow = WORKFLOW.replace(
        "        uses: ./touche/.github/actions/greet",
        "        if: failure()\n        uses: ./touche/.github/actions/greet",
    );

    // When
    let planned = plan_of(&workflow).expect("a plan");

    // Then
    assert!(
        planned.iter().all(|step| step.name != "ci: greet: say it"),
        "{planned:?}"
    );
}

#[test]
fn a_step_ci_runs_on_success_is_planned_and_one_only_on_failure_is_not() {
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
