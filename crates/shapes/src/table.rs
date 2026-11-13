//! The shape table (docs/plans/one-scene.md 1.3): one `static` per
//! crate, emitted by the compiler beside its `wisp.rs` (S5a), holding
//! every literal shape of the crate's templates once. An entry is its
//! geometry in viewBox units, its canonical bytes and their hash; the
//! process registry (S5b) merges the tables by hash and issues the
//! [`ShapeId`]s items draw by. The compiler emits the tables and the
//! registry checks each at the merge; nothing is drawn from a table
//! that fails.

use std::fmt;

use crate::geometry::{Geometry, check_geometry};

/// A shape the process registry has taken from a crate's table: the
/// index into the registry's merged entries. Issued by the registry
/// alone, once per distinct shape in the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId(u32);

impl ShapeId {
    /// The id of the registry's entry `index`.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The index into the registry's merged entries.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// One crate's shapes, emitted by the compiler as a `static`.
#[derive(Debug)]
pub struct ShapeTable {
    crate_name: &'static str,
    entries: &'static [ShapeEntry],
}

/// One distinct shape of a crate's templates.
#[derive(Debug, PartialEq)]
pub struct ShapeEntry {
    /// Where the shape was first written, `file:line`, for messages.
    pub site: &'static str,
    /// The canonical form (one-scene.md 1.3): absolute commands in
    /// viewBox units, the fill rule and the stroke parameters that
    /// change the mesh. Two entries with equal bytes are one shape.
    pub canonical: &'static [u8],
    /// The hash of `canonical`; equal hashes are compared by bytes at
    /// the registry's merge.
    pub hash: u64,
    /// What is drawn.
    pub geometry: Geometry,
}

/// Why a table was refused: which crate's, which entry, and what is
/// wrong. The table is the compiler's output, so the fix is there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableError {
    /// The crate whose table it is.
    pub crate_name: &'static str,
    /// The refused entry's site, or `None` for a lookup past the end.
    pub site: Option<&'static str>,
    /// What is wrong.
    pub reason: String,
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shape table of crate {}", self.crate_name)?;
        if let Some(site) = self.site {
            write!(f, ", entry at {site}")?;
        }
        write!(
            f,
            ": {}; the table is emitted by vapors-compiler from the crate's templates, so fix the emitter",
            self.reason
        )
    }
}

impl std::error::Error for TableError {}

impl ShapeTable {
    /// The table of `crate_name` holding `entries`.
    #[must_use]
    pub const fn new(crate_name: &'static str, entries: &'static [ShapeEntry]) -> Self {
        Self {
            crate_name,
            entries,
        }
    }

    /// The crate whose shapes these are.
    #[must_use]
    pub const fn crate_name(&self) -> &'static str {
        self.crate_name
    }

    /// Every entry, in the compiler's order; an element's emitted code
    /// names its shape by index here.
    #[must_use]
    pub const fn entries(&self) -> &'static [ShapeEntry] {
        self.entries
    }

    /// The entry an element's code named.
    ///
    /// # Errors
    /// [`TableError`] when `index` is past the end, naming the crate.
    pub fn entry(&self, index: usize) -> Result<&'static ShapeEntry, TableError> {
        self.entries.get(index).ok_or_else(|| TableError {
            crate_name: self.crate_name,
            site: None,
            reason: format!(
                "entry {index} was named but the table holds {}",
                self.entries.len()
            ),
        })
    }

    /// Whether every entry can be drawn and the table holds each shape
    /// once: the first entry that cannot is refused, naming it.
    ///
    /// # Errors
    /// [`TableError`] for a geometry that cannot be drawn
    /// ([`check_geometry`]), or for two entries with one hash: the same
    /// bytes (the compiler deduplicates, so a repeat is its bug) or
    /// different bytes (a collision the registry would also refuse).
    pub fn check(&self) -> Result<(), TableError> {
        for (index, entry) in self.entries.iter().enumerate() {
            check_geometry(&entry.geometry).map_err(|reason| self.refuse(entry, reason))?;
            if let Some(earlier) = self.entries[..index].iter().find(|e| e.hash == entry.hash) {
                return Err(self.refuse(entry, hash_shared(earlier, entry)));
            }
        }
        Ok(())
    }

    fn refuse(&self, entry: &ShapeEntry, reason: String) -> TableError {
        TableError {
            crate_name: self.crate_name,
            site: Some(entry.site),
            reason,
        }
    }
}

/// Why `entry`, whose hash `earlier` already has, is refused.
fn hash_shared(earlier: &ShapeEntry, entry: &ShapeEntry) -> String {
    if earlier.canonical == entry.canonical {
        return format!(
            "is the same shape as the entry at {}; one crate's table holds each shape once",
            earlier.site
        );
    }
    format!(
        "has the hash {:#x} of the entry at {} with different canonical bytes; a hash names one shape",
        entry.hash, earlier.site
    )
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
