//! Removing the directories of runs that are not going: a run killed or
//! panicking cannot remove its own. Only this user's directories are
//! touched; this run's own pid names a directory left by an earlier run
//! with that pid, since this one has made none yet.

use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::walk::{remove_or_set_aside, walk};

/// Who is sweeping.
pub struct Who<'a> {
    pub pid: u32,
    pub uid: u32,
    pub is_running: &'a dyn Fn(u32) -> bool,
}

/// Remove each `xtask-run-<pid>` under `base` that is `who`'s and whose
/// run is not going; one line for each removed or left.
///
/// # Errors
/// `base` cannot be read, or a directory could not be removed (named,
/// set aside so the next run is not blocked, with the fix).
pub fn sweep(base: &Path, who: &Who<'_>) -> Result<Vec<String>, String> {
    let listed = std::fs::read_dir(base).map_err(|e| format!("{}: {e}", base.display()))?;
    let mut said = Vec::new();
    for entry in listed {
        let entry = entry.map_err(|e| format!("{}: {e}", base.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(pid) = name
            .strip_prefix("xtask-run-")
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != who.pid && (who.is_running)(pid) {
            continue;
        }
        said.extend(one(&entry.path(), pid, who.uid)?);
    }
    Ok(said)
}

/// Remove one dead run's directory, if it is this user's.
fn one(path: &Path, pid: u32, uid: u32) -> Result<Option<String>, String> {
    let owner = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if owner.uid() != uid {
        let other = owner.uid();
        return Ok(Some(format!(
            "left {} alone: it is user {other}'s, not this one's",
            path.display()
        )));
    }
    let count = walk(path).entries.len();
    remove_or_set_aside(path)?;
    Ok(Some(format!(
        "removed {} ({count} entries), left by run {pid}, which is not going",
        path.display()
    )))
}
