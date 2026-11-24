//! The gate's in-process parts are watched (`gate_watchdog.rs`): with a
//! FIFO where the gate reads or writes, the part blocks, and the gate
//! fails at the part's deadline naming it, exit code 1. Each runs the
//! gate in a scratch repository of one step (`support::gate`), HOME
//! there, so no real lock is waited on.

#[allow(
    dead_code,
    unused_imports,
    reason = "each test binary compiles `support` and uses part of it"
)]
mod support;

use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

const SETTINGS: &str = "[web]\npackage = \"none\"\n\n[load]\nwait = \"false\"\n";

/// The gate's wait for its child: the part's 5 s deadline
/// (`gate_watchdog::IN_PROCESS`) and the scratch gate's start before it,
/// under 1 s in the quiet and load wiring tests; 30 s in all.
const WITHIN: Duration = Duration::from_secs(30);

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("watchdog-{name}"));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join(".config")).expect("scratch dir");
    dir
}

/// A FIFO at `path`, in place of what is there.
fn fifo(path: &Path) {
    drop(std::fs::remove_file(path));
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("its directory");
    let made = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo {}", path.display());
}

/// `xtask gate` in `dir`.
fn gate(dir: &Path) -> Output {
    let tmp = dir.join("tmp");
    std::fs::create_dir_all(&tmp).expect("tmp");
    let bin = support::bin(dir, support::NO_WORK);
    let mut child = support::xtask(dir, &bin, &tmp);
    child.arg("gate");
    support::output_within(child, WITHIN)
}

/// The gate failed, its last line naming `part` and its 5 s deadline.
fn failed_naming(output: &Output, part: &str) {
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{text}");
    let line = format!("FAIL gate: {part} did not finish within 5s");
    assert!(text.lines().any(|l| l.starts_with(&line)), "{text}");
}

#[test]
fn a_stuck_read_of_the_settings_fails_the_gate_naming_it() {
    // Given
    let dir = scratch("settings");
    support::gate::workspace(&dir, SETTINGS, "shared");
    fifo(&dir.join(".config/factory.toml"));

    // When / Then
    failed_naming(&gate(&dir), "the settings");
}

#[test]
fn a_stuck_read_of_the_twin_s_files_fails_the_gate_naming_the_twin_check() {
    // Given: a twin whose one xtask file cannot be read
    let dir = scratch("twin");
    let twin = dir.join("twin");
    std::fs::create_dir_all(dir.join("xtask")).expect("ours");
    std::fs::write(dir.join("xtask/a"), "a").expect("our file");
    fifo(&twin.join("xtask/a"));
    let key = format!("twin = \"{}\"\n", twin.display());
    support::gate::workspace_with(&dir, SETTINGS, "shared", (&key, support::gate::WORKFLOW));

    // When / Then
    failed_naming(&gate(&dir), "the twin check");
}

#[test]
fn a_stuck_read_of_a_composite_action_fails_the_gate_naming_the_plan() {
    // Given: the workflow uses a local action whose file cannot be read
    let dir = scratch("plan");
    let workflow = support::gate::WORKFLOW.replace(
        "      - name: check\n",
        "      - uses: ./actions/stuck\n      - name: check\n",
    );
    fifo(&dir.join("actions/stuck/action.yml"));
    support::gate::workspace_with(&dir, SETTINGS, "shared", ("", &workflow));

    // When / Then
    failed_naming(&gate(&dir), "the workflow, its plan and its holds");
}

#[test]
fn a_stuck_write_of_the_github_output_file_fails_the_gate_naming_it() {
    // Given
    let dir = scratch("output");
    support::gate::workspace(&dir, SETTINGS, "shared");
    fifo(&dir.join("target/gate/github_output"));

    // When / Then
    failed_naming(&gate(&dir), "the GITHUB_OUTPUT file");
}
