//! Check 1 of compiled winners (`docs/plans/build-time-styles.md`,
//! section 2): each sheet a key's winners index holds the text the
//! compiler read, so its rule and declaration indices name the rules
//! the compiler judged. The user-agent sheet and the key's own sheet
//! are in the cascade before its elements exist, so their absence
//! means the compiler and the runtime name a sheet differently.

use vapors_dom::{SheetRef, StyleKey};

use crate::{Document, SheetHash};

impl Document {
    /// What the cascade holds as `sheet`: its text's hash, nothing, or
    /// a reason it cannot tell.
    #[must_use]
    pub fn sheet_hash(&self, sheet: SheetRef) -> SheetHash {
        self.sheets.hash_of(sheet)
    }

    /// Whether every sheet `key`'s winners index is the one the compiler
    /// read. A sheet absent from the cascade is not a mismatch (its rules
    /// apply nowhere), unless it is the user-agent sheet or the key's
    /// own; a key without winners has nothing to check.
    ///
    /// # Errors
    /// The first sheet that hashes otherwise, is absent though it is the
    /// key's own, or cannot be told apart, naming the key's site, the
    /// sheet and the fix.
    pub fn check_compiled_sheets(&self, key: &StyleKey) -> Result<(), String> {
        let Some(winners) = key.winners.compiled() else {
            return Ok(());
        };
        let refusal = winners
            .sheets
            .iter()
            .find_map(|&(sheet, compiled)| self.refusal(key, sheet, compiled));
        refusal.map_or(Ok(()), |why| Err(format!("{}: {why}", key.site)))
    }

    fn refusal(&self, key: &StyleKey, sheet: SheetRef, compiled: u64) -> Option<String> {
        let own = sheet == SheetRef::UserAgent
            || matches!(sheet, SheetRef::Scope(s) if Some(s) == key.scope);
        match self.sheet_hash(sheet) {
            SheetHash::Registered(hash) if hash == compiled => None,
            SheetHash::Registered(hash) => Some(format!(
                "its compiled winners read {sheet:?} with hash {}, but the sheet in the cascade \
                 under that name hashes {}, so their rule indices name other rules; fix: rebuild \
                 the crate that registered the sheet, with the same vapors-css; reproduce: mount \
                 the component after a clean build",
                hex(compiled),
                hex(hash)
            )),
            SheetHash::Absent if own => Some(format!(
                "its compiled winners name its own sheet {sheet:?}, which the cascade does not \
                 hold, though a component adds its sheet before its elements: the compiler and \
                 the runtime name the sheet differently (codegen's SCOPE and the key's SheetRef); \
                 fix: give both one name; reproduce: cargo nextest run -p vapors-perf \
                 every_compiled_row"
            )),
            SheetHash::Absent => None,
            SheetHash::Unresolvable(why) => Some(format!(
                "its compiled winners name {sheet:?}, which cannot be checked: {why}; fix: build \
                 after R1a, or build the component with scoped styles"
            )),
        }
    }
}

/// `hash` as the generated code writes it (`codegen::key_winners`).
fn hex(hash: u64) -> String {
    let hex = format!("{hash:016x}");
    format!(
        "0x{}_{}_{}_{}",
        &hex[..4],
        &hex[4..8],
        &hex[8..12],
        &hex[12..]
    )
}
