//! `cargo xtask ci` and `cargo xtask gate` call the load wait, and a ci
//! run inside a gate does not: each runs as a process in a scratch
//! workspace whose `[load] wait = "false"` makes the wait say so without
//! reading the load. HOME is the scratch directory, so the gate lock is
//! its own and no real gate is waited on.

use std::path::{Path, PathBuf};
use std::process::Command;

const WEB: &str = "[web]\npackage = \"none\"\n\n";
const WAIT_OFF: &str = "[load]\nwait = \"false\"\n";
const NOT_WAITED: &str = "load: not waited for, [load] wait = false in .config/factory.toml";

fn scratch(name: &str, load: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join(".config")).expect("scratch dir");
    std::fs::write(dir.join(".config/factory.toml"), format!("{WEB}{load}")).expect("config");
    dir
}

/// How one run is set up.
struct Run<'a> {
    command: &'a str,
    load: &'a str,
    gate_env: Option<&'a str>,
    /// HOME, or the scratch directory when `None`.
    home: Option<&'a str>,
}

/// `xtask <command>` in a fresh scratch workspace; its stdout.
fn run(spec: &Run<'_>) -> String {
    let name = format!(
        "load-wait-{}-{}-{}-{}",
        spec.command,
        spec.gate_env.is_some(),
        spec.home.is_some(),
        spec.load.len()
    );
    let dir = scratch(&name, spec.load);
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"));
    child
        .arg(spec.command)
        .current_dir(&dir)
        .env("HOME", spec.home.map_or(dir.clone(), PathBuf::from))
        .env_remove("GITHUB_ACTIONS")
        .env_remove("XTASK_GATE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    if let Some(value) = spec.gate_env {
        child.env("XTASK_GATE", value);
    }
    let output = child.output().expect("xtask runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn plain(command: &str) -> Run<'_> {
    Run {
        command,
        load: WAIT_OFF,
        gate_env: None,
        home: None,
    }
}

#[test]
fn ci_calls_the_load_wait() {
    // When
    let text = run(&plain("ci"));

    // Then
    assert!(text.contains(&format!("ci: {NOT_WAITED}")), "{text}");
}

#[test]
fn gate_calls_the_load_wait() {
    // When
    let text = run(&plain("gate"));

    // Then
    assert!(text.contains(&format!("gate: {NOT_WAITED}")), "{text}");
}

#[test]
fn ci_inside_a_gate_does_not_wait_again() {
    // When
    let text = run(&Run {
        gate_env: Some("1"),
        ..plain("ci")
    });

    // Then: it went past where the wait would be (to sccache or a step,
    // not a refusal by `ci` itself), and said nothing of it
    let went_on = text.lines().any(|line| {
        line.starts_with("ok ") || (line.starts_with("FAIL ") && !line.starts_with("FAIL ci:"))
    });
    assert!(went_on, "{text}");
    assert!(!text.contains("load:"), "{text}");
}

// The gate waits once it holds its lock: with a HOME the lock refuses
// (not absolute), it stops before any load line.
#[test]
fn the_gate_takes_its_lock_before_it_waits() {
    // When
    let text = run(&Run {
        home: Some("relative-home"),
        ..plain("gate")
    });

    // Then
    assert!(
        text.contains("HOME is `relative-home`, not an absolute path"),
        "{text}"
    );
    assert!(!text.contains("load:"), "{text}");
}

// Past the deadline, the final summary repeats the wait's line. The
// threshold is below any real load (0.001) and the deadline 0.6 s.
#[test]
fn a_run_past_the_load_deadline_says_so_again_in_its_summary() {
    // When
    let text = run(&Run {
        load: "[load]\nthreshold = \"0.001\"\ndeadline_minutes = \"0.01\"\n",
        ..plain("ci")
    });

    // Then
    let still = text
        .lines()
        .filter(|line| line.contains("ci: load: STILL"))
        .count();
    assert_eq!(still, 2, "{text}");
}
