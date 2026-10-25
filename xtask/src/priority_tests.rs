//! Builds and checks outside the gate yield to the gate.

use super::*;

#[test]
fn outside_the_gate_a_step_runs_under_nice_10() {
    // When
    let command = command_for("cargo", false);

    // Then
    assert_eq!(command.get_program(), "nice");
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args, ["-n", "10", "cargo"]);
}

#[test]
fn in_the_gate_a_step_runs_at_normal_priority() {
    assert_eq!(command_for("cargo", true).get_program(), "cargo");
}

#[test]
fn the_job_cap_is_the_configured_number_or_four() {
    let config = |text: &str| Config::parse(text).expect("config");
    assert_eq!(jobs(&config("[web]\npackage = \"p\"\n")), Ok(4));
    assert_eq!(jobs(&config("[build]\njobs = \"6\"\n")), Ok(6));
}

#[test]
fn a_job_cap_that_is_not_a_positive_number_is_refused() {
    for text in ["[build]\njobs = \"0\"\n", "[build]\njobs = \"many\"\n"] {
        let refusal = jobs(&Config::parse(text).expect("config")).expect_err(text);
        assert!(refusal.contains("[build] jobs"), "{refusal}");
    }
}

#[test]
fn a_gate_step_tells_the_xtask_it_runs_that_it_is_in_the_gate() {
    // When
    let command = command_for("cargo", true);

    // Then: `cargo xtask lint` run by the gate is not niced either
    let marked = command
        .get_envs()
        .any(|(name, value)| name == GATE_ENV && value == Some("1".as_ref()));
    assert!(marked);
}

#[test]
fn an_xtask_a_gate_step_started_counts_as_the_gate() {
    assert!(in_gate(true, None));
    assert!(in_gate(false, Some("1")));
    assert!(!in_gate(false, None));
    assert!(!in_gate(false, Some("0")));
}
