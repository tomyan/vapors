//! A temp directory of Chromium's own for each launch.

use super::chromium::ChromiumError;

/// A new directory for one launch's `TMPDIR`, removed when dropped.
///
/// Chromium leaves directories in `TMPDIR` on every launch: an empty
/// `org.chromium.Chromium.chromium_chrome_url_fetcher_.*` each time and,
/// when it is killed, `org.chromium.Chromium.*` holding its singleton
/// socket. One vapors test run left 64 (2026-11-02), and /tmp's inodes ran
/// out that day. The directory is made under the temp dir, as short a
/// path as before: Chromium's socket path must fit in 108 bytes.
///
/// # Errors
///
/// [`ChromiumError::Failed`] when it cannot be made.
pub(super) fn launch_tmp() -> Result<tempfile::TempDir, ChromiumError> {
    tempfile::Builder::new()
        .prefix("chromium-")
        .tempdir()
        .map_err(|e| {
            let under = std::env::temp_dir();
            ChromiumError::Failed(format!("a TMPDIR for it under {}: {e}", under.display()))
        })
}

/// Contain `command`'s disk cache in `tmp`, the launch's own directory:
/// `XDG_CACHE_HOME` and `--disk-cache-dir` both name `tmp/cache`, so
/// nothing lands in the caller's `~/.cache` and it goes with `tmp`.
///
/// # Errors
///
/// [`ChromiumError::Failed`] when the directory cannot be made.
pub(super) fn contain_cache(
    command: &mut std::process::Command,
    tmp: &std::path::Path,
) -> Result<(), ChromiumError> {
    let cache = tmp.join("cache");
    std::fs::create_dir(&cache).map_err(|e| {
        ChromiumError::Failed(format!(
            "a disk cache for it at {}: {e}. Fix: free space or inodes under {}",
            cache.display(),
            std::env::temp_dir().display()
        ))
    })?;
    command
        .env("XDG_CACHE_HOME", &cache)
        .arg(format!("--disk-cache-dir={}", cache.display()));
    Ok(())
}

#[cfg(test)]
#[path = "chromium_tmp_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chromium_cache_tests.rs"]
mod cache_tests;
