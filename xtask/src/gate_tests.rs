//! The gate's list, its steps, and the one answer it shares with CI.

use super::*;

fn config(text: &str) -> Config {
    Config::parse(text).expect("valid settings")
}

const SETTINGS: &str = r#"
[gate]
02 = "cargo xtask test --tier unit"
01 = "cargo xtask lint"
10 = "VAPORS_GPU_ADAPTER=llvmpipe cargo xtask test --tier gpu"

[gate.setup]
"sudo " = "apt installs and freeing disk"
"rustup " = "the toolchain"
"#;

#[test]
fn the_gate_runs_its_commands_in_key_order() {
    // When
    let commands = gate_commands(&config(SETTINGS));

    // Then
    assert_eq!(
        commands,
        [
            "cargo xtask lint",
            "cargo xtask test --tier unit",
            "VAPORS_GPU_ADAPTER=llvmpipe cargo xtask test --tier gpu"
        ]
    );
}

#[test]
fn the_setup_prefixes_are_the_gate_setup_keys() {
    // When / Then
    assert_eq!(setup_prefixes(&config(SETTINGS)), ["rustup ", "sudo "]);
}

#[test]
fn a_gate_step_runs_its_command_in_sh_and_reproduces_as_a_pasteable_line() {
    // When
    let step = step_for("scripts/check.sh target/web \"$(mktemp -d)\"");

    // Then
    assert_eq!(step.program, "sh");
    assert_eq!(step.name, "scripts/check.sh target/web \"$(mktemp -d)\"");
    assert_eq!(
        step.reproduce(),
        "sh -c 'scripts/check.sh target/web \"$(mktemp -d)\"'"
    );
}

#[test]
fn a_gate_step_is_named_by_its_command_without_the_environment() {
    // When
    let step =
        step_for("NEXTEST_PROFILE=ci VAPORS_GPU_ADAPTER=llvmpipe cargo xtask test --tier gpu");

    // Then
    assert_eq!(step.name, "cargo xtask test --tier gpu");
}

#[test]
fn this_repositorys_gate_runs_exactly_what_ci_yml_runs() {
    // Given
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let settings =
        std::fs::read_to_string(root.join(".config/factory.toml")).expect("factory.toml");
    let workflow = std::fs::read_to_string(root.join(WORKFLOW)).expect("ci.yml");
    let settings = config(&settings);

    // When
    let agreed = agree(
        &gate_commands(&settings),
        &workflow,
        &setup_prefixes(&settings),
    );

    // Then
    assert_eq!(
        agreed,
        Ok(()),
        "fix: make [gate] in .config/factory.toml and {WORKFLOW} run the same commands"
    );
}
