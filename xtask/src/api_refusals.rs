//! What the harness server refused, kept for the run's summary and
//! printed as it happens (review of #473-#475, M3). A session chooses
//! these bytes, so the log is bounded (the first [`KEPT`] entries, each at
//! most [`ENTRY_CHARS`] characters, and a count of all) and every entry
//! is escaped before it reaches a terminal, so no control sequence a
//! session sends is printed raw.

use std::sync::Mutex;

/// The refusals kept for the summary; the rest are counted.
pub const KEPT: usize = 50;

/// The most characters of one refusal kept or printed, escaped.
pub const ENTRY_CHARS: usize = 200;

/// The refusals of one run.
#[derive(Default)]
pub struct Refusals {
    log: Mutex<(Vec<String>, u64)>,
}

impl Refusals {
    /// `why` printed, escaped and cut, and kept while fewer than [`KEPT`]
    /// are; counted always.
    /// The first [`KEPT`] are printed too; after them, one line every
    /// [`KEPT`] more, so a loop of refusals cannot flood the terminal
    /// (review of #476-#478, L5).
    pub fn push(&self, why: &str) {
        let entry = escaped(why);
        let Ok(mut log) = self.log.lock() else {
            eprintln!(
                "api-build: refused: {entry} (the refusal log's lock was poisoned; this refusal is only here)"
            );
            return;
        };
        log.1 += 1;
        if log.0.len() < KEPT {
            eprintln!("api-build: refused: {entry}");
            log.0.push(entry);
        } else if log.1 % KEPT as u64 == 0 {
            eprintln!("api-build: {} requests refused so far", log.1);
        }
    }

    /// The refusals kept, and how many there were in all.
    pub fn kept(&self) -> (Vec<String>, u64) {
        self.log
            .lock()
            .map_or_else(|_| (Vec::new(), 0), |log| log.clone())
    }
}

/// `text` with every control character escaped, cut to [`ENTRY_CHARS`].
pub fn escaped(text: &str) -> String {
    let escaped: String = text.escape_debug().collect();
    let mut cut: String = escaped.chars().take(ENTRY_CHARS).collect();
    if escaped.chars().count() > ENTRY_CHARS {
        cut.push_str("...");
    }
    cut
}

#[cfg(test)]
#[path = "api_refusals_tests.rs"]
mod tests;
