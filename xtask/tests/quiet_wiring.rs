//! `cargo xtask ci`, `test` and `gate` wait while someone else holds the
//! quiet lock, and `cargo xtask quiet` takes and releases it. Each runs
//! as a process in a scratch workspace that is also its HOME, so the
//! locks are its own; `[load] wait = "false"` keeps the load out of it.
//! A child that gets past its wait finds only a stand-in `cargo` and no
//! repository (`support`), so it fails its first step at once instead of
//! running the real tiers, which ran this test again (issue #358).

#[allow(
    dead_code,
    reason = "each test binary compiles `support` and uses part of it"
)]
mod support;

use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CONFIG: &str = "[web]\npackage = \"none\"\n\n[load]\nwait = \"false\"\n";
/// A pid above Linux's largest `pid_max` (2^22), so never a live process.
const GONE: u32 = 4_194_305;
/// How long the scratch holder holds the lock: long enough that a child
/// xtask started on a loaded machine still finds it held.
const HELD: Duration = Duration::from_secs(5);

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
fn held_by_another(home: &Path) -> u128 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    let (since, until) = (now.as_secs(), (now + HELD).as_millis());
    std::fs::create_dir_all(home.join(".cache/factory")).expect("lock dir");
    let line =
        format!("pid {GONE} in /w/timed since {since} until {until} as hold running a timed run\n");
    std::fs::write(holder_file(home), line).expect("holder");
    until
}

/// The longest a child may take from the holder's deadline to its exit:
/// it polls every 100 ms (`quiet_lock.rs`, `POLL`), then fails its first
/// step against the stand-in `cargo`. Measured 2026-11-14 over 10 runs
/// of this test at 1-minute load 17 to 24 (gates running): ci at most
/// 862 ms after the deadline, test 436 ms, gate 460 ms. 4 s is about
/// four and a half times the largest, so a loaded hook run passes, and a
/// child that starts real work is killed and named here instead of at
/// nextest's 60 s.
const AFTER_WAIT: Duration = Duration::from_secs(4);

/// The xtask binary in `dir` with its own TMPDIR, which must be empty
/// after it; killed and failed past [`HELD`] plus [`AFTER_WAIT`].
fn xtask(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let tmp = dir.join("tmp");
    std::fs::create_dir_all(&tmp).expect("tmp");
    let bin = support::bin(dir, support::NO_WORK);
    let mut child = support::xtask(dir, &bin, &tmp);
    child.args(args).envs(env.iter().copied());
    let output = support::output_within(child, HELD + AFTER_WAIT);
    assert_eq!(
        support::entries(&tmp),
        Vec::<String>::new(),
        "{args:?} left"
    );
    output
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
        let until = held_by_another(&dir);

        // When
        let text = stdout(&xtask(&dir, command, &[]));

        // Then
        let line = format!(
            "{who}: waiting for the quiet lock, held by pid {GONE} in /w/timed running `a timed run` (hold), expires in "
        );
        assert!(text.contains(&line), "{who}: {text}");
        let ended = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
        assert!(
            ended.as_millis() >= until,
            "{who} went on before the deadline"
        );
        let after = ended.as_millis() - until;
        println!("{who}: ended {after} ms after the deadline");
        assert!(
            after <= AFTER_WAIT.as_millis(),
            "{who} ran {after} ms past the deadline, over {AFTER_WAIT:?}: it did work after its wait; {text}"
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

#[test]
fn release_refuses_a_gate_s_hold_even_from_its_worktree() {
    // Given: a live gate (this test's process) holds it from the scratch dir
    let dir = scratch("release-gate");
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock");
    let until = (now + Duration::from_secs(60)).as_millis();
    let line = format!(
        "pid {} in {} since {} until {until} as gate running gate\n",
        std::process::id(),
        dir.canonicalize().expect("the scratch dir").display(),
        now.as_secs()
    );
    std::fs::create_dir_all(dir.join(".cache/factory")).expect("lock dir");
    std::fs::write(holder_file(&dir), line).expect("holder");

    // When
    let output = xtask(&dir, &["quiet", "release"], &[]);

    // Then
    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("not a hold from"),
        "{}",
        stdout(&output)
    );
    assert!(holder_file(&dir).exists());
}
