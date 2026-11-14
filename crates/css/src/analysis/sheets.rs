//! The sheets the analysis reads, each by identity: what a compiled
//! reference names at run time (`build-time-styles.md`, section 2).

use crate::ast::Stylesheet;
use crate::cascade::Origin;

/// A sheet's identity, the same at build time and run time.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SheetId {
    /// `vapors_css::user_agent`.
    UserAgent,
    /// A component's `<style>`, scoped, under its scope name.
    Scope(String),
    /// A `Styles::Global` component's `<style>`, by its file.
    Global(String),
}

/// One sheet for the analysis, parsed and, for a scope, already scoped
/// (`scope::scoped`).
#[derive(Clone, Copy, Debug)]
pub struct Sheet<'a> {
    /// Which sheet.
    pub id: &'a SheetId,
    /// Its rules.
    pub rules: &'a Stylesheet,
    /// Whether its rules may be judged `Always`: the user-agent sheet
    /// and the key's own scoped sheet, whose rank against each other
    /// never ties. A rule of any other sheet is conditional whatever
    /// its selector, since its order against the key's own rules is
    /// the order sheets were added at run time.
    pub own: bool,
}

impl Sheet<'_> {
    /// The origin its declarations are ranked under.
    #[must_use]
    pub fn origin(&self) -> Origin {
        match self.id {
            SheetId::UserAgent => Origin::UserAgent,
            SheetId::Scope(_) | SheetId::Global(_) => Origin::Author,
        }
    }
}
