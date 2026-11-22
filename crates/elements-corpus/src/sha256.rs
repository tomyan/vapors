//! SHA-256 of bytes or a file, as `sha256sum` prints it, as xtask's
//! `download_install::sha256` takes a file's: the saved references'
//! hashes (`paint_reference`).

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// The SHA-256 of `bytes`, as `sha256sum` prints it (as xtask's
/// `download_install::sha256` takes a file's).
///
/// # Errors
/// When `sha256sum` is missing or fails.
pub fn sha256(bytes: &[u8]) -> Result<String, String> {
    let mut child = sha256sum(Stdio::piped())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("sha256sum: its stdin was not piped")?;
    stdin
        .write_all(bytes)
        .map_err(|e| format!("sha256sum: {e}"))?;
    drop(stdin);
    digest(child)
}

/// The SHA-256 of the file at `path`, read by `sha256sum` itself.
///
/// # Errors
/// When the file cannot be opened or `sha256sum` fails.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    digest(sha256sum(Stdio::from(file))?)
}

fn sha256sum(stdin: Stdio) -> Result<Child, String> {
    Command::new("sha256sum")
        .stdin(stdin)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("sha256sum: {e}; fix: install coreutils"))
}

/// The hash `child` prints. No deadline, as xtask's `sha256` has none: it
/// reads a file or a pipe this process has already closed.
fn digest(child: Child) -> Result<String, String> {
    let output = child
        .wait_with_output()
        .map_err(|e| format!("sha256sum: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    match text.split_whitespace().next() {
        Some(hash) if output.status.success() && hash.len() == 64 => Ok(hash.to_owned()),
        _ => Err(format!("sha256sum failed with {}", output.status)),
    }
}
