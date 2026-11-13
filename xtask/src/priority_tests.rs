//! Builds and checks outside the gate yield to the gate.

use super::*;

#[test]
fn outside_the_gate_a_step_runs_under_nice_10() {
    // When
    let command = command_for("cargo", false, false);

    // Then
    assert_eq!(command.get_program(), "nice");
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args, ["-n", "10", "cargo"]);
}

#[test]
fn in_the_gate_a_step_runs_at_normal_priority() {
    assert_eq!(command_for("cargo", true, false).get_program(), "cargo");
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
    let command = command_for("cargo", true, false);

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

#[test]
fn an_xtask_already_niced_does_not_nice_its_steps_again() {
    // Given: `cargo xtask check-files`, run niced by `cargo xtask ci`

    // When
    let command = command_for("cargo", false, true);

    // Then: it stays at nice 10, not 20
    assert_eq!(command.get_program(), "cargo");
}

#[test]
fn a_niced_step_says_so_to_the_xtask_it_runs() {
    let command = command_for("cargo", false, false);
    let marked = command
        .get_envs()
        .any(|(name, value)| name == NICED_ENV && value == Some("1".as_ref()));
    assert!(marked);
}

#[test]
fn the_gate_builds_on_every_thread_over_any_inherited_or_configured_cap() {
    // Given
    let threads = std::thread::available_parallelism()
        .expect("threads")
        .to_string();

    // When
    let command = command_for("cargo", true, false);

    // Then
    let jobs = command
        .get_envs()
        .find(|(name, _)| *name == "CARGO_BUILD_JOBS")
        .and_then(|(_, value)| value);
    assert_eq!(jobs, Some(threads.as_ref()));
}

#[test]
fn plain_cargo_is_capped_at_the_same_jobs_as_xtask_s_steps() {
    // Given
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let text = std::fs::read_to_string(root.join(".cargo/config.toml")).expect("config.toml");
    let config = Config::load_from(&root).expect(".config/factory.toml");

    // When
    let configured = jobs(&config).expect("[build] jobs");

    // Then
    let line = format!("jobs = {configured}");
    assert!(
        text.lines().any(|l| l.trim() == line),
        ".cargo/config.toml has no `{line}` under [build], the [build] jobs xtask's steps use; \
         fix: make the two the same"
    );
}

#[test]
fn sccache_is_never_niced_since_its_server_keeps_the_niceness_it_starts_with() {
    assert_eq!(
        command_for("sccache", false, false).get_program(),
        "sccache"
    );
}
