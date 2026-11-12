//! Reading and removing a run directory without letting one bad entry
//! block later runs: what can be read is counted, what cannot is named,
//! and a directory that cannot be removed is set aside under a name the
//! sweep no longer matches, so it fails the run that found it, once.

use std::path::{Path, PathBuf};

/// What a walk of a directory found: entries as paths relative to it,
/// sorted, and what could not be read, each naming the path and error.
#[derive(Debug, Default)]
pub struct Walked {
    pub entries: Vec<String>,
    pub unread: Vec<String>,
}

/// Every entry under `dir` that can be read.
#[must_use]
pub fn walk(dir: &Path) -> Walked {
    let mut walked = Walked::default();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let listed = match std::fs::read_dir(&next) {
            Ok(listed) => listed,
            Err(e) => {
                walked.unread.push(format!("{}: {e}", next.display()));
                continue;
            }
        };
        for entry in listed {
            match entry {
                Ok(entry) => note(dir, &entry, &mut walked.entries, &mut pending),
                Err(e) => walked.unread.push(format!("{}: {e}", next.display())),
            }
        }
    }
    walked.entries.sort();
    walked
}

fn note(
    dir: &Path,
    entry: &std::fs::DirEntry,
    found: &mut Vec<String>,
    pending: &mut Vec<PathBuf>,
) {
    let path = entry.path();
    if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
        pending.push(path.clone());
    }
    found.push(
        path.strip_prefix(dir)
            .unwrap_or(&path)
            .display()
            .to_string(),
    );
}

/// The marker in a set-aside directory's name; the sweep counts them.
pub const SET_ASIDE: &str = "-unremovable-";

/// How many names a set-aside tries before giving up.
const SET_ASIDE_TRIES: u32 = 100;

/// Remove `dir`; when it cannot be, rename it to the first free
/// `<dir>-unremovable-<n>` (never onto one already there, which may hold
/// another run's leftovers) so no later sweep trips on it, and say how
/// to remove it by hand.
///
/// # Errors
/// It could not be removed: the error, where it now is, and the fix.
pub fn remove_or_set_aside(dir: &Path) -> Result<(), String> {
    let error = match std::fs::remove_dir_all(dir) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => e,
    };
    let (shown, now) = match set_aside(dir) {
        Ok(aside) => (aside.clone(), format!("set aside as {}", aside.display())),
        Err(why) => (dir.to_path_buf(), format!("not set aside either ({why})")),
    };
    Err(format!(
        "{shown} could not be removed ({error}), {now}; \
         fix: chmod -R u+rwx {shown} && rm -rf {shown} \
         (sudo rm -rf {shown} if a file in it is not yours)",
        shown = shown.display(),
    ))
}

/// Rename `dir` to the first `<dir>-unremovable-<n>` that is not there.
fn set_aside(dir: &Path) -> Result<PathBuf, String> {
    for n in 1..=SET_ASIDE_TRIES {
        let aside = PathBuf::from(format!("{}{SET_ASIDE}{n}", dir.display()));
        if aside.symlink_metadata().is_ok() {
            continue;
        }
        return std::fs::rename(dir, &aside)
            .map(|()| aside)
            .map_err(|e| e.to_string());
    }
    Err(format!("{SET_ASIDE_TRIES} set-aside names are taken"))
}
