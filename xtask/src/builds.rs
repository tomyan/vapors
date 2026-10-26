//! `cargo xtask check-builds` (the `builds` step of `cargo xtask lint`,
//! so the gate runs it): no shipped build's dependency tree holds a crate
//! only the hosted or corporate product may use. Cargo features are not the
//! boundary, since they unify across a workspace
//! (docs/decisions/2026-10-26-product-builds-separate.md in touche).
//! Each build in `[server_only.builds]` is read with
//! `cargo tree -e normal -p <package> [--target <triple>]`; a server-only
//! crate in it fails the step, named with the path that pulled it in.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use crate::builds_config::{Build, Separation};
use crate::builds_tree::pulled_in;
use crate::config::Config;

/// `cargo tree` for touche-web's wasm tree took 0.54 s warm on the dev
/// machine (2026-10-26); a cold run fetches the index, as `cargo
/// metadata`'s 10.5 s cold run did (layers.rs). 60 s is about 6 times
/// that.
const TREE_DEADLINE: Duration = Duration::from_secs(60);

/// Check every shipped build and report.
pub fn run() -> ExitCode {
    match check() {
        Ok(lines) => {
            for line in lines {
                println!("ok   builds: {line}");
            }
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for line in problems {
                println!("FAIL builds: {line}");
            }
            println!("     reproduce: cargo xtask check-builds");
            ExitCode::FAILURE
        }
    }
}

fn check() -> Result<Vec<String>, Vec<String>> {
    let config = Config::load().map_err(|e| vec![e.to_string()])?;
    let separation = Separation::from(&config).map_err(|e| vec![e])?;
    let (mut said, mut problems) = (Vec::new(), Vec::new());
    for build in &separation.builds {
        match check_build(build, &separation.crates) {
            Ok(line) => said.push(line),
            Err(found) => problems.extend(found),
        }
    }
    if problems.is_empty() {
        Ok(said)
    } else {
        Err(problems)
    }
}

/// One build's tree against the server-only crates.
fn check_build(build: &Build, server_only: &[String]) -> Result<String, Vec<String>> {
    let shown = described(build);
    let tree = tree(build).map_err(|why| vec![format!("{shown}: {why}")])?;
    let found = pulled_in(&tree, server_only).map_err(|why| vec![format!("{shown}: {why}")])?;
    if found.is_empty() {
        return Ok(format!(
            "{shown} holds none of the {} server-only crates",
            server_only.len()
        ));
    }
    Err(found
        .iter()
        .map(|crate_and_path| {
            format!("{shown} ships the server-only crate {crate_and_path}; fix: remove that dependency, or move the code out of what this build depends on")
        })
        .collect())
}

fn described(build: &Build) -> String {
    let target = build.target.as_deref().unwrap_or("host");
    format!("{} ({}, {target})", build.name, build.package)
}

/// `build`'s tree, `cargo tree`'s stdout written to a file so none of it
/// is dropped, within `TREE_DEADLINE`.
fn tree(build: &Build) -> Result<String, String> {
    let out = std::env::temp_dir().join(format!(
        "xtask-builds-{}-{}",
        build.name,
        std::process::id()
    ));
    let ran = run_tree(&tree_command(build), &out);
    let text = std::fs::read_to_string(&out);
    let _ = std::fs::remove_file(&out);
    ran?;
    text.map_err(|e| format!("{}: {e}", out.display()))
}

/// The `cargo tree` call for `build`.
#[must_use]
pub fn tree_command(build: &Build) -> Vec<String> {
    let mut args: Vec<String> = ["tree", "-e", "normal", "-p", &build.package]
        .map(str::to_owned)
        .to_vec();
    if let Some(target) = &build.target {
        args.extend(["--target".to_owned(), target.clone()]);
    }
    args.extend(["--prefix", "depth", "--format", "{p}"].map(str::to_owned));
    args
}

fn run_tree(args: &[String], out: &Path) -> Result<(), String> {
    let file = std::fs::File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut child = Command::new("cargo")
        .args(args)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start cargo tree: {e}"))?;
    let started = Instant::now();
    while started.elapsed() < TREE_DEADLINE {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            let output = child.wait_with_output().map_err(|e| e.to_string())?;
            return refused_unless(&output, args);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(format!(
        "`cargo {}` did not finish within {TREE_DEADLINE:?}",
        args.join(" ")
    ))
}

fn refused_unless(output: &std::process::Output, args: &[String]) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "`cargo {}` failed: {}",
        args.join(" "),
        stderr.trim()
    ))
}

#[cfg(test)]
#[path = "builds_tests.rs"]
mod tests;
