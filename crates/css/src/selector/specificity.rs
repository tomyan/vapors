//! Specificity, Selectors Level 4 section 17.

use std::cmp::Ordering;

use super::{Compound, PseudoClass, Selector};

/// Specificity, Selectors Level 4 section 17: ids, then classes
/// (attributes and pseudo-classes included), then types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Specificity {
    /// Id selectors.
    pub ids: u32,
    /// Class and attribute selectors and pseudo-classes.
    pub classes: u32,
    /// Type selectors.
    pub types: u32,
}

impl Specificity {
    fn add(self, other: Self) -> Self {
        Self {
            ids: self.ids + other.ids,
            classes: self.classes + other.classes,
            types: self.types + other.types,
        }
    }
}

impl PartialOrd for Specificity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Specificity {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.ids, self.classes, self.types).cmp(&(other.ids, other.classes, other.types))
    }
}

impl Selector {
    /// The selector's specificity.
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        self.compounds
            .iter()
            .fold(Specificity::default(), |acc, c| acc.add(c.specificity()))
    }
}

impl Compound {
    fn specificity(&self) -> Specificity {
        let mut s = Specificity {
            ids: u32::from(self.id.is_some()),
            classes: u32::try_from(self.classes.len() + self.attributes.len()).unwrap_or(u32::MAX),
            types: u32::from(self.tag.is_some()),
        };
        for p in &self.pseudo_classes {
            s = s.add(p.specificity());
        }
        s
    }
}

impl PseudoClass {
    /// `:where()` adds nothing; `:not()`, `:is()`, `:global()` and `:has()` add their
    /// most specific argument; every other pseudo-class counts as a class.
    fn specificity(&self) -> Specificity {
        match self {
            Self::Where(_) => Specificity::default(),
            Self::Not(list) | Self::Is(list) | Self::Global(list) => most_specific(list.iter()),
            Self::Has(list) => most_specific(list.iter().map(|r| &r.selector)),
            _ => Specificity {
                classes: 1,
                ..Specificity::default()
            },
        }
    }
}

fn most_specific<'a>(list: impl Iterator<Item = &'a Selector>) -> Specificity {
    list.map(Selector::specificity).max().unwrap_or_default()
}
