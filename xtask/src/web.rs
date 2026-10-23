//! `cargo xtask web`: build the web package with trunk into `target/web`
//! and say where it went and how big it is. `--debug` builds the debug
//! page into `target/web-debug` instead, for checks of what only a debug
//! build does (`check_dev_keys`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::config::Config;
use crate::deadline::{Finished, run_with_deadline};

/// Where trunk writes the page, relative to the workspace root.
pub const DIST: &str = "target/web";

/// Where trunk writes the debug page.
pub const DEBUG_DIST: &str = "target/web-debug";

/// One trunk build: release or debug, and its directory.
pub struct Build {
    /// Debug (no `--release`).
    pub debug: bool,
    /// The directory, relative to the workspace root.
    pub dist: &'static str,
}

impl Build {
    /// The release page, or with `debug` the debug page.
    #[must_use]
    pub fn of(debug: bool) -> Self {
        let dist = if debug { DEBUG_DIST } else { DIST };
        Self { debug, dist }
    }

    /// trunk's arguments, up to the dist directory. The public URL is
    /// relative (`./`), so the page loads its script and wasm from beside
    /// itself and works at `/` and under a path such as `/vapors/`;
    /// trunk's default `/` loads them from the server root, and under
    /// `/vapors/` the page stays at `starting` (vapors #32, checked by hand
    /// on 2026-10-23, trunk 0.22.0, Chromium 155).
    #[must_use]
    pub fn trunk_args(&self) -> Vec<&'static str> {
        if self.debug {
            vec!["build", "--public-url", "./", "--dist"]
        } else {
            vec!["build", "--release", "--public-url", "./", "--dist"]
        }
    }
}

/// How long the trunk build may take. A cold release build with the
/// wasm-opt download measured 45 s on the dev box on 2026-10-22; CI
/// runners are slower and download more, hence the margin.
const BUILD_DEADLINE: Duration = Duration::from_secs(600);

/// Build the package named by `web.package` and report the dist directory.
pub fn run(debug: bool) -> ExitCode {
    let build = Build::of(debug);
    match build_page(&build) {
        Ok(line) => {
            println!("ok   web      {line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL web: {message}");
            println!(
                "     reproduce: cargo xtask web{}",
                if debug { " --debug" } else { "" }
            );
            ExitCode::FAILURE
        }
    }
}

fn build_page(build: &Build) -> Result<String, String> {
    let package = web_package()?;
    let crate_dir = package_dir(&package)?;
    let dist = std::env::current_dir()
        .map_err(|e| format!("no current directory: {e}"))?
        .join(build.dist);
    let mut command = Command::new("trunk");
    crate::sccache::apply(&mut command)?;
    command
        .args(build.trunk_args())
        .arg(&dist)
        .current_dir(&crate_dir);
    match run_with_deadline(command, BUILD_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(format!("{} {}", build.dist, size_of(&dist)?)),
        Finished::Exited { output, .. } => Err(format!(
            "`trunk {}` in {} failed:\n{output}",
            build.trunk_args().join(" "),
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

#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;
