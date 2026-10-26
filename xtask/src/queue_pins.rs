//! A worktree's pinned siblings (`[gate.pinned]`: a sibling checkout and
//! the file naming its pin), moved to their pins before the queue gates
//! or publishes from it, as an agent's recipe worktree has them.

use std::path::Path;

use crate::config::Config;

/// Fetch each pinned sibling of `dir` and check it out at its pin.
///
/// # Errors
///
/// The config, a pin file, or git refusing, naming which.
pub fn move_siblings(dir: &Path) -> Result<(), String> {
    let config = Config::load_from(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (sibling, pin_file) in config.entries_under("gate.pinned.") {
        let text = std::fs::read_to_string(dir.join(&pin_file))
            .map_err(|e| format!("{}: {e}", dir.join(&pin_file).display()))?;
        let pin = pin_in(&text, &pin_file)?;
        let sibling = dir.join(&sibling);
        crate::queue_remote::fetch(&sibling, &[&pin])?;
        crate::queue_git::git(&sibling, &["checkout", "-q", "--detach", &pin])?;
    }
    Ok(())
}

/// The pin in a pin file's text: its first line that is not a comment.
///
/// # Errors
///
/// No such line, naming the file.
pub fn pin_in(text: &str, file: &str) -> Result<String, String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .ok_or_else(|| format!("{file} names no pin; fix: put the commit on a line of its own"))
}

#[cfg(test)]
#[path = "queue_pins_tests.rs"]
mod tests;
