//! A component's `<style>` as the compiler emitted it: the only thing
//! `Dom::add_style` and `Dom::add_global_style` take, so a sheet reaches
//! a tree of analysed elements only from generated code
//! (`docs/plans/build-time-styles.md`, decision 2 (a), R1b). Its
//! constructor is hidden and named `__compiled`; `cargo xtask lint`
//! refuses that name outside the code that emits and defines it
//! (`[imports.__compiled]` in `.config/factory.toml`), so hand-written
//! code that names it shows in review and in the lint.

/// A compiled sheet: its name (scope or global sheet), its text, and the
/// hash the compiler took of the text (`vapors_css::sheet_hash`), which
/// the tree checks before registering it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledSheet<'a> {
    name: &'a str,
    text: &'a str,
    hash: Option<u64>,
}

impl<'a> CompiledSheet<'a> {
    /// The sheet as generated code passes it. Not for hand-written code
    /// (see the module doc).
    #[doc(hidden)]
    #[must_use]
    pub const fn __compiled(name: &'a str, text: &'a str, hash: u64) -> Self {
        Self {
            name,
            text,
            hash: Some(hash),
        }
    }

    /// A sheet for a test of the tree, with no compiled hash to check.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn for_tests(name: &'a str, text: &'a str) -> Self {
        Self {
            name,
            text,
            hash: None,
        }
    }

    /// A sheet for a test of the hash check, claiming `hash`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn for_tests_hashed(name: &'a str, text: &'a str, hash: u64) -> Self {
        Self {
            name,
            text,
            hash: Some(hash),
        }
    }

    /// Its scope or global sheet name.
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Its text.
    #[must_use]
    pub const fn text(&self) -> &'a str {
        self.text
    }

    /// The hash the compiler took of its text; `None` from a test.
    #[must_use]
    pub const fn hash(&self) -> Option<u64> {
        self.hash
    }
}
