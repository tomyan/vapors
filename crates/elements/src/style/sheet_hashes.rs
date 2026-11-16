//! The hash of each registered sheet's text, as compiled winners name
//! the sheet (`vapors_dom::SheetRef`) and the compiler hashed it
//! (`vapors_css::sheet_hash`).

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::SheetRef;

use super::Sheets;

impl Sheets {
    /// The hash of the text registered as `sheet`; `None` when no sheet
    /// is registered so: a scope whose component has not mounted, or a
    /// global sheet, which is registered by name, not by file, until
    /// the compiler registers global sheets itself (build-time-styles.md,
    /// R1a).
    pub(crate) fn hash_of(&self, sheet: SheetRef) -> Option<u64> {
        match sheet {
            SheetRef::UserAgent => Some(sheet_hash(vapors_css::user_agent::TEXT)),
            SheetRef::Scope(scope) => self.scopes.get(scope).map(|text| sheet_hash(text)),
            SheetRef::Global(_) => None,
        }
    }
}
