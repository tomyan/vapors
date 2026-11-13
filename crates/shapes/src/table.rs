//! The shape table (docs/plans/one-scene.md 1.3): one `static` per
//! crate, emitted by the compiler beside its `wisp.rs` (S5a), holding
//! every literal shape of the crate's templates once. An entry is its
//! geometry in viewBox units, its canonical bytes and their hash; the
//! process registry (S5b) merges the tables by hash and issues the
//! [`ShapeId`]s items draw by. The compiler emits the tables and the
//! registry checks each at the merge; nothing is drawn from a table
//! that fails.

use std::collections::HashMap;

use crate::geometry::{Geometry, check_geometry};
pub use crate::shape_id::{ShapeId, ShapeIndex};
use crate::table_error::EMITTER;
pub use crate::table_error::TableError;

/// The format of a table's canonical bytes and geometry. A table built
/// by a compiler with another format is refused at the merge: equal
/// canonical bytes must mean equal geometry across crates (D3), and a
/// changed flattening tolerance or tessellator changes this number.
pub const TABLE_FORMAT: u32 = 1;

/// One crate's shapes, emitted by the compiler as a `static`.
#[derive(Debug)]
pub struct ShapeTable {
    crate_name: &'static str,
    format: u32,
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
    /// the registry's merge. Unverified until S2 brings the hash
    /// function, when [`ShapeTable::check`] recomputes it.
    pub hash: u64,
    /// What is drawn.
    pub geometry: Geometry,
}

impl ShapeTable {
    /// The table of `crate_name` holding `entries`, in `format`
    /// ([`TABLE_FORMAT`] of the compiler that emitted it).
    #[must_use]
    pub const fn new(
        crate_name: &'static str,
        format: u32,
        entries: &'static [ShapeEntry],
    ) -> Self {
        Self {
            crate_name,
            format,
            entries,
        }
    }

    /// The crate whose shapes these are.
    #[must_use]
    pub const fn crate_name(&self) -> &'static str {
        self.crate_name
    }

    /// Every entry, in the compiler's order.
    #[must_use]
    pub const fn entries(&self) -> &'static [ShapeEntry] {
        self.entries
    }

    /// The entry an element's code named.
    ///
    /// # Errors
    /// [`TableError`] when `index` is past the end, naming the crate.
    pub fn entry(&self, index: ShapeIndex) -> Result<&'static ShapeEntry, TableError> {
        self.entries
            .get(index.get() as usize)
            .ok_or_else(|| TableError {
                crate_name: self.crate_name,
                site: None,
                reason: format!(
                    "entry {} was named but the table holds {}",
                    index.get(),
                    self.entries.len()
                ),
                fix: EMITTER,
            })
    }

    /// Whether the table is in this crate's format, every entry can be
    /// drawn and names its site and bytes, and the table holds each
    /// shape once: the first fault is refused, naming it. One pass
    /// over the entries (1.3: the merge is O(entries)).
    ///
    /// # Errors
    /// [`TableError`] for another [`TABLE_FORMAT`], an empty crate
    /// name, site or canonical bytes, a geometry that cannot be drawn
    /// ([`check_geometry`]), or two entries with one hash: the same
    /// bytes (the compiler deduplicates, so a repeat is its bug) or
    /// different bytes (a collision the registry would also refuse).
    pub fn check(&self) -> Result<(), TableError> {
        self.check_format()?;
        let mut seen: HashMap<u64, usize> = HashMap::with_capacity(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            check_entry(entry).map_err(|reason| self.refuse(entry, reason, EMITTER))?;
            if let Some(earlier) = seen.insert(entry.hash, index) {
                return Err(self.hash_shared(&self.entries[earlier], entry));
            }
        }
        Ok(())
    }

    fn check_format(&self) -> Result<(), TableError> {
        if self.crate_name.is_empty() {
            return Err(self.refuse_table("the crate name is empty".to_owned(), EMITTER));
        }
        if self.format != TABLE_FORMAT {
            return Err(self.refuse_table(
                format!(
                    "is in table format {} and this vapors-shapes reads format {TABLE_FORMAT}",
                    self.format
                ),
                "build every crate with one vapors-compiler, the one this runtime matches",
            ));
        }
        Ok(())
    }

    /// Why `entry`, whose hash `earlier` already has, is refused.
    fn hash_shared(&self, earlier: &ShapeEntry, entry: &ShapeEntry) -> TableError {
        if earlier.canonical == entry.canonical {
            let reason = format!(
                "is the same shape as the entry at {}; one crate's table holds each shape once",
                earlier.site
            );
            return self.refuse(entry, reason, EMITTER);
        }
        let reason = format!(
            "has the hash {:#x} of the entry at {} with different canonical bytes; a hash names one shape",
            entry.hash, earlier.site
        );
        self.refuse(
            entry,
            reason,
            "change one shape's canonical bytes (another stroke width, or a point added to the path), or widen the hash",
        )
    }

    fn refuse(&self, entry: &ShapeEntry, reason: String, fix: &'static str) -> TableError {
        TableError {
            crate_name: self.crate_name,
            site: Some(entry.site),
            reason,
            fix,
        }
    }

    fn refuse_table(&self, reason: String, fix: &'static str) -> TableError {
        TableError {
            crate_name: self.crate_name,
            site: None,
            reason,
            fix,
        }
    }
}

/// Why `entry` is wrong on its own, if it is.
fn check_entry(entry: &ShapeEntry) -> Result<(), String> {
    if entry.site.is_empty() {
        return Err("the site is empty; an entry names where its shape was written".to_owned());
    }
    if entry.canonical.is_empty() {
        return Err(
            "the canonical bytes are empty; an entry carries its shape's canonical form".to_owned(),
        );
    }
    check_geometry(&entry.geometry)
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
