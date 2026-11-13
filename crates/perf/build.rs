//! Compile the Mount workload's components, and record the revision the
//! harness is built from (`git describe --always --dirty`), which
//! `--scene-run` prints so a recorded run names the code it timed.
use std::process::Command;

fn main() {
    vapors_compiler::build::compile_dir("ui");
    println!("cargo:rustc-env=VAPORS_PERF_REVISION={}", revision());
    // the revision moves with every commit or checkout (HEAD's log), a
    // staged change (the index) and any edit to the tree
    for watched in [
        git(&["rev-parse", "--git-path", "logs/HEAD"]),
        git(&["rev-parse", "--git-path", "index"]),
    ]
    .into_iter()
    .flatten()
    .chain(["../../crates".into(), "../../Cargo.lock".into()])
    {
        println!("cargo:rerun-if-changed={watched}");
    }
}

/// `git args`'s output, or why it failed; git's own environment is
/// stripped, since a hook running cargo exports it.
fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|error| format!("git: {error}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The tree's revision, or `unknown (<why>)`.
fn revision() -> String {
    git(&["describe", "--always", "--dirty", "--abbrev=12"])
        .unwrap_or_else(|why| format!("unknown ({why})"))
}
