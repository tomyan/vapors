//! The gate's setup lines, its steps, and this repository's workflow.

use std::path::Path;

use super::*;

const SETTINGS: &str = r#"
[gate.setup]
"df -h / | tail -1" = "logs the runner's free disk"
"rustup toolchain install && rustup show active-toolchain" = "the toolchain"
"#;

#[test]
fn the_setup_lines_are_the_gate_setup_keys_exactly() {
    // When
    let lines = setup_lines(&Config::parse(SETTINGS).expect("valid"));

    // Then
    assert_eq!(
        lines.into_iter().collect::<Vec<_>>(),
        [
            "df -h / | tail -1",
            "rustup toolchain install && rustup show active-toolchain"
        ]
    );
}

#[test]
fn an_action_listed_without_a_reason_is_refused() {
    // Given
    let settings = "[gate.actions]\n\"actions/checkout\" = \" \"\n";

    // When
    let refusal = actions(&Config::parse(settings).expect("valid")).expect_err("no reason");

    // Then
    assert!(
        refusal.contains("`actions/checkout` has no reason"),
        "{refusal}"
    );
}

#[test]
fn a_tree_with_changes_is_refused_before_anything_runs() {
    // Given
    let dirty = State {
        head: format!("abc{DIRTY}"),
        siblings: Vec::new(),
    };
    let clean = State {
        head: "abc".to_owned(),
        siblings: Vec::new(),
    };

    // When / Then
    assert!(
        refuse_dirty(&dirty)
            .expect_err("dirty")
            .contains("the tree has changes")
    );
    assert_eq!(refuse_dirty(&clean), Ok(()));
}

#[test]
fn this_repositorys_workflow_is_one_the_gate_can_run() {
    // Given
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let settings =
        std::fs::read_to_string(root.join(".config/factory.toml")).expect("factory.toml");
    let workflow = std::fs::read_to_string(root.join(WORKFLOW)).expect("ci.yml");

    let config = Config::parse(&settings).expect("valid");
    let (setup, actions) = (setup_lines(&config), actions(&config).expect("reasons"));
    let read =
        |path: &str| std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"));

    // When
    let planned = plan(
        &workflow,
        &Sources {
            setup: &setup,
            actions: &actions,
            read: &read,
        },
    );

    // Then
    // A refusal names its own fix when it has one; the generic one otherwise.
    let planned = planned.unwrap_or_else(|why| {
        let fix = if why.contains("fix:") {
            ""
        } else {
            "; fix: make the step one the gate models"
        };
        panic!("{WORKFLOW}: {why}{fix}")
    });
    assert!(
        planned.iter().all(|step| !step.run.contains("sudo ")),
        "a setup step is planned: {planned:?}"
    );
}
