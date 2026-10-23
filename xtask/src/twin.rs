//! `cargo xtask check-xtask-twin <sibling>`: this repository's xtask
//! against the sibling's, file by file (target/ aside), failing and naming
//! each file that differs or exists on one side only. touche and vapors
//! keep one xtask; touche's CI runs this against ../vapors at its pin, so
//! an xtask change lands in vapors first and touche's pin follows.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Compare `./xtask` with `<sibling>/xtask` and report.
pub fn run(sibling: &Path) -> ExitCode {
    let theirs = sibling.join("xtask");
    match differences(Path::new("xtask"), &theirs) {
        Ok(found) if found.is_empty() => {
            println!("ok   twin     xtask is the same as {}", theirs.display());
            ExitCode::SUCCESS
        }
        Ok(found) => {
            println!("FAIL twin: xtask differs from {}:", theirs.display());
            for line in found {
                println!("     {line}");
            }
            println!(
                "     fix: make the same change in both (vapors first), then bump touche's .config/vapors-rev"
            );
            ExitCode::FAILURE
        }
        Err(why) => {
            println!("FAIL twin: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Each file that differs, or is on one side only, relative to the roots.
///
/// # Errors
///
/// A root that cannot be read, naming it.
pub fn differences(ours: &Path, theirs: &Path) -> Result<Vec<String>, String> {
    let (here, there) = (files(ours)?, files(theirs)?);
    let mut found = Vec::new();
    for path in here.union(&there) {
        let line = match (here.contains(path), there.contains(path)) {
            (true, false) => format!("only here: {}", path.display()),
            (false, true) => format!("only in the sibling: {}", path.display()),
            _ if read(&ours.join(path))? != read(&theirs.join(path))? => {
                format!("differs: {}", path.display())
            }
            _ => continue,
        };
        found.push(line);
    }
    Ok(found)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Every file under `root`, relative to it, skipping `target`.
fn files(root: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let mut found = BTreeSet::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(root.join(&dir))
            .map_err(|e| format!("{}: {e}", root.join(&dir).display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", root.display()))?;
            let path = dir.join(entry.file_name());
            if path == Path::new("target") {
                continue;
            }
            if entry.path().is_dir() {
                pending.push(path);
            } else {
                found.insert(path);
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
#[path = "twin_tests.rs"]
mod tests;
