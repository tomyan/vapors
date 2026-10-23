//! Collecting the declarations that apply to an element and picking the
//! winner per property.

use std::collections::BTreeMap;

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
fn rank(origin: Origin, important: bool) -> u8 {
    match (origin, important) {
        (Origin::UserAgent, false) => 0,
        (Origin::Author, false) => 1,
        (Origin::Author, true) => 2,
        (Origin::UserAgent, true) => 3,
    }
}

/// The winning declaration per longhand (by table index) and per custom
/// property.
pub(crate) struct Winners {
    pub(crate) longhands: Vec<Option<Candidate<Specified>>>,
    pub(crate) custom: BTreeMap<String, Candidate<Vec<ComponentValue>>>,
}

pub(crate) fn winners<'a, E: Element>(
    rules: impl Iterator<Item = &'a CascadeRule>,
    element: &E,
    env: &Environment,
) -> Winners {
    let mut winners = Winners {
        longhands: vec![None; PROPERTIES.len()],
        custom: BTreeMap::new(),
    };
    for rule in rules {
        if !rule.conditions.hold(element, env) || !matches(&rule.rule.selector, element) {
            continue;
        }
        let specificity = rule.rule.selector.specificity();
        for decl in &rule.rule.declarations {
            let standing = Standing {
                rank: rank(rule.origin, decl.important),
                specificity,
                order: rule.order,
                sheet: rule.sheet,
            };
            winners.consider(decl, standing);
        }
    }
    winners
}

impl Winners {
    /// Let one declaration compete for the properties it sets.
    fn consider(&mut self, decl: &Declaration, standing: Standing) {
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
                value: decl.value.clone(),
                position,
                sheet,
            };
            match self.custom.get_mut(&decl.name) {
                Some(current) if !new.beats(current) => {}
                Some(current) => *current = new,
                None => {
                    self.custom.insert(decl.name.clone(), new);
                }
            }
            return;
        }
        for longhand in &decl.longhands {
            let new = Candidate {
                rank,
                specificity,
                order,
                value: longhand.value.clone(),
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
