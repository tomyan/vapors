//! Check 1 of compiled winners (`docs/plans/build-time-styles.md`,
//! section 2): each sheet a key's winners index, when registered, holds
//! the text the compiler read, so the rule and declaration indices name
//! the rules the compiler judged.

use vapors_dom::{SheetRef, StyleKey};

use crate::Document;

impl Document {
    /// The hash of the text registered as `sheet`, as
    /// `vapors_css::sheet_hash::sheet_hash` makes it; `None` when no sheet
    /// is registered so.
    #[must_use]
    pub fn sheet_hash(&self, sheet: SheetRef) -> Option<u64> {
        self.sheets.hash_of(sheet)
    }

    /// Whether every registered sheet `key`'s winners index has the hash
    /// the compiler gave it. A sheet not registered is not a mismatch:
    /// its rules are in no cascade, and a key without winners has
    /// nothing to check.
    ///
    /// # Errors
    /// The first sheet whose hashes differ, naming the key's site, the
    /// sheet, both hashes and the fix.
    pub fn check_compiled_sheets(&self, key: &StyleKey) -> Result<(), String> {
        let Some(winners) = key.winners else {
            return Ok(());
        };
        let mismatch = winners.sheets.iter().find_map(|&(sheet, compiled)| {
            let registered = self.sheet_hash(sheet)?;
            (registered != compiled).then_some((sheet, compiled, registered))
        });
        let Some((sheet, compiled, registered)) = mismatch else {
            return Ok(());
        };
        Err(format!(
            "{}: its compiled winners read {sheet:?} with hash {compiled}, but the sheet \
             registered under that name hashes {registered}, so their rule indices name other \
             rules; fix: rebuild the crate that registered the sheet, with the same vapors-css",
            key.site
        ))
    }
}
