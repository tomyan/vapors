//! A pinned tool's checksum (`[checksums]` in `.config/tools.toml`): the
//! installed binary, found on `PATH` as the shell finds it, hashed with
//! `sha256sum` and compared. A tool that serves pages is held to the
//! exact binary its checksum was taken from.

use std::process::Command;

use crate::tools::Tool;

/// `Ok` when the tool has no checksum or its binary matches it.
///
/// # Errors
///
/// The binary not found, unreadable, or another binary, naming the fix.
pub fn verify(tool: &Tool) -> Result<(), String> {
    let Some(expected) = &tool.checksum else {
        return Ok(());
    };
    let path = located(&tool.binary)?;
    let found = sha256(&path)?;
    if &found == expected {
        return Ok(());
    }
    Err(format!(
        "{path} has SHA-256 {found}, not the {expected} pinned for {} {} in .config/tools.toml; fix: `cargo install --list` to see where it came from, then `cargo xtask setup` (or bump the pin in a pull request, saying where the new binary came from)",
        tool.crate_name, tool.version
    ))
}

fn located(binary: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .args(["-c", &format!("command -v {binary}")])
        .output()
        .map_err(|e| format!("sh: {e}"))?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if path.is_empty() {
        return Err(format!("{binary} is not on PATH; fix: cargo xtask setup"));
    }
    Ok(path)
}

/// The file's SHA-256 as `sha256sum` prints it.
pub fn sha256(path: &str) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|e| format!("sha256sum: {e}; fix: install coreutils"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    match text.split_whitespace().next() {
        Some(hash) if output.status.success() && hash.len() == 64 => Ok(hash.to_owned()),
        _ => Err(format!(
            "sha256sum {path} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

#[cfg(test)]
#[path = "setup_checksum_tests.rs"]
mod tests;
