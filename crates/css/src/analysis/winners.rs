//! Per longhand and custom property, what wins for a key's elements
//! (`build-time-styles.md`, section 1): the highest-standing
//! declaration among the `Always` rules of the key's own sheets and
//! its literal `style` is a **constant winner** when no `Conditional`
//! rule declares the property at a standing that can beat it; else the
//! winner is **variable**, with the rules that can override it. The
//! standing is the cascade's (`standing.rs`): origin and importance
//! (`collect::rank`), specificity, then source order, known within a
//! sheet and decided at run time across sheets.

use std::collections::BTreeMap;

use super::sheets::Sheet;
use super::slots::Slots;
use super::{Depends, KeyFacts, Match, classify};
use crate::ast::Declaration;
use crate::cascade::{rule_indices_for_key, unshareable};

/// One candidate rule of one sheet, judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassifiedRule {
    /// Index into the sheets given to [`analyse`].
    pub sheet: usize,
    /// Index of the rule within its sheet.
    pub rule: usize,
    /// The judgement; a rule of a sheet that is not `own` is conditional
    /// (`Depends::other_sheet`) whenever it is not `Never`.
    pub verdict: Match,
}

/// Where a declaration is: its sheet (`INLINE_SHEET` for the literal
/// `style`), rule and declaration index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// Index into the sheets given to [`analyse`], or `INLINE_SHEET`.
    pub sheet: usize,
    /// The rule within the sheet; 0 for the literal `style`.
    pub rule: usize,
    /// The declaration within the rule.
    pub declaration: usize,
}

/// Whether a winner's value needs context to compute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constancy {
    /// The same under every parent, viewport, font size and scheme.
    Constant,
    /// Reads the environment, the parent's font size, `currentcolor`
    /// or a `var()`.
    Variable,
    /// Taken from the parent (`inherit`, `unset` on an inherited
    /// property, or no declaration of an inherited property), so it
    /// is constant when the parent's is.
    FromParent,
}

/// What wins for one longhand or custom property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Winner {
    /// One declaration wins for every element of the key.
    Constant {
        /// The winning declaration.
        place: Place,
        /// Its value's constancy.
        value: Constancy,
    },
    /// Decided at run time among the conditional rules named.
    Variable {
        /// The best always declaration, if any.
        base: Option<Place>,
        /// (sheet, rule) of each conditional rule that can beat it.
        overriders: Vec<(usize, usize)>,
    },
    /// No candidate rule declares it.
    Undeclared {
        /// Initial, or from the parent when the property inherits.
        value: Constancy,
    },
}

/// The analysis of one key.
#[derive(Debug)]
pub struct KeyWinners {
    /// Every candidate rule, in sheet then source order.
    pub rules: Vec<ClassifiedRule>,
    /// Per longhand, by `PropertyId::index`.
    pub longhands: Vec<Winner>,
    /// Per custom property, by name.
    pub custom: BTreeMap<String, Winner>,
    /// No candidate rule blocks sharing (`cascade::unshareable`), as
    /// the runtime decides it per key.
    pub shareable: bool,
}

/// The longhands' and custom properties' winners for the elements of
/// `facts`, over `sheets` (the user-agent sheet, the key's own scoped
/// sheet, and the crate's other sheets) and the literal `style`'s
/// declarations `inline`.
#[must_use]
pub fn analyse(sheets: &[Sheet<'_>], facts: &KeyFacts<'_>, inline: &[Declaration]) -> KeyWinners {
    let classes: Vec<&str> = facts.classes.iter().chain(facts.toggles).copied().collect();
    let mut rules = Vec::new();
    let mut shareable = true;
    let mut slots = Slots::default();
    for (index, sheet) in sheets.iter().enumerate() {
        for rule in rule_indices_for_key(sheet.rules, facts.tag, &classes) {
            let classified = classify_in(sheet, rule, index, facts);
            shareable &= unshareable(&sheet.rules.rules[rule]).is_none();
            slots.consider_rule(sheet, &classified);
            rules.push(classified);
        }
    }
    slots.consider_inline(inline);
    KeyWinners {
        rules,
        longhands: slots.longhands(),
        custom: slots.custom(),
        shareable,
    }
}

fn classify_in(
    sheet: &Sheet<'_>,
    rule: usize,
    index: usize,
    facts: &KeyFacts<'_>,
) -> ClassifiedRule {
    let verdict = match classify(&sheet.rules.rules[rule], facts) {
        Match::Never => Match::Never,
        verdict if sheet.own => verdict,
        Match::Always => Match::Conditional(Depends {
            other_sheet: true,
            ..Depends::default()
        }),
        Match::Conditional(mut depends) => {
            depends.other_sheet = true;
            Match::Conditional(depends)
        }
    };
    ClassifiedRule {
        sheet: index,
        rule,
        verdict,
    }
}
