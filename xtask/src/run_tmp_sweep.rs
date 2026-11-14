//! Removing the directories of runs that are not going: a run killed or
//! panicking cannot remove its own. Only this user's directories are
//! touched; this run's own pid names a directory left by an earlier run
//! with that pid, since this one has made none yet.

use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::walk::{SET_ASIDE, remove_or_set_aside, walk};

/// The most set-aside directories (runs' leftovers that could not be
/// removed) the sweep lets runs go on with. One is a single bad run, said
/// on every run until someone removes it; more than this many means
/// something keeps leaving files the factory cannot remove, which needs a
/// person to look before it fills /tmp, so runs stop until then.
const SET_ASIDE_LIMIT: usize = 3;

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
    let mut aside = Vec::new();
    for entry in listed {
        let entry = entry.map_err(|e| format!("{}: {e}", base.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("xtask-run-")
            && name.contains(SET_ASIDE)
            && mine(&entry.path(), who.uid)
        {
            aside.push(entry.path().display().to_string());
            continue;
        }
        let Some(pid) = name
            .strip_prefix("xtask-run-")
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != who.pid && (who.is_running)(pid) {
            continue;
        }
        said.extend(one(&entry.path(), pid, who)?);
    }
    said.extend(set_aside(aside)?);
    Ok(said)
}

/// A line naming the set-aside directories, or a refusal when there are
/// more than `SET_ASIDE_LIMIT`.
fn set_aside(mut aside: Vec<String>) -> Result<Option<String>, String> {
    if aside.is_empty() {
        return Ok(None);
    }
    aside.sort();
    let listed = format!(
        "{} set-aside run directories: {}",
        aside.len(),
        aside.join(", ")
    );
    let fix = "fix: chmod -R u+rwx <dir> && rm -rf <dir> for each, after looking at what is in it";
    if aside.len() > SET_ASIDE_LIMIT {
        return Err(format!(
            "{listed}; more than {SET_ASIDE_LIMIT}, so runs stop; {fix}"
        ));
    }
    Ok(Some(format!("{listed}; {fix}")))
}

fn mine(path: &Path, uid: u32) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.uid() == uid)
}

/// Remove one dead run's directory, if it is this user's and no run
/// started inside it is still going (that run's TMPDIR is in it).
fn one(path: &Path, pid: u32, who: &Who<'_>) -> Result<Option<String>, String> {
    let owner = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if owner.uid() != who.uid {
        let other = owner.uid();
        return Ok(Some(format!(
            "left {} alone: it is user {other}'s, not this one's",
            path.display()
        )));
    }
    let entries = walk(path).entries;
    if let Some(live) = live_inside(&entries, who.is_running) {
        return Ok(Some(format!(
            "left {} alone: run {live} inside it is still going",
            path.display()
        )));
    }
    let count = entries.len();
    remove_or_set_aside(path)?;
    Ok(Some(format!(
        "removed {} ({count} entries), left by run {pid}, which is not going",
        path.display()
    )))
}

/// The pid of a run still going whose `xtask-run-<pid>` is among
/// `entries` (paths under a dead run's directory).
fn live_inside(entries: &[String], is_running: &dyn Fn(u32) -> bool) -> Option<u32> {
    entries
        .iter()
        .filter_map(|entry| Path::new(entry).file_name())
        .filter_map(|name| name.to_str()?.strip_prefix("xtask-run-")?.parse().ok())
        .find(|&pid| is_running(pid))
}
