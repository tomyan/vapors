//! Check 1 of compiled winners (`docs/plans/build-time-styles.md`,
//! section 2): each sheet a key's winners index holds the text the
//! compiler read, so its rule and declaration indices name the rules
//! the compiler judged. The user-agent sheet and the key's own sheet
//! are in the cascade before its elements exist, so their absence
//! means the compiler and the runtime name a sheet differently.

use vapors_dom::{SheetRef, StyleKey};

use crate::{Document, SheetHash};

impl Document {
    /// What the cascade holds as `sheet`: its text's hash, or nothing.
    #[must_use]
    pub fn sheet_hash(&self, sheet: SheetRef) -> SheetHash {
        self.sheets.hash_of(sheet)
    }

    /// How many compiled sheets' texts were hashed to check their claim
    /// (`Sheets::compiled_text`): once per sheet, text and claim, not per
    /// mount, which a test holds.
    #[doc(hidden)]
    #[must_use]
    pub fn sheet_hashes_taken(&self) -> usize {
        self.sheets.hashes_taken
    }

    /// Whether every sheet `key`'s winners index is the one the compiler
    /// read. A sheet absent from the cascade is not a mismatch (its rules
    /// apply nowhere), unless it is the user-agent sheet or the key's
    /// own; a key without winners has nothing to check.
    ///
    /// # Errors
    /// The first sheet that hashes otherwise, or is absent though it is
    /// the key's own, naming the key's site, the sheet and the fix.
    pub fn check_compiled_sheets(&self, key: &StyleKey) -> Result<(), String> {
        let Some(winners) = key.winners.compiled() else {
            return Ok(());
        };
        let own = |index: usize| {
            index == 0 && winners.sheets[0].0 == SheetRef::UserAgent
                || winners.own.is_some_and(|own| usize::from(own) == index)
        };
        let mut sheets = winners.sheets.iter().enumerate();
        let refusal = sheets
            .find_map(|(index, &(sheet, compiled))| self.refusal(sheet, compiled, own(index)));
        refusal.map_or(Ok(()), |why| Err(format!("{}: {why}", key.site)))
    }

    fn refusal(&self, sheet: SheetRef, compiled: u64, own: bool) -> Option<String> {
        match self.sheet_hash(sheet) {
            SheetHash::Registered(hash) if hash == compiled => None,
            SheetHash::Registered(hash) => Some(format!(
                "its compiled winners read {sheet:?} with hash {}, but the sheet in the cascade \
                 under that name hashes {}, so their rule indices name other rules; fix: rebuild \
                 the crate that registered the sheet, with the same vapors-css; reproduce: \
                 `cargo clean -p <that crate> && cargo build`, then mount the component",
                hex(compiled),
                hex(hash)
            )),
            SheetHash::Absent if own => Some(format!(
                "its compiled winners name its own sheet {sheet:?}, which the cascade does not \
                 hold, though a component adds its sheet before its elements: the compiler and \
                 the runtime name the sheet differently (codegen's SCOPE or SHEET and the key's SheetRef); \
                 fix: give both one name; reproduce: cargo nextest run -p vapors-perf \
                 every_compiled_row"
            )),
            SheetHash::Absent => None,
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
