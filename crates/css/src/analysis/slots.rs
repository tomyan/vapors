//! The per-property slots the winners analysis fills: every candidate
//! declaration of every classified rule and of the literal `style`,
//! then each slot decided (`standing.rs`).

use std::collections::BTreeMap;

use super::Match;
use super::sheets::Sheet;
use super::standing::{Slot, Standing};
use super::value::is_constant;
use super::winners::{ClassifiedRule, Constancy, Place, Winner};
use crate::ast::Declaration;
use crate::cascade::{INLINE_SHEET, INLINE_SPECIFICITY, Origin, initial, is_parent_relative, rank};
use crate::property::{PROPERTIES, PropertyId, Specified, contains_var};

pub(super) struct Slots {
    longhands: Vec<Slot<Specified>>,
    custom: BTreeMap<String, Slot<bool>>,
}

impl Default for Slots {
    fn default() -> Self {
        Self {
            longhands: (0..PROPERTIES.len()).map(|_| Slot::default()).collect(),
            custom: BTreeMap::new(),
        }
    }
}

impl Slots {
    pub(super) fn consider_rule(&mut self, sheet: &Sheet<'_>, classified: &ClassifiedRule) {
        let always = match classified.verdict {
            Match::Never => return,
            Match::Always => true,
            Match::Conditional(_) => false,
        };
        let rule = &sheet.rules.rules[classified.rule];
        let specificity = rule.selector.specificity();
        for (declaration, decl) in rule.declarations.iter().enumerate() {
            let standing = Standing {
                rank: rank(sheet.origin(), decl.important),
                specificity,
                sheet: classified.sheet,
                order: sheet.own.then_some(classified.rule),
            };
            let place = Place {
                sheet: classified.sheet,
                rule: classified.rule,
                declaration,
            };
            self.consider(decl, standing, place, always);
        }
    }

    pub(super) fn consider_inline(&mut self, inline: &[Declaration]) {
        for (declaration, decl) in inline.iter().enumerate() {
            let standing = Standing {
                rank: rank(Origin::Author, decl.important),
                specificity: INLINE_SPECIFICITY,
                sheet: INLINE_SHEET,
                order: Some(usize::MAX),
            };
            let place = Place {
                sheet: INLINE_SHEET,
                rule: 0,
                declaration,
            };
            self.consider(decl, standing, place, true);
        }
    }

    fn consider(&mut self, decl: &Declaration, standing: Standing, place: Place, always: bool) {
        if decl.name.starts_with("--") {
            let slot = self.custom.entry(decl.name.clone()).or_default();
            return slot.consider(standing, place, contains_var(&decl.value), always);
        }
        for longhand in &decl.longhands {
            let slot = &mut self.longhands[longhand.id.index()];
            slot.consider(standing, place, longhand.value.clone(), always);
        }
    }

    /// Every slot decided: the longhands by table index, the custom
    /// properties by name.
    pub(super) fn decide(self) -> (Vec<Winner>, BTreeMap<String, Winner>) {
        let Self { longhands, custom } = self;
        let longhands = (longhands.into_iter().zip(PropertyId::all()))
            .map(|(slot, id)| {
                let undeclared = if id.inherited() {
                    Constancy::FromParent
                } else {
                    initial_constancy(id)
                };
                slot.decide(|value| specified_constancy(value, id), undeclared)
            })
            .collect();
        let custom_constancy = |has_var: &bool| {
            if *has_var {
                Constancy::Variable
            } else {
                Constancy::Constant
            }
        };
        let custom = (custom.into_iter())
            .map(|(name, slot)| (name, slot.decide(custom_constancy, Constancy::FromParent)))
            .collect();
        (longhands, custom)
    }
}

/// `bolder` and `lighter` compute from the parent's weight
/// (`cascade::font_weight`); every other value by its own shape.
fn specified_constancy(value: &Specified, id: PropertyId) -> Constancy {
    match value {
        Specified::Value(value) if id.name() == "font-weight" && is_parent_relative(value) => {
            Constancy::FromParent
        }
        Specified::Value(value) if is_constant(value) => Constancy::Constant,
        Specified::Value(_) | Specified::Pending { .. } => Constancy::Variable,
        Specified::Inherit => Constancy::FromParent,
        Specified::Unset if id.inherited() => Constancy::FromParent,
        Specified::Initial | Specified::Unset => initial_constancy(id),
    }
}

/// The constancy of `id`'s initial value, which `initial`, `unset` on
/// a non-inherited property and no declaration all compute to: a
/// `currentcolor` initial (the border and outline colours) reads the
/// element's `color`.
fn initial_constancy(id: PropertyId) -> Constancy {
    if is_constant(initial(id)) {
        Constancy::Constant
    } else {
        Constancy::Variable
    }
}
