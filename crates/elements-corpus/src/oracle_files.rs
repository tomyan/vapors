//! What the oracles share about their committed files: a committed
//! answer compared with Chromium's (missing, malformed, different at its
//! first line, or the same), outputs with no fixture refused, and a
//! scratch directory removed however the run ends.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::oracle::fixtures_dir;

/// A committed answer, against Chromium's.
#[derive(Debug, PartialEq, Eq)]
pub enum Committed {
    /// The same text.
    Same,
    /// No file.
    Missing,
    /// Not JSON, with why.
    Malformed(String),
    /// The first line that differs, from 1.
    Different {
        /// The line.
        line: usize,
        /// The committed line.
        committed: String,
        /// Chromium's.
        chromium: String,
    },
}

impl Committed {
    /// A line for the log, naming the fixture and the fix.
    #[must_use]
    pub fn report(&self, name: &str) -> String {
        match self {
            Self::Same => format!("{name}: the same"),
            Self::Missing => {
                format!("{name}: no committed answer; rerun without --check and commit it")
            }
            Self::Malformed(why) => {
                format!("{name}: the committed answer is not JSON ({why}); rerun without --check")
            }
            Self::Different {
                line,
                committed,
                chromium,
            } => {
                format!(
                    "{name}: line {line}: committed `{committed}`, Chromium `{chromium}`; rerun without --check, review the diff, and commit it"
                )
            }
        }
    }
}

/// The file at `path` against Chromium's `text`.
///
/// # Errors
/// The file exists but cannot be read.
pub fn compare(path: &Path, text: &str) -> Result<Committed, String> {
    let committed = match std::fs::read_to_string(path) {
        Ok(committed) => committed,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Committed::Missing),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if let Err(e) = serde_json::from_str::<Value>(&committed) {
        return Ok(Committed::Malformed(e.to_string()));
    }
    Ok(
        first_difference(&committed, text).map_or(
            Committed::Same,
            |(line, committed, chromium)| Committed::Different {
                line,
                committed,
                chromium,
            },
        ),
    )
}

/// The first line, from 1, where `committed` and `chromium` differ, with
/// each side's line; a side that ends first reads `(end)`.
fn first_difference(committed: &str, chromium: &str) -> Option<(usize, String, String)> {
    let at = |t: &str, index: usize| t.lines().nth(index).unwrap_or("(end)").trim().to_owned();
    let longest = committed.lines().count().max(chromium.lines().count());
    (0..longest)
        .find(|&i| at(committed, i) != at(chromium, i))
        .map(|i| (i + 1, at(committed, i), at(chromium, i)))
}

/// Every `fixtures/*<suffix>` with no `.wisp` beside it, refused.
///
/// # Errors
/// The first such file, or the directory cannot be read.
pub fn refuse_orphans(suffix: &str) -> Result<(), String> {
    let dir = fixtures_dir();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.filter_map(Result::ok) {
        let file = entry.file_name().to_string_lossy().into_owned();
        if let Some(name) = file.strip_suffix(suffix)
            && !dir.join(format!("{name}.wisp")).is_file()
        {
            return Err(format!(
                "{}: no {name}.wisp beside it; fix: remove it or add the fixture",
                dir.join(&file).display()
            ));
        }
    }
    Ok(())
}

/// A scratch directory, removed when dropped, whatever the run did.
pub struct Work(PathBuf);

impl Work {
    /// `<temp>/<name>-<pid>`, made empty.
    ///
    /// # Errors
    /// It cannot be made.
    pub fn new(name: &str) -> Result<Self, String> {
        let dir = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Self(dir))
    }

    /// Its path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
#[path = "oracle_files_tests.rs"]
mod tests;
