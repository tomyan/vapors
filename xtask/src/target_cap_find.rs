//! Where the capped build directory is (`target_cap.rs`).

use std::path::{Path, PathBuf};

use super::SETTINGS;

/// The nearest directory at or above `here` holding the settings file.
///
/// # Errors
/// None found, naming where it looked.
pub fn workspace_root(here: &Path) -> Result<PathBuf, String> {
    here.ancestors()
        .find(|dir| dir.join(SETTINGS).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "no {SETTINGS} in {} or any directory above it; fix: run cargo xtask from inside the workspace",
                here.display()
            )
        })
}

/// The `CARGO_TARGET_DIR` `command` builds with: its own when it sets
/// one, none when it removes it, else the one xtask inherited.
pub fn command_target_dir(command: &std::process::Command) -> Option<std::ffi::OsString> {
    let own = command
        .get_envs()
        .find(|(name, _)| *name == "CARGO_TARGET_DIR");
    match own {
        Some((_, value)) => value.map(std::ffi::OsStr::to_os_string),
        None => std::env::var_os("CARGO_TARGET_DIR"),
    }
}

/// The directory cargo builds into: `cargo_target_dir` when set (taken
/// from the current directory when relative, as cargo takes it), else
/// `<root>/target`; `None` when it does not exist yet.
///
/// # Errors
/// A `target-dir` in `<root>/.cargo/config.toml`, which is not read
/// here; a target link to nothing.
pub fn target_dir(
    root: &Path,
    cargo_target_dir: Option<&std::ffi::OsStr>,
) -> Result<Option<PathBuf>, String> {
    if let Some(dir) = cargo_target_dir.filter(|dir| !dir.is_empty()) {
        let dir = PathBuf::from(dir);
        return Ok(dir.exists().then_some(dir));
    }
    refuse_configured_dir(root)?;
    let found = crate::gate::floor::target_dir(root)?;
    Ok((found != root).then_some(found))
}

fn refuse_configured_dir(root: &Path) -> Result<(), String> {
    let config = root.join(".cargo/config.toml");
    let text = match std::fs::read_to_string(&config) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("{}: {e}; fix: make it readable", config.display())),
    };
    match text
        .lines()
        .position(|line| line.trim_start().starts_with("target-dir"))
    {
        None => Ok(()),
        Some(at) => Err(format!(
            "{}:{}: a target-dir the build directory's cap does not read; fix: set CARGO_TARGET_DIR instead, which it does",
            config.display(),
            at + 1
        )),
    }
}
