//! Every step the gate runs is shared or exclusive, by name, with a reason.

use std::time::Duration;

use super::*;

fn step(name: &str) -> PlannedStep {
    PlannedStep {
        name: name.to_owned(),
        run: "true".to_owned(),
        env: BTreeMap::new(),
        dir: ".".to_owned(),
        job: name.split(':').next().unwrap_or_default().to_owned(),
        job_timeout: Duration::from_mins(9),
        step_timeout: None,
    }
}

fn config(text: &str) -> Config {
    Config::parse(text).expect("settings")
}

const BOTH: &str = r#"
[gate.shared]
"ci: lint" = "fmt and clippy"
[gate.exclusive]
"web: page contract" = "frame deadlines"
"#;

#[test]
fn each_step_takes_the_hold_its_table_names() {
    // When
    let holds = holds(
        &config(BOTH),
        &[step("ci: lint"), step("web: page contract")],
    );

    // Then
    let expected = BTreeMap::from([
        ("ci: lint".to_owned(), Hold::Shared),
        ("web: page contract".to_owned(), Hold::Exclusive),
    ]);
    assert_eq!(holds, Ok(expected));
}

#[test]
fn a_step_in_neither_table_is_refused_naming_it_and_both_tables() {
    // When
    let planned = [
        step("ci: lint"),
        step("web: page contract"),
        step("ci: unit"),
    ];
    let refusal = holds(&config(BOTH), &planned).expect_err("unlisted");

    // Then
    assert!(
        refusal.contains("`ci: unit` is in neither [gate.shared] nor [gate.exclusive]")
            && refusal.contains(".config/factory.toml")
            && refusal.contains("cargo xtask gate"),
        "{refusal}"
    );
}

#[test]
fn a_step_in_both_tables_is_refused_naming_it() {
    // Given
    let text = format!("{BOTH}\"ci: lint\" = \"also alone\"\n");

    // When
    let planned = [step("ci: lint"), step("web: page contract")];
    let refusal = holds(&config(&text), &planned).expect_err("in both");

    // Then
    assert!(refusal.contains("`ci: lint` is in both"), "{refusal}");
}

#[test]
fn an_entry_that_names_no_step_is_refused_naming_its_table() {
    // When
    let refusal = holds(&config(BOTH), &[step("ci: lint")]).expect_err("stale");

    // Then
    assert!(
        refusal.contains("[gate.exclusive] `web: page contract` names no step"),
        "{refusal}"
    );
}

#[test]
fn every_step_of_the_workflow_is_classified_in_the_repository_s_settings() {
    // Given
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let settings = Config::load_from(&root).expect("settings");
    let read = |path: &str| std::fs::read_to_string(root.join(path)).map_err(|e| e.to_string());
    let workflow = read(crate::gate::WORKFLOW).expect("workflow");
    let (setup, actions) = (
        crate::gate::setup_lines(&settings),
        crate::gate::actions(&settings),
    );
    let sources = crate::gate_plan::Sources {
        setup: &setup,
        actions: &actions.expect("actions"),
        read: &read,
    };
    let planned = crate::gate_plan::plan(&workflow, &sources).expect("plan");

    // When
    let holds = holds(&settings, &planned).expect("classified");

    // Then
    assert_eq!(holds.len(), planned.len());
}
