//! Why a shape table was refused: which crate's, which entry, what is
//! wrong and what helps.

use std::fmt;

/// Why a table was refused: which crate's, which entry, what is wrong
/// and what helps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableError {
    /// The crate whose table it is.
    pub crate_name: &'static str,
    /// The refused entry's site, or `None` for the table as a whole.
    pub site: Option<&'static str>,
    /// What is wrong. A cross-crate refusal (S5b) names the other
    /// crate and site here.
    pub reason: String,
    /// What helps.
    pub fix: &'static str,
}

/// The fix for a table that is wrong as emitted.
pub(crate) const EMITTER: &str =
    "the table is emitted by vapors-compiler from the crate's templates, so fix the emitter";

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shape table of crate {}", self.crate_name)?;
        if let Some(site) = self.site {
            write!(f, ", entry at {site}")?;
        }
        write!(f, ": {}; {}", self.reason, self.fix)
    }
}

impl std::error::Error for TableError {}
