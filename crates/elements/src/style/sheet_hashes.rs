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

/// A component sheet in the cascade: where it sits and its text's hash.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Held {
    /// Its index in the cascade (the order sheets were added).
    pub(crate) index: usize,
    hash: u64,
}

impl Sheets {
    /// Notes `text`, just added to the cascade as the `kind` sheet
    /// `name`: the last sheet added.
    pub(crate) fn note_sheet(&mut self, kind: SheetKind, name: &str, text: &str) {
        let held = Held {
            index: self.names.len() - 1,
            hash: sheet_hash(text),
        };
        self.components.insert((kind, name.to_owned()), held);
    }

    /// The component sheet `sheet` names, if it is in the cascade: one
    /// table, by kind and name, for its hash (check 1) and its place
    /// (the winners oracle, the compiled rules).
    pub(crate) fn held(&self, sheet: SheetRef) -> Option<Held> {
        let (kind, name) = match sheet {
            SheetRef::UserAgent => {
                return Some(Held {
                    index: super::USER_AGENT_SHEET,
                    hash: user_agent_hash(),
                });
            }
            SheetRef::Scope(name) => (SheetKind::Scope, name),
            SheetRef::Global(name) => (SheetKind::Global, name),
        };
        self.components.get(&(kind, name.to_owned())).copied()
    }

    /// The hash of the sheet in the cascade as `sheet`.
    pub(crate) fn hash_of(&self, sheet: SheetRef) -> SheetHash {
        self.held(sheet)
            .map_or(SheetHash::Absent, |held| SheetHash::Registered(held.hash))
    }
}

fn user_agent_hash() -> u64 {
    static USER_AGENT: OnceLock<u64> = OnceLock::new();
    *USER_AGENT.get_or_init(|| sheet_hash(vapors_css::user_agent::TEXT))
}
