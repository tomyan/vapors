//! The cap on a worktree's build directory: every build xtask starts
//! (each step, as with the compilation cache, `sccache.rs`) is refused
//! while the directory cargo builds into holds more than
//! `[targets] max_gb` in `.config/factory.toml`, naming the size, how
//! to see it, and the command that empties it. The limit's source is
//! CLAUDE.md's Builds section.
//!
//! The directory is the `CARGO_TARGET_DIR` the guarded command is given
//! (`sccache::apply`): a step's own, from its `env` (taken from the
//! step's directory when relative), else xtask's, else the workspace's
//! `target/`. RELAXED: what the cap cannot see is a `--target-dir`
//! argument, or a `CARGO_TARGET_DIR` set or exported inside a step's
//! script; a step doing either is measured on the directory its
//! environment names, not the one it builds into. No step does today;
//! retired by refusing a planned step whose arguments or `run:` text
//! set either, naming the file and line. The workspace is the nearest directory at or above the
//! current one holding `.config/factory.toml`, so a run from a
//! subdirectory is capped too. A `target-dir` in the workspace's
//! `.cargo/config.toml` is refused, as it is not read here; one in a
//! user-level `~/.cargo/config.toml` is not detected.
//!
//! The size is measured cheaply, without walking the whole tree: in each
//! `target/<profile>` and `target/<triple>/<profile>`, the files directly
//! in `deps` and `examples` (one listing each), and `incremental` to the
//! depth cargo writes it (crate, session, file), summed as allocated
//! blocks (as `du` counts). `incremental` is empty for xtask's builds
//! and `.cargo/config.toml`'s; a plain build with `CARGO_INCREMENTAL=1`
//! fills it. Measured 2026-11-16 on six of the dev box's targets, `deps`
//! alone held 82% to 96% of what `du -sb` counted and `examples` most of
//! the rest (10.1 GB beside 97.5 in the largest), read in 2 to 34 ms
//! where `du` took 9 to 400 ms warm and more cold. What is left out
//! (`build/` script outputs, `.fingerprint/`, `tmp/`) is not bounded by
//! this measure.
//!
//! The size is checked when each step starts, not during one: a step
//! started under the cap can end over it, and the next step is refused.

use std::path::Path;

const GB: u64 = 1_000_000_000;
const SETTINGS: &str = ".config/factory.toml";
/// The subdirectories of a profile directory counted, and how deep
/// below each its files are: `incremental/<crate>/<session>/<file>`.
const COUNTED: [(&str, usize); 3] = [("deps", 1), ("examples", 1), ("incremental", 3)];

/// Refuse when the build directory, at `cargo_target_dir` when set
/// (`command_target_dir`), holds more than the configured cap; a
/// repository that sets none is not capped, said once per run.
///
/// # Errors
/// Over the cap; no settings file at or above the current directory;
/// the cap not a number above 0; a `target-dir` in `.cargo/config.toml`;
/// or a directory or file that could not be read, each naming the fix.
pub fn check(cargo_target_dir: Option<&std::ffi::OsStr>) -> Result<(), String> {
    let here = std::env::current_dir().map_err(|e| format!("the current directory: {e}"))?;
    let root = workspace_root(&here)?;
    let config = crate::config::Config::load_from(&root).map_err(|e| e.to_string())?;
    let Some(max_gb) = cap(config.get("targets.max_gb"))? else {
        uncapped_once();
        return Ok(());
    };
    let Some(target) = target_dir(&root, cargo_target_dir)? else {
        return Ok(()); // nothing built yet
    };
    over(&target, measure(&target)?, max_gb)
}

#[path = "target_cap_find.rs"]
mod find;
#[path = "target_cap_measure.rs"]
mod measure;
pub use find::{step_target_dir, target_dir, workspace_root};
pub use measure::measure;

/// `[targets] max_gb` as a number of GB; `None` when unset.
///
/// # Errors
/// Set to anything but a finite number above 0.
pub fn cap(value: Option<&str>) -> Result<Option<f64>, String> {
    let Some(text) = value else {
        return Ok(None);
    };
    match text.parse::<f64>() {
        Ok(gb) if gb.is_finite() && gb > 0.0 => Ok(Some(gb)),
        _ => Err(format!(
            "{SETTINGS}: [targets] max_gb = \"{text}\" is not a number of GB above 0; fix: set it so, such as \"80\""
        )),
    }
}

/// Refuse when `held` bytes pass `max_gb`, naming how to see the size
/// and the exact command that empties the directory.
///
/// # Errors
/// Over the cap.
#[allow(
    clippy::cast_precision_loss,
    reason = "bytes of a disk, far inside f64's exact range"
)]
pub fn over(target: &Path, held: u64, max_gb: f64) -> Result<(), String> {
    if held as f64 <= max_gb * GB as f64 {
        return Ok(());
    }
    let real = quoted(
        &target
            .canonicalize()
            .unwrap_or_else(|_| target.to_path_buf()),
    );
    Err(format!(
        "{} holds {:.1} GB of build products (deps, examples and incremental under each profile), \
         over [targets] max_gb = {max_gb} in {SETTINGS}; see it with `du -sh {real}`; \
         fix: `find {real} -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +` (empties it and keeps the \
         directory, so the worktree's target link stays valid; if another worktree's target links \
         there too, stop its build first), then rerun the xtask command",
        target.display(),
        held as f64 / GB as f64,
    ))
}

/// `path` in single quotes for a shell, any quote in it escaped.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

fn uncapped_once() {
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
        println!("targets: no [targets] max_gb in {SETTINGS}; the build directory is not capped");
    });
}

#[cfg(test)]
#[path = "target_cap_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "target_cap_bypass_tests.rs"]
mod bypass_tests;

#[cfg(test)]
#[path = "target_cap_command_tests.rs"]
mod command_tests;
