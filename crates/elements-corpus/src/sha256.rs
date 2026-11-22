//! SHA-256 of bytes or a file, as `sha256sum` prints it, as xtask's
//! `download_install::sha256` takes a file's: the saved references'
//! hashes (`paint_reference`). `sha256sum` reads a file as its stdin, so
//! the one wait is for it to exit, held to [`DEADLINE`].

use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How long `sha256sum` may run before it is killed and the hash
/// refused. Measured on this machine on 2026-11-22 at load 14 to 18:
/// the pinned Chromium executable (533 MB, the largest input, hashed by a
/// capture) took 3.1 to 4.3 s; the fonts, page and PNG an ordinary run
/// hashes (171 KB at most) took 0.02 s at most. 30 s is about seven times
/// the slowest.
const DEADLINE: Duration = Duration::from_secs(30);

/// How often the wait looks whether `sha256sum` has exited.
const POLL: Duration = Duration::from_millis(5);

/// The SHA-256 of `bytes`, written to a temporary file for `sha256sum`.
///
/// # Errors
/// When the file cannot be written, or as [`sha256_file`].
pub fn sha256(bytes: &[u8]) -> Result<String, String> {
    let mut file = tempfile::tempfile().map_err(|e| format!("a file to hash: {e}"))?;
    file.write_all(bytes)
        .and_then(|()| file.rewind())
        .map_err(|e| format!("a file to hash: {e}"))?;
    digest_with("sha256sum", &[], file, DEADLINE)
}

/// The SHA-256 of the file at `path`, read by `sha256sum` itself.
///
/// # Errors
/// When the file cannot be opened, `sha256sum` is missing, fails, or runs
/// past [`DEADLINE`] (killed).
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    digest_with("sha256sum", &[], file, DEADLINE).map_err(|e| format!("{}: {e}", path.display()))
}

/// The hash `program` prints for `input`, its stdin, when it exits
/// within `deadline`.
fn digest_with(
    program: &str,
    args: &[&str],
    input: File,
    deadline: Duration,
) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::from(input))
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{program}: {e}; fix: install coreutils"))?;
    let status = wait_within(&mut child, deadline).map_err(|e| format!("{program}: {e}"))?;
    let mut text = String::new();
    child
        .stdout
        .take()
        .ok_or("its stdout was not piped")?
        .read_to_string(&mut text)
        .map_err(|e| format!("{program}: {e}"))?;
    match text.split_whitespace().next() {
        Some(hash) if status.success() && hash.len() == 64 => Ok(hash.to_owned()),
        _ => Err(format!("{program} failed with {status}")),
    }
}

/// `child`'s exit, or it killed and an error once `deadline` passes.
fn wait_within(child: &mut Child, deadline: Duration) -> Result<ExitStatus, String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(status);
        }
        if started.elapsed() >= deadline {
            let killed = child.kill().and_then(|()| child.wait());
            let killed = killed.map_or_else(|e| format!("not killed: {e}"), |_| "killed".into());
            return Err(format!(
                "ran past its {deadline:?} deadline ({killed}); fix: look for a stalled disk or \
                 an overloaded machine (sha256.rs DEADLINE gives the measurement)"
            ));
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
#[path = "sha256_tests.rs"]
mod tests;
