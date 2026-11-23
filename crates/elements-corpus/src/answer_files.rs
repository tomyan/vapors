//! Which oracle answers are committed against which are needed: for a
//! suffix such as `.ax.json`, the fixtures that need an answer and have
//! none, and the answers no fixture needs (an orphan, left when its
//! fixture or script was removed, or when its fixture moved to another
//! page). The oracles refuse orphans before they run; the corpus's
//! `committed_references` test refuses both without a browser.

use std::collections::BTreeSet;
use std::path::Path;

use crate::oracle::fixtures_dir;

/// The answers with one suffix, against the names that need one.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Unmatched {
    /// Names that need an answer and have none, sorted.
    pub missing: Vec<String>,
    /// Names with an answer that none needs, sorted.
    pub orphaned: Vec<String>,
}

/// `fixtures/<name><suffix>` for each of `wanted`, against the files there.
///
/// # Errors
/// The directory cannot be read.
pub fn unmatched(wanted: &[String], suffix: &str) -> Result<Unmatched, String> {
    unmatched_in(&fixtures_dir(), wanted, suffix)
}

fn unmatched_in(dir: &Path, wanted: &[String], suffix: &str) -> Result<Unmatched, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut found = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        if let Some(name) = entry.file_name().to_string_lossy().strip_suffix(suffix) {
            found.insert(name.to_owned());
        }
    }
    let wanted: BTreeSet<String> = wanted.iter().cloned().collect();
    Ok(Unmatched {
        missing: wanted.difference(&found).cloned().collect(),
        orphaned: found.difference(&wanted).cloned().collect(),
    })
}

/// Refuses the first answer with `suffix` that none of `wanted` needs.
///
/// # Errors
/// That answer, naming the file and the fix, or the directory cannot be read.
pub fn refuse_orphans(wanted: &[String], suffix: &str) -> Result<(), String> {
    match unmatched(wanted, suffix)?.orphaned.first() {
        None => Ok(()),
        Some(name) => Err(orphan(name, suffix)),
    }
}

/// An orphan's refusal, naming the file and the fix.
#[must_use]
pub fn orphan(name: &str, suffix: &str) -> String {
    format!(
        "{}: answers no fixture that needs it; fix: remove it, or restore its fixture or script",
        fixtures_dir().join(format!("{name}{suffix}")).display()
    )
}

#[cfg(test)]
#[path = "answer_files_tests.rs"]
mod tests;
