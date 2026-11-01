//! `cargo xtask ci` and `cargo xtask gate` call the load wait, and a ci
//! run inside a gate does not: each runs as a process in a scratch
//! workspace whose `[load] wait = "false"` makes the wait say so without
//! reading the load. HOME is the scratch directory, so the gate lock is
//! its own and no real gate is waited on.

use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "[web]\npackage = \"none\"\n\n[load]\nwait = \"false\"\n";
const NOT_WAITED: &str = "load: not waited for, [load] wait = false in .config/factory.toml";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join(".config")).expect("scratch dir");
    std::fs::write(dir.join(".config/factory.toml"), CONFIG).expect("config");
    dir
}

/// `xtask <command>` in a fresh scratch workspace, with `gate_env` as
/// `XTASK_GATE` when given; its stdout.
fn run(command: &str, gate_env: Option<&str>) -> String {
    let dir = scratch(&format!("load-wait-{command}-{}", gate_env.is_some()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"));
    child
        .arg(command)
        .current_dir(&dir)
        .env("HOME", &dir)
        .env_remove("GITHUB_ACTIONS")
        .env_remove("XTASK_GATE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    if let Some(value) = gate_env {
        child.env("XTASK_GATE", value);
    }
    let output = child.output().expect("xtask runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn ci_calls_the_load_wait() {
    // When
    let text = run("ci", None);

    // Then
    assert!(text.contains(&format!("ci: {NOT_WAITED}")), "{text}");
}

#[test]
fn gate_calls_the_load_wait() {
    // When
    let text = run("gate", None);

    // Then
    assert!(text.contains(&format!("gate: {NOT_WAITED}")), "{text}");
}

#[test]
fn ci_inside_a_gate_does_not_wait_again() {
    // When
    let text = run("ci", Some("1"));

    // Then: it went past where the wait would be, and said nothing of it
    let went_on = text
        .lines()
        .any(|line| line.starts_with("ok ") || line.starts_with("FAIL "));
    assert!(went_on, "{text}");
    assert!(!text.contains("load:"), "{text}");
}
