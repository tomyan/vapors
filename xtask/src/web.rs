//! `cargo xtask web`: build the web package with trunk into `target/web`
//! and say where it went and how big it is.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::config::Config;
use crate::deadline::{Finished, run_with_deadline};

/// Where trunk writes the page, relative to the workspace root.
pub const DIST: &str = "target/web";

/// How long the trunk build may take. A cold release build with the
/// wasm-opt download measured 45 s on the dev box on 2026-10-22; CI
/// runners are slower and download more, hence the margin.
const BUILD_DEADLINE: Duration = Duration::from_secs(600);

/// Build the package named by `web.package` and report the dist directory.
pub fn run() -> ExitCode {
    match build() {
        Ok(line) => {
            println!("ok   web      {line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL web: {message}");
            println!("     reproduce: cargo xtask web");
            ExitCode::FAILURE
        }
    }
}

fn build() -> Result<String, String> {
    let package = web_package()?;
    let crate_dir = package_dir(&package)?;
    let dist = std::env::current_dir()
        .map_err(|e| format!("no current directory: {e}"))?
        .join(DIST);
    let mut command = Command::new("trunk");
    command
        .args(["build", "--release", "--dist"])
        .arg(&dist)
        .current_dir(&crate_dir);
    match run_with_deadline(command, BUILD_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(format!("{DIST} {}", size_of(&dist)?)),
        Finished::Exited { output, .. } => Err(format!(
            "`trunk build --release` in {} failed:\n{output}",
            crate_dir.display()
        )),
        Finished::TimedOut => Err(format!(
            "trunk did not finish within {}s",
            BUILD_DEADLINE.as_secs()
        )),
    }
}

/// `web.package` from `.config/factory.toml`.
pub fn web_package() -> Result<String, String> {
    Config::load()
        .and_then(|c| c.require("web.package").map(str::to_owned))
        .map_err(|e| e.to_string())
}

/// The directory of `package`, from `cargo pkgid`, whose answer for a
/// path dependency is `path+file:///<dir>#<name>@<version>`.
pub fn package_dir(package: &str) -> Result<PathBuf, String> {
    let output = Command::new("cargo")
        .args(["pkgid", "-p", package])
        .output()
        .map_err(|e| format!("could not run cargo pkgid: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo pkgid -p {package} failed: {}fix: name a workspace package in `[web] package`",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    id.strip_prefix("path+file://")
        .and_then(|rest| rest.split_once('#'))
        .map(|(dir, _)| PathBuf::from(dir))
        .ok_or_else(|| {
            format!("cargo pkgid gave `{id}`, not `path+file:///<dir>#<name>@<version>`")
        })
}

/// Total bytes and file count of a flat directory, as `616K (3 files)`.
/// Dotfiles (trunk's lock) are not part of the page.
fn size_of(dir: &Path) -> Result<String, String> {
    let mut bytes = 0;
    let mut files = 0;
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        if metadata.is_file() && !hidden {
            bytes += metadata.len();
            files += 1;
        }
    }
    Ok(format!("{}K ({files} files)", bytes.div_ceil(1024)))
}
