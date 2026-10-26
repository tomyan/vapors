//! `queue-pass.sh` as a process: a failure before `cargo xtask queue`
//! starts goes to the queue's log as well as to stderr, since the foreman
//! reads the log and not the journal.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/queue-pass.sh");

/// A home directory of its own for one test, empty.
fn scratch_home(name: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("xtask-queue-pass-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(home.join(".cache/w-queue/vapors-main")).expect("scratch home");
    home
}

/// The script run for vapors with `home` as HOME, git's hook variables
/// stripped.
fn run_script(home: &Path) -> Output {
    Command::new("bash")
        .args([SCRIPT, "vapors", "--dry-run"])
        .env("HOME", home)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("bash runs")
}

fn log_of(home: &Path) -> String {
    std::fs::read_to_string(home.join(".cache/queue/vapors.log")).unwrap_or_default()
}

#[test]
fn a_missing_main_worktree_is_logged_naming_the_fix() {
    // Given: no worktree at ~/.cache/w-queue/vapors-main/vapors
    let home = scratch_home("missing");

    // When
    let output = run_script(&home);

    // Then
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let log = log_of(&home);
    assert!(log.contains("FAILED queue-pass.sh: "), "{log}");
    assert!(
        log.contains("is not a worktree; fix: git -C ~/projects/vapors worktree add"),
        "{log}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_command_failing_inside_the_script_is_logged_with_its_code() {
    // Given: a repository there with no origin, so its fetch fails
    let home = scratch_home("fetch");
    let main = home.join(".cache/w-queue/vapors-main/vapors");
    let init = Command::new("git")
        .args(["init", "-q"])
        .arg(&main)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .status();
    assert!(init.expect("git runs").success());

    // When
    let output = run_script(&home);

    // Then
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let log = log_of(&home);
    assert!(log.contains("FAILED queue-pass.sh: line "), "{log}");
    assert!(log.contains("exited 128"), "{log}");
    let _ = std::fs::remove_dir_all(&home);
}
