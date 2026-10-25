//! `cargo xtask serve`: the release build of main on the tailnet, where
//! `[serve]` in `.config/factory.toml` says (`serve_config.rs`). Run from
//! a clean checkout at `origin/main` after `cargo xtask web`. It refuses
//! another commit or a changed tree, runs `[serve] check` (touche's
//! `check_dist` refuses, among the rest, any file requesting the dev
//! server's keys), copies `target/web` to `[serve] dir` under the home
//! directory, which outlives any worktree, and serves that copy at
//! `[serve] https` and `path`. `cargo xtask dev` serves on its own port.

use std::path::Path;
use std::process::{Command, ExitCode};

use crate::config::Config;
use crate::gate::git;
use crate::serve_config;
use crate::tailscale::{Target, require_tailscale, served_url, start_serving};
use crate::web::DIST;

/// Serve the release build of main and print its URL.
pub fn run() -> ExitCode {
    match serve() {
        Ok(url) => {
            println!("ok   serve    {url} (the release build of main)");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL serve: {message}");
            println!("     reproduce: cargo xtask web && cargo xtask serve");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> Result<String, String> {
    let tailscale = require_tailscale()?;
    let home = std::env::var_os("HOME").ok_or("HOME is unset; fix: set it")?;
    let config = Config::load().map_err(|e| e.to_string())?;
    let settings = serve_config::serve(&config, Path::new(&home))?;
    let root = std::env::current_dir().map_err(|e| format!("no current directory: {e}"))?;
    require_main(&root.display().to_string())?;
    let dist = root.join(DIST);
    if !dist.join("index.html").is_file() {
        return Err(format!(
            "{} has no index.html; fix: cargo xtask web",
            dist.display()
        ));
    }
    run_check(&settings.check)?;
    publish(&dist, &settings.dir)?;
    start_serving(
        &tailscale,
        &Target::Directory(&settings.dir),
        &settings.mount,
    )?;
    served_url(&tailscale, &settings.mount)
}

/// The tree at `dir` is clean and at `origin/main`.
fn require_main(dir: &str) -> Result<(), String> {
    let head = git(dir, &["rev-parse", "HEAD"])?;
    let main = git(dir, &["rev-parse", "origin/main"])?;
    let dirty = !git(dir, &["status", "--porcelain"])?.is_empty();
    main_refusal(&head, &main, dirty).map_or(Ok(()), Err)
}

/// Why a checkout at `head` cannot be served as main, or `None`.
#[must_use]
pub fn main_refusal(head: &str, main: &str, dirty: bool) -> Option<String> {
    if head != main {
        return Some(format!(
            "HEAD is {head}, not origin/main {main}; fix: git fetch, then run this from a clean worktree at origin/main"
        ));
    }
    dirty.then(|| "the tree has changes, so the build is not main's; fix: commit or stash them, or use a clean worktree".to_owned())
}

/// `[serve] check`, run by `sh -c` with its output shown; refused
/// unless it exits 0.
fn run_check(check: &str) -> Result<(), String> {
    let status = Command::new("sh")
        .args(["-c", check])
        .status()
        .map_err(|e| format!("could not run `{check}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{check}` refused the built page ({status}); fix: what it names above, then cargo xtask web"
        ))
    }
}

/// Replace `served` with a copy of `dist`: the copy is made beside it and
/// swapped in by renames, so `served` is never half written.
pub fn publish(dist: &Path, served: &Path) -> Result<(), String> {
    let fresh = served.with_extension("new");
    let old = served.with_extension("old");
    for stale in [&fresh, &old] {
        remove(stale)?;
    }
    copy_dir(dist, &fresh)?;
    if served.exists() {
        rename(served, &old)?;
    }
    rename(&fresh, served)?;
    remove(&old)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("{}: {e}", from.display()))?
            .path();
        let target = to.join(path.file_name().ok_or("an entry with no name")?);
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)
                .map_err(|e| format!("{} -> {}: {e}", path.display(), target.display()))?;
        }
    }
    Ok(())
}

fn rename(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|e| format!("{} -> {}: {e}", from.display(), to.display()))
}

fn remove(dir: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {e}", dir.display()))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "serve_tests.rs"]
mod tests;
