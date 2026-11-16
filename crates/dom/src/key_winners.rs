//! What the compiler worked out about a template key's cascade
//! (`docs/plans/build-time-styles.md`, section 2), emitted as a static
//! beside its [`StyleKey`](crate::StyleKey). Rules are named by sheet
//! and position within the sheet, never copied, so this crate holds no
//! CSS value types and no reference depends on the order sheets were
//! added at run time.

/// Whether a key carries compiled winners: three states, so a key the
/// compiler could not analyse is not taken for one it never saw.
#[derive(Clone, Copy, Debug)]
pub enum Winners {
    /// A hand-written key (tests, hand-built pages): the runtime
    /// cascades it in full.
    Unanalysed,
    /// A component tag's key: its tag is its props type's `ANCHOR_TAG`,
    /// which the analysis does not see, so the runtime cascades it in
    /// full.
    ComponentTag,
    /// The compiler's analysis.
    Compiled(&'static KeyWinners),
}

impl Winners {
    /// The compiled winners, if any.
    #[must_use]
    pub fn compiled(self) -> Option<&'static KeyWinners> {
        match self {
            Self::Compiled(winners) => Some(winners),
            Self::Unanalysed | Self::ComponentTag => None,
        }
    }
}

/// A key's compiled winners.
#[derive(Debug)]
pub struct KeyWinners {
    /// The sheets the references below index, each with its
    /// `vapors_css::sheet_hash::sheet_hash` when the compiler read it.
    pub sheets: &'static [(SheetRef, u64)],
    /// The always rules' winning declarations, at most one a property.
    pub base: &'static [BaseWinner],
    /// The key's conditional rules and every rule of another of the
    /// crate's sheets that can reach it: tested at run time on the real
    /// element.
    pub conditional: &'static [RuleRef],
    /// No candidate rule blocks sharing.
    pub shareable: bool,
}

/// A sheet by identity, as the runtime registers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetRef {
    /// The user-agent sheet.
    UserAgent,
    /// A component's scoped `<style>`, under its scope.
    Scope(&'static str),
    /// A `Styles::Global` component's `<style>`, by its file.
    Global(&'static str),
}

/// A rule: its sheet, as an index into [`KeyWinners::sheets`], and its
/// place within that sheet: its index in the sheet's parsed
/// `vapors_css::ast::Stylesheet::rules`, which the cascade keeps one for
/// one and in order (`Cascade::sheet_rule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleRef {
    /// Index into [`KeyWinners::sheets`].
    pub sheet: u8,
    /// The rule's index within its sheet.
    pub rule: u32,
}

/// The always declaration that wins one property for every element of
/// the key, unless a conditional rule beats it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseWinner {
    /// The property.
    pub property: PropertyRef,
    /// The declaration.
    pub declaration: DeclarationRef,
}

/// A property a winner is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyRef {
    /// A longhand, by `vapors_css::property::PropertyId::index`.
    Longhand(u16),
    /// A custom property, by name.
    Custom(&'static str),
}

/// Where a winning declaration is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationRef {
    /// In a rule.
    Rule {
        /// The rule.
        rule: RuleRef,
        /// The declaration's index in the rule's `declarations`, as
        /// written: before a shorthand is expanded, so every longhand of
        /// one shorthand names the same declaration.
        declaration: u32,
    },
    /// In the key's literal `style`, by its index as written there.
    Inline(u32),
    /// A presentation attribute's, by its index among the declarations
    /// the key's presentation attributes make
    /// (`vapors_css::cascade::declarations_of`).
    Presentation(u32),
}
