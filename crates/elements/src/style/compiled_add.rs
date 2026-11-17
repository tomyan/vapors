//! A compiled sheet registered from a mount (`Dom::add_style`,
//! `Dom::add_global_style`): its text's hash is checked once per sheet,
//! when the sheet, its text or its claimed hash is new, not on every
//! mount of its component, which took 11 M instructions on the Mount
//! page's 10,000 rows (`docs/perf/first-frame-2026-11-16.md`).

use vapors_dom::CompiledSheet;

use super::Sheets;
use super::sheet_hashes::SheetKind;

/// A component sheet as added: its text, and the hash its last mount
/// claimed and `compiled_text` accepted (`None`: a test's sheet).
pub(crate) struct Registered {
    pub(crate) text: String,
    claimed: Option<u64>,
}

impl Registered {
    pub(crate) fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            claimed: None,
        }
    }
}

impl Sheets {
    /// `sheet` added as a `kind` sheet: true when the cascade changed.
    /// A mount of a sheet already added with this text and this claimed
    /// hash changes nothing and takes no hash; any other is checked
    /// (`compiled_text`) before it is added.
    pub(crate) fn add_compiled(&mut self, kind: SheetKind, sheet: CompiledSheet<'_>) -> bool {
        if self.already_added(kind, sheet) || !self.compiled_text(sheet) {
            return false;
        }
        let added = match kind {
            SheetKind::Scope => self.add_scoped(sheet.name(), sheet.text()),
            SheetKind::Global => self.add_global(sheet.name(), sheet.text()),
        };
        if let Some(held) = self.registered_mut(kind, sheet.name()) {
            if held.text == sheet.text() {
                held.claimed = sheet.hash();
            }
        }
        added
    }

    fn already_added(&self, kind: SheetKind, sheet: CompiledSheet<'_>) -> bool {
        let held = match kind {
            SheetKind::Scope => self.scopes.get(sheet.name()),
            SheetKind::Global => self.globals.get(sheet.name()),
        };
        held.is_some_and(|held| held.text == sheet.text() && held.claimed == sheet.hash())
    }

    fn registered_mut(&mut self, kind: SheetKind, name: &str) -> Option<&mut Registered> {
        match kind {
            SheetKind::Scope => self.scopes.get_mut(name),
            SheetKind::Global => self.globals.get_mut(name),
        }
    }
}
