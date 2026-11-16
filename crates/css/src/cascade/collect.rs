//! Collecting the declarations that apply to an element and picking the
//! winner per property. A winner refers to its declaration's value; the
//! computed style is built from the references
//! (docs/plans/style-allocations.md, O2), so nothing is cloned here.

use std::collections::BTreeMap;

use super::inline::{INLINE_SHEET, INLINE_SPECIFICITY};
use super::presentation::{PRESENTATION_SHEET, PRESENTATION_SPECIFICITY};
use super::{CascadeRule, Environment, Origin};
use crate::Position;
use crate::ast::{ComponentValue, Declaration};
use crate::property::{PROPERTIES, Specified};
use crate::selector::{Element, Specificity, matches};

/// One declaration in the running for a property.
#[derive(Clone)]
pub(crate) struct Candidate<V> {
    rank: u8,
    specificity: Specificity,
    order: usize,
    pub(crate) value: V,
    pub(crate) position: Position,
    /// The sheet the declaration came from, by the order sheets were
    /// added to the cascade.
    pub(crate) sheet: usize,
}

/// Where a rule stands in the cascade: what ranks its declarations.
#[derive(Clone, Copy)]
struct Standing {
    rank: u8,
    specificity: Specificity,
    order: usize,
    sheet: usize,
}

impl<V> Candidate<V> {
    fn beats(&self, other: &Self) -> bool {
        (self.rank, self.specificity, self.order) >= (other.rank, other.specificity, other.order)
    }
}

/// Cascade Level 4 section 6.1: user agent normal, author normal, author
/// important, user agent important.
pub(crate) fn rank(origin: Origin, important: bool) -> u8 {
    match (origin, important) {
        (Origin::UserAgent, false) => 0,
        (Origin::Author, false) => 1,
        (Origin::Author, true) => 2,
        (Origin::UserAgent, true) => 3,
    }
}

/// The winning declaration per longhand (by table index) and per custom
/// property, each by reference into its declaration.
pub(crate) struct Winners<'a> {
    pub(crate) longhands: Vec<Option<Candidate<&'a Specified>>>,
    pub(crate) custom: BTreeMap<&'a str, Candidate<&'a [ComponentValue]>>,
}

/// The declarations attached to an element rather than matched: its
/// presentation attributes and its `style` attribute.
#[derive(Clone, Copy)]
pub(crate) struct Attached<'a> {
    pub(crate) presentation: &'a [Declaration],
    pub(crate) inline: &'a [Declaration],
}

/// The winners among the element's presentation attributes, the rules
/// that match `element`, and the declarations of its `style`
/// attribute. Presentation attributes go first, so an author rule of
/// equal standing (`*`) beats them by coming later.
pub(crate) fn winners<'a, E: Element>(
    rules: impl Iterator<Item = &'a CascadeRule>,
    element: &E,
    env: &Environment,
    attached: Attached<'a>,
) -> Winners<'a> {
    let mut winners = Winners {
        longhands: vec![None; PROPERTIES.len()],
        custom: BTreeMap::new(),
    };
    winners.consider_attached(attached.presentation, PRESENTATION);
    for rule in rules {
        winners.consider_rule(rule, element, env);
    }
    winners.consider_attached(attached.inline, INLINE);
    winners
}

/// [`winners`] from a key's compiled rules (`compiled.rs`): each `base`
/// declaration of `rules` entered without running the matcher, at its
/// rule's standing; the `tested` rules and the candidates of sheets the
/// key does not reference matched on `element`, as in [`winners`].
pub(crate) fn winners_compiled<'a, E: Element>(
    rules: &'a [CascadeRule],
    compiled: &super::CompiledRules,
    foreign: impl Iterator<Item = &'a CascadeRule>,
    (element, env): (&E, &Environment),
    attached: Attached<'a>,
) -> Winners<'a> {
    let mut winners = Winners {
        longhands: vec![None; PROPERTIES.len()],
        custom: BTreeMap::new(),
    };
    winners.consider_attached(attached.presentation, PRESENTATION);
    for &(rule, declaration) in &compiled.base {
        let rule = &rules[rule];
        let decl = &rule.rule.declarations[declaration];
        winners.consider(decl, standing(rule, decl));
    }
    for &rule in &compiled.tested {
        winners.consider_rule(&rules[rule], element, env);
    }
    for rule in foreign {
        winners.consider_rule(rule, element, env);
    }
    winners.consider_attached(attached.inline, INLINE);
    winners
}

/// Where `decl` of `rule` stands.
fn standing(rule: &CascadeRule, decl: &Declaration) -> Standing {
    Standing {
        rank: rank(rule.origin, decl.important),
        specificity: rule.rule.selector.specificity(),
        order: rule.order,
        sheet: rule.sheet,
    }
}

/// Where the presentation attributes stand: author normal,
/// specificity 0, first, so an author rule of equal standing beats
/// them by coming later.
const PRESENTATION: Standing = Standing {
    rank: 1,
    specificity: PRESENTATION_SPECIFICITY,
    order: 0,
    sheet: PRESENTATION_SHEET,
};

/// Where the `style` attribute stands: above every selector, last.
const INLINE: Standing = Standing {
    rank: 1,
    specificity: INLINE_SPECIFICITY,
    order: usize::MAX,
    sheet: INLINE_SHEET,
};

impl<'a> Winners<'a> {
    /// Each of `declarations`, attached to the element, at `standing`,
    /// ranked author normal or author important as each says.
    fn consider_attached(&mut self, declarations: &'a [Declaration], standing: Standing) {
        for decl in declarations {
            let rank = rank(Origin::Author, decl.important);
            self.consider(decl, Standing { rank, ..standing });
        }
    }

    /// Each declaration of `rule`, when it matches `element`.
    fn consider_rule<E: Element>(&mut self, rule: &'a CascadeRule, element: &E, env: &Environment) {
        if !rule.conditions.hold(element, env) || !matches(&rule.rule.selector, element) {
            return;
        }
        for decl in &rule.rule.declarations {
            self.consider(decl, standing(rule, decl));
        }
    }

    /// Let one declaration compete for the properties it sets.
    fn consider(&mut self, decl: &'a Declaration, standing: Standing) {
        let Standing {
            rank,
            specificity,
            order,
            sheet,
        } = standing;
        let position = decl.position;
        if decl.name.starts_with("--") {
            let new = Candidate {
                rank,
                specificity,
                order,
                value: decl.value.as_slice(),
                position,
                sheet,
            };
            match self.custom.get_mut(decl.name.as_str()) {
                Some(current) if !new.beats(current) => {}
                Some(current) => *current = new,
                None => {
                    self.custom.insert(decl.name.as_str(), new);
                }
            }
            return;
        }
        for longhand in &decl.longhands {
            let new = Candidate {
                rank,
                specificity,
                order,
                value: &longhand.value,
                position,
                sheet,
            };
            let slot = &mut self.longhands[longhand.id.index()];
            if slot.as_ref().is_none_or(|current| new.beats(current)) {
                *slot = Some(new);
            }
        }
    }
}
