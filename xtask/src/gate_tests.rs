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
fn a_gate_step_is_bash_with_pipefail_and_reproduces_with_its_environment() {
    // Given
    let planned = PlannedStep {
        name: "gpu: gpu tier".to_owned(),
        run: "cargo xtask test --tier gpu".to_owned(),
        env: [("CI", "true"), ("VAPORS_GPU_ADAPTER", "llvmpipe")]
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .into(),
    };

    // When
    let step = step_for(&planned);

    // Then
    assert_eq!(step.name, "gpu: gpu tier");
    assert_eq!(
        step.reproduce(),
        "CI=true VAPORS_GPU_ADAPTER=llvmpipe bash -eo pipefail -c 'cargo xtask test --tier gpu'"
    );
}

#[test]
fn this_repositorys_workflow_is_one_the_gate_can_run() {
    // Given
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let settings =
        std::fs::read_to_string(root.join(".config/factory.toml")).expect("factory.toml");
    let workflow = std::fs::read_to_string(root.join(WORKFLOW)).expect("ci.yml");

    // When
    let planned = plan(
        &workflow,
        &setup_lines(&Config::parse(&settings).expect("valid")),
    );

    // Then
    let planned = planned
        .unwrap_or_else(|why| panic!("{WORKFLOW}: {why}; fix: make the step one the gate models"));
    assert!(
        planned.iter().all(|step| !step.run.contains("sudo ")),
        "a setup step is planned: {planned:?}"
    );
}
