//! The last-level cache's line size, for `--counters`' estimate of the
//! bytes its misses bring in: read from sysfs, the highest-level data or
//! unified cache of cpu0, or refused, never assumed.

use std::path::Path;
use std::sync::OnceLock;

/// Where Linux describes cpu0's caches.
const CACHES: &str = "/sys/devices/system/cpu/cpu0/cache";

/// The last-level cache's line size in bytes, read once.
///
/// # Errors
/// The caches are not described, or a file does not hold what it
/// should; each names the path.
pub fn llc_line_bytes() -> Result<u64, String> {
    static LINE: OnceLock<Result<u64, String>> = OnceLock::new();
    LINE.get_or_init(|| line_bytes_under(Path::new(CACHES)))
        .clone()
}

/// The line size of the highest-level data or unified cache under
/// `caches` (its `index*` directories).
///
/// # Errors
/// As [`llc_line_bytes`].
pub fn line_bytes_under(caches: &Path) -> Result<u64, String> {
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    for entry in std::fs::read_dir(caches).map_err(|e| unreadable(caches, &e))? {
        let dir = entry.map_err(|e| unreadable(caches, &e))?.path();
        if !dir
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("index"))
        {
            continue;
        }
        if read(&dir.join("type"))? == "Instruction" {
            continue;
        }
        let level = number(&dir.join("level"))?;
        if best.as_ref().is_none_or(|(at, _)| level > *at) {
            best = Some((level, dir));
        }
    }
    let (_, dir) = best.ok_or_else(|| {
        format!(
            "{}: no data or unified cache is described; --counters cannot estimate LLC miss bytes",
            caches.display()
        )
    })?;
    number(&dir.join("coherency_line_size"))
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|text| text.trim().to_owned())
        .map_err(|e| unreadable(path, &e))
}

fn number(path: &Path) -> Result<u64, String> {
    let text = read(path)?;
    text.parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| format!("{}:1: `{text}` is not a number above 0", path.display()))
}

fn unreadable(path: &Path, error: &std::io::Error) -> String {
    format!(
        "{}: {error}; --counters reads the LLC's line size there to estimate the bytes its \
         misses bring in",
        path.display()
    )
}

#[cfg(test)]
#[path = "llc_line_tests.rs"]
mod tests;
