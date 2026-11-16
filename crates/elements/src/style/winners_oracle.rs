//! Debug builds: the compiler's emitted winners held against this
//! cascade's answer for each element computed from a key (review of
//! #429, M1), so the two answers to "which declaration wins" agree
//! before B3a uses the emitted one. Per longhand, the cascade's winning
//! declaration must be the key's base winner, a declaration of one of
//! its conditional rules, or one of a sheet its winners do not
//! reference (another crate's, ranked at run time); with no base winner
//! it may also be none. Custom properties are not checked here: the
//! computed style keeps no position for them; B3a's oracle compares
//! them.

use std::collections::{HashMap, HashSet};

use vapors_css::Position;
use vapors_css::cascade::{ComputedStyle, INLINE_SHEET, PRESENTATION_SHEET};
use vapors_css::property::PropertyId;
use vapors_dom::{DeclarationRef, KeyWinners, PropertyRef, SheetRef, StyleKey};

use super::{Sheets, USER_AGENT_SHEET};

/// Where the cascade's sheets hold a declaration: (sheet, position), or
/// the inline or presentation sheet by itself.
type Place = (usize, Option<Position>);

impl Sheets {
    /// Panics naming `element`, its key's site and the property when
    /// `style` took a declaration `key`'s winners do not allow. A key
    /// naming a global sheet is not checked: until R1a the cascade holds
    /// global sheets by name, not by file (`SheetHash::Unresolvable`);
    /// R1a retires that.
    pub(crate) fn check_winners(&self, key: &StyleKey, element: &str, style: &ComputedStyle) {
        let Some(winners) = key.winners.compiled() else {
            return;
        };
        let Some(sheets) = self.cascade_sheets(winners) else {
            return;
        };
        let base = self.base_places(winners, &sheets);
        let conditional = self.conditional_places(winners, &sheets);
        let referenced: HashSet<usize> = sheets.iter().flatten().copied().collect();
        for id in PropertyId::all() {
            let won = style
                .declared_in(id)
                .map(|sheet| winning_place(sheet, style.declared_at(id)));
            let expected = base.get(&id.index()).copied();
            assert!(
                allowed(won, expected, &conditional, &referenced),
                "the compiled winners of {element} (key at {}) allow {expected:?} for `{}`, but the \
                 cascade's winner is {won:?} (sheet, position); vapors_css::analysis and the cascade \
                 disagree on this key. Reproduce with the failing test",
                key.site,
                id.name()
            );
        }
    }

    /// Each of `winners.sheets` as its index in the cascade (`None`
    /// when absent); `None` when one is a global sheet.
    fn cascade_sheets(&self, winners: &KeyWinners) -> Option<Vec<Option<usize>>> {
        let index = |sheet| match sheet {
            SheetRef::UserAgent => Some(Some(USER_AGENT_SHEET)),
            SheetRef::Scope(scope) => Some(self.names.iter().position(|name| name == scope)),
            SheetRef::Global(_) => None,
        };
        winners
            .sheets
            .iter()
            .map(|&(sheet, _)| index(sheet))
            .collect()
    }

    fn base_places(&self, winners: &KeyWinners, sheets: &[Option<usize>]) -> HashMap<usize, Place> {
        let mut places = HashMap::new();
        for winner in winners.base {
            let PropertyRef::Longhand(index) = winner.property else {
                continue;
            };
            let place = match winner.declaration {
                DeclarationRef::Inline(_) => Some((INLINE_SHEET, None)),
                DeclarationRef::Presentation(_) => Some((PRESENTATION_SHEET, None)),
                DeclarationRef::Rule { rule, declaration } => self
                    .rule_place(sheets, rule, Some(declaration))
                    .and_then(|mut found| found.pop()),
            };
            places.extend(place.map(|place| (usize::from(index), place)));
        }
        places
    }

    fn conditional_places(&self, winners: &KeyWinners, sheets: &[Option<usize>]) -> HashSet<Place> {
        let places = winners.conditional.iter();
        places
            .filter_map(|&rule| self.rule_place(sheets, rule, None))
            .flatten()
            .collect()
    }

    /// The places of a rule's declarations: the one at `declaration`, or
    /// all of them; `None` when its sheet is absent.
    fn rule_place(
        &self,
        sheets: &[Option<usize>],
        rule: vapors_dom::RuleRef,
        declaration: Option<u32>,
    ) -> Option<Vec<Place>> {
        let sheet = sheets[usize::from(rule.sheet)]?;
        let found = self.cascade.sheet_rule(sheet, rule.rule as usize)?;
        let declarations = found.declarations.iter().enumerate();
        let chosen = declarations.filter(|(at, _)| declaration.is_none_or(|d| d as usize == *at));
        Some(chosen.map(|(_, d)| (sheet, Some(d.position))).collect())
    }
}

/// Whether the cascade's winner `won` is one the key's winners allow:
/// the base winner `expected`, a conditional rule's declaration, or one
/// of a sheet they do not reference (inline and presentation are the
/// key's own, so never foreign); none only when no base winner exists.
fn allowed(
    won: Option<Place>,
    expected: Option<Place>,
    conditional: &HashSet<Place>,
    referenced: &HashSet<usize>,
) -> bool {
    let Some(place) = won else {
        return expected.is_none();
    };
    let foreign = place.1.is_some() && !referenced.contains(&place.0);
    expected == Some(place) || conditional.contains(&place) || foreign
}

/// The cascade's winning place: inline and presentation declarations by
/// their sheet alone.
fn winning_place(sheet: usize, position: Option<Position>) -> Place {
    if sheet == INLINE_SHEET || sheet == PRESENTATION_SHEET {
        return (sheet, None);
    }
    (sheet, position)
}
