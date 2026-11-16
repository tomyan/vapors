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
}

/// Which kind of component sheet a name is held under: a scope and a
/// global sheet may share a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SheetKind {
    Scope,
    Global,
}

impl Sheets {
    /// Notes `text`, now in the cascade as the `kind` sheet `name`.
    pub(crate) fn note_hash(&mut self, kind: SheetKind, name: &str, text: &str) {
        self.hashes
            .insert((kind, name.to_owned()), sheet_hash(text));
    }

    /// The hash of the sheet in the cascade as `sheet`.
    pub(crate) fn hash_of(&self, sheet: SheetRef) -> SheetHash {
        static USER_AGENT: OnceLock<u64> = OnceLock::new();
        let held = |kind, name: &str| {
            let hash = self.hashes.get(&(kind, name.to_owned()));
            hash.map_or(SheetHash::Absent, |&hash| SheetHash::Registered(hash))
        };
        match sheet {
            SheetRef::UserAgent => SheetHash::Registered(
                *USER_AGENT.get_or_init(|| sheet_hash(vapors_css::user_agent::TEXT)),
            ),
            SheetRef::Scope(scope) => held(SheetKind::Scope, scope),
            SheetRef::Global(sheet) => held(SheetKind::Global, sheet),
        }
    }
}
