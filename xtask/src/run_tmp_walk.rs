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

/// Remove `dir`; when it cannot be, rename it to `<dir>-unremovable` so
/// no later sweep trips on it, and say how to remove it by hand.
///
/// # Errors
/// It could not be removed: the error, where it now is, and the fix.
pub fn remove_or_set_aside(dir: &Path) -> Result<(), String> {
    let error = match std::fs::remove_dir_all(dir) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => e,
    };
    let aside = PathBuf::from(format!("{}-unremovable", dir.display()));
    let now = match std::fs::rename(dir, &aside) {
        Ok(()) => format!("set aside as {}", aside.display()),
        Err(e) => format!("not set aside either ({e})"),
    };
    let shown = if aside.exists() { &aside } else { dir };
    Err(format!(
        "{shown} could not be removed ({error}), {now}; \
         fix: chmod -R u+rwx {shown} && rm -rf {shown} \
         (sudo rm -rf {shown} if a file in it is not yours)",
        shown = shown.display(),
    ))
}
