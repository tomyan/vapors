//! `cargo xtask ci`, `test` and `gate` wait while someone else holds the
//! quiet lock, and `cargo xtask quiet` takes and releases it. Each runs
//! as a process in a scratch workspace that is also its HOME, so the
//! locks are its own; `[load] wait = "false"` keeps the load out of it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CONFIG: &str = "[web]\npackage = \"none\"\n\n[load]\nwait = \"false\"\n";
/// A pid above Linux's largest `pid_max` (2^22), so never a live process.
const GONE: u32 = 4_194_305;
/// How long the scratch holder holds the lock.
const HELD: Duration = Duration::from_secs(2);

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("quiet-{name}"));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join(".config")).expect("scratch dir");
    std::fs::write(dir.join(".config/factory.toml"), CONFIG).expect("config");
    dir
}

fn holder_file(home: &Path) -> PathBuf {
    home.join(".cache/factory/quiet.holder")
}

/// Another process's hold on the lock in `home`, for [`HELD`] from now.
fn held_by_another(home: &Path) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    let until = (now + HELD).as_millis();
    std::fs::create_dir_all(home.join(".cache/factory")).expect("lock dir");
    let line = format!("pid {GONE} in /w/timed until {until} running a timed run\n");
    std::fs::write(holder_file(home), line).expect("holder");
}

fn xtask(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"));
    child.args(args).current_dir(dir).env("HOME", dir);
    for name in [
        "GITHUB_ACTIONS",
        "XTASK_GATE",
        "XTASK_QUIET_HOLDER",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
    ] {
        child.env_remove(name);
    }
    child.envs(env.iter().copied());
    child.output().expect("xtask runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn ci_test_and_gate_wait_for_another_holder_saying_who_and_until_when() {
    for (command, who) in [
        (&["ci"][..], "ci"),
        (&["test"][..], "test"),
        (&["gate"][..], "gate"),
    ] {
        // Given
        let dir = scratch(who);
        held_by_another(&dir);

        // When
        let started = Instant::now();
        let text = stdout(&xtask(&dir, command, &[]));

        // Then
        let line = format!(
            "{who}: waiting for the quiet lock, held by pid {GONE} in /w/timed running `a timed run`, expires in "
        );
        assert!(text.contains(&line), "{who}: {text}");
        assert!(
            started.elapsed() >= Duration::from_millis(1_500),
            "{who} went on early"
        );
    }
}

#[test]
fn the_holders_own_ci_run_does_not_wait() {
    // Given
    let dir = scratch("own");
    held_by_another(&dir);

    // When
    let text = stdout(&xtask(
        &dir,
        &["ci"],
        &[("XTASK_QUIET_HOLDER", &GONE.to_string())],
    ));

    // Then
    assert!(!text.contains("waiting for the quiet lock"), "{text}");
}

#[test]
fn quiet_runs_its_command_holding_the_lock_then_releases_it() {
    // Given
    let dir = scratch("run");
    let probe = format!("cat {}", holder_file(&dir).display());

    // When
    let output = xtask(&dir, &["quiet", "1", "--", "sh", "-c", &probe], &[]);

    // Then
    let text = stdout(&output);
    assert!(output.status.success(), "{text}");
    assert!(
        text.contains("running sh -c cat"),
        "the command ran under the lock: {text}"
    );
    assert!(!holder_file(&dir).exists(), "released");
}

#[test]
fn a_hold_outlives_its_command_until_released_from_its_worktree() {
    // Given
    let dir = scratch("hold");

    // When
    let held = xtask(&dir, &["quiet", "hold", "1"], &[]);
    let kept = holder_file(&dir).exists();
    let released = xtask(&dir, &["quiet", "release"], &[]);

    // Then
    assert!(held.status.success() && kept, "{}", stdout(&held));
    assert!(released.status.success(), "{}", stdout(&released));
    assert!(!holder_file(&dir).exists());
}
