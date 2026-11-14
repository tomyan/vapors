//! Running the xtask binary in a scratch directory that cannot reach
//! the workspace: its HOME, its own TMPDIR, and a PATH holding only a
//! stand-in `cargo`, the real `sccache` and the system's tools. Without
//! this, a child `xtask ci` or `test` that got past its wait found the
//! workspace above `target/tmp` (where `target` is a plain directory)
//! and ran the real tiers, which ran these tests again: the `quiet_wiring`
//! timeouts and nested leftovers of issue #358.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What the stand-in `cargo` does when a child gets past its wait and
/// starts a step: say so and fail, doing no work.
pub const NO_WORK: &str = "#!/bin/sh\necho \"stand-in cargo: $*\"\nexit 1\n";

/// Variables that would let a child find a toolchain or a repository.
const STRIPPED: [&str; 12] = [
    "GITHUB_ACTIONS",
    "XTASK_GATE",
    "XTASK_QUIET_HOLDER",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "CARGO",
    "CARGO_HOME",
    "CARGO_TARGET_DIR",
    "RUSTC_WRAPPER",
];

/// `dir/bin` holding `cargo` (the script `cargo`) and a link to the
/// sccache on this process's PATH, which every step needs.
pub fn bin(dir: &Path, cargo: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin dir");
    let script = bin.join("cargo");
    std::fs::write(&script, cargo).expect("stand-in cargo");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("mode");
    let sccache = on_path("sccache").expect("sccache on PATH; fix: cargo xtask setup");
    let link = bin.join("sccache");
    if !link.exists() {
        std::os::unix::fs::symlink(sccache, link).expect("sccache link");
    }
    bin
}

fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// The xtask binary run in `dir` as its HOME, with `bin` before the
/// system's directories on PATH, `tmp` as TMPDIR, and git unable to
/// look above `dir`.
pub fn xtask(dir: &Path, bin: &Path, tmp: &Path) -> Command {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"));
    child.current_dir(dir).env("HOME", dir).env("TMPDIR", tmp);
    for name in STRIPPED {
        child.env_remove(name);
    }
    let parent = dir.parent().expect("a scratch dir has a parent");
    child
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("GIT_CEILING_DIRECTORIES", parent);
    child
}

/// Every entry under `dir`, for an assertion that it is empty.
pub fn entries(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("readable").flatten() {
        found.push(entry.path().display().to_string());
    }
    found
}

/// `command`'s output, killing its process group and failing the test,
/// naming `limit` and what it printed, when it runs longer than `limit`.
/// A child that went on to real work would otherwise run until nextest's
/// kill, leaving its run directory (issue #358).
pub fn output_within(mut command: Command, limit: std::time::Duration) -> std::process::Output {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    let started = std::time::Instant::now();
    let mut child = command
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("xtask starts");
    while child.try_wait().expect("waitable").is_none() {
        if started.elapsed() > limit {
            kill_group(child.id());
            let output = child.wait_with_output().expect("killed child");
            panic!(
                "still running after {limit:?}, killed: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    child.wait_with_output().expect("output")
}

/// SIGKILL to the group `leader` leads, through kill(1): the crate
/// allows no `unsafe`.
fn kill_group(leader: u32) {
    let status = Command::new("kill")
        .args(["-KILL", "--", &format!("-{leader}")])
        .status()
        .expect("kill runs");
    assert!(status.success(), "kill -KILL -{leader} failed");
}
