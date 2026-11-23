//! An xtask a gate's step runs builds with the jobs the gate gave the
//! step (`XTASK_GATE_JOBS`), not on every thread: `xtask lint` runs as a
//! process in a scratch workspace with a stand-in `cargo` (`support`),
//! which writes the environment it was given to `$HOME/cargo.env` and
//! fails, doing no work.

#[allow(
    dead_code,
    reason = "each test binary compiles `support` and uses part of it"
)]
mod support;

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long `xtask lint` may take to reach its stand-in `cargo` and
/// fail: the `quiet_wiring` children did so within 862 ms at load 17 to 24
/// (2026-11-14); 10 s is over ten times that.
const WITHIN: Duration = Duration::from_secs(10);

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("gate-jobs-{name}"));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join(".config")).expect("scratch dir");
    std::fs::write(
        dir.join(".config/factory.toml"),
        "[web]\npackage = \"none\"\n",
    )
    .expect("config");
    dir
}

/// The `CARGO_BUILD_JOBS` the stand-in `cargo` was given by `xtask lint`
/// run with `env`.
fn jobs_given(name: &str, env: &[(&str, &str)]) -> Option<String> {
    let dir = scratch(name);
    let tmp = dir.join("tmp");
    std::fs::create_dir_all(&tmp).expect("tmp");
    let bin = support::bin(&dir, support::NO_WORK);
    let mut child = support::xtask(&dir, &bin, &tmp);
    child.arg("lint").envs(env.iter().copied());
    support::output_within(child, WITHIN);
    let seen = std::fs::read_to_string(dir.join("cargo.env")).expect("the stand-in cargo ran");
    seen.lines()
        .find_map(|line| line.strip_prefix("CARGO_BUILD_JOBS="))
        .map(str::to_owned)
}

#[test]
fn an_xtask_in_a_gate_s_shared_step_builds_with_the_step_s_jobs() {
    // When
    let jobs = jobs_given("shared", &[("XTASK_GATE", "1"), ("XTASK_GATE_JOBS", "4")]);

    // Then
    assert_eq!(jobs.as_deref(), Some("4"));
}

#[test]
fn an_xtask_in_a_gate_s_step_without_the_jobs_variable_builds_on_every_thread() {
    // When
    let jobs = jobs_given("unset", &[("XTASK_GATE", "1")]);

    // Then
    let threads = std::thread::available_parallelism()
        .expect("threads")
        .to_string();
    assert_eq!(jobs, Some(threads));
}
