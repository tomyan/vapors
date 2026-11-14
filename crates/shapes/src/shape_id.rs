//! The two integers that name a shape: its place in its own crate's
//! table, and the id the process registry issues once the tables are
//! merged. Each is its own type, so one is never read as the other.

/// A shape the process registry has taken from a crate's table: the
/// index into the registry's merged entries. Issued by the registry
/// alone (in this crate, S5b), once per distinct shape in the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId(u32);

impl ShapeId {
    /// The id of the registry's entry `index`.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the registry (S5b) issues ids; retire this when it does"
        )
    )]
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The index into the registry's merged entries.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// An entry's place in its own crate's table, which the crate's
/// emitted code names; never an index into the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShapeIndex(u32);

impl ShapeIndex {
    /// The entry at `index` of the crate's table.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The place in the table.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
