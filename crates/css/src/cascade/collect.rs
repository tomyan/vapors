//! Collecting the declarations that apply to an element and picking the
//! winner per property.

use std::collections::BTreeMap;

use super::{CascadeRule, Environment, Origin};
use crate::Position;
use crate::ast::ComponentValue;
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
            let rank = rank(rule.origin, decl.important);
            if decl.name.starts_with("--") {
                let candidate = Candidate {
                    rank,
                    specificity,
                    order: rule.order,
                    value: decl.value.clone(),
                    position: decl.position,
                };
                let slot = winners.custom.entry(decl.name.clone());
                slot.and_modify(|current| {
                    if candidate.beats(current) {
                        *current = candidate.clone();
                    }
                })
                .or_insert(candidate);
                continue;
            }
            for longhand in &decl.longhands {
                let candidate = Candidate {
                    rank,
                    specificity,
                    order: rule.order,
                    value: longhand.value.clone(),
                    position: decl.position,
                };
                let slot = &mut winners.longhands[longhand.id.index()];
                if slot.as_ref().is_none_or(|current| candidate.beats(current)) {
                    *slot = Some(candidate);
                }
            }
        }
    }
    winners
}
