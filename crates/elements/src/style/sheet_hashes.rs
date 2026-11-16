//! The hash of each sheet in the cascade, as compiled winners name the
//! sheet (`vapors_dom::SheetRef`) and the compiler hashed it
//! (`vapors_css::sheet_hash`), taken once when the sheet joins the
//! cascade.

use std::sync::OnceLock;

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::SheetRef;

use super::Sheets;

/// What the cascade holds under a sheet's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetHash {
    /// A sheet whose text hashes so.
    Registered(u64),
    /// None: its component has not mounted, or its sheet was refused.
    Absent,
    /// The cascade cannot tell, for the reason given.
    Unresolvable(&'static str),
}

/// Why a global sheet cannot be told apart: until R1a, a
/// `Styles::Global` component's sheet is added by name with
/// `add_stylesheet`, not under its file.
const GLOBAL_UNTIL_R1A: &str = "a global component's sheet is added from Rust under a name, not \
     under its file, until the compiler registers global sheets itself (build-time-styles.md, R1a)";

impl Sheets {
    /// Notes `text`, now in the cascade as `name`.
    pub(crate) fn note_hash(&mut self, name: &str, text: &str) {
        self.hashes.insert(name.to_owned(), sheet_hash(text));
    }

    /// The hash of the sheet in the cascade as `sheet`.
    pub(crate) fn hash_of(&self, sheet: SheetRef) -> SheetHash {
        static USER_AGENT: OnceLock<u64> = OnceLock::new();
        match sheet {
            SheetRef::UserAgent => SheetHash::Registered(
                *USER_AGENT.get_or_init(|| sheet_hash(vapors_css::user_agent::TEXT)),
            ),
            SheetRef::Scope(scope) => self
                .hashes
                .get(scope)
                .map_or(SheetHash::Absent, |&hash| SheetHash::Registered(hash)),
            SheetRef::Global(_) => SheetHash::Unresolvable(GLOBAL_UNTIL_R1A),
        }
    }
}
