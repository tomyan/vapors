//! A declaration's standing in the cascade, and one property's slot of
//! candidates: the best `Always` declaration and the `Conditional`
//! ones that may beat it.

use super::winners::{Constancy, Place, Winner};
use crate::selector::Specificity;

/// A declaration's standing: origin and importance (`collect::rank`),
/// specificity, and source order within its sheet.
#[derive(Clone, Copy)]
pub(super) struct Standing {
    pub(super) rank: u8,
    pub(super) specificity: Specificity,
    pub(super) sheet: usize,
    /// `None` for a sheet whose order against the key's own sheets is
    /// decided at run time.
    pub(super) order: Option<usize>,
}

impl Standing {
    /// `Candidate::beats` for two declarations of the key's own sheets:
    /// their ranks differ across sheets, so order compares within one.
    fn beats(self, other: Self) -> bool {
        (self.rank, self.specificity, self.order) >= (other.rank, other.specificity, other.order)
    }

    /// Whether a conditional declaration with this standing can beat
    /// the always declaration `base` at run time: on rank and
    /// specificity as the cascade compares them; on a tie, by order
    /// when both are in one sheet, else unknown, so yes.
    fn may_beat(self, base: Self) -> bool {
        match (self.rank, self.specificity).cmp(&(base.rank, base.specificity)) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => match (self.order, base.order) {
                (Some(order), Some(base_order)) if self.sheet == base.sheet => order > base_order,
                _ => true,
            },
        }
    }
}

/// One property's candidates.
pub(super) struct Slot<V> {
    base: Option<(Standing, Place, V)>,
    conditional: Vec<(Standing, (usize, usize))>,
}

impl<V> Default for Slot<V> {
    fn default() -> Self {
        Self {
            base: None,
            conditional: Vec::new(),
        }
    }
}

impl<V> Slot<V> {
    /// One declaration competes: an always one for the base, a
    /// conditional one as a possible overrider.
    pub(super) fn consider(&mut self, standing: Standing, place: Place, value: V, always: bool) {
        if !always {
            return self.conditional.push((standing, (place.sheet, place.rule)));
        }
        if self
            .base
            .as_ref()
            .is_none_or(|(current, _, _)| standing.beats(*current))
        {
            self.base = Some((standing, place, value));
        }
    }

    /// The winner, with `constancy` of the base's value and the
    /// constancy an undeclared property has.
    pub(super) fn decide(
        self,
        constancy: impl Fn(&V) -> Constancy,
        undeclared: Constancy,
    ) -> Winner {
        let Some((standing, place, value)) = self.base else {
            if self.conditional.is_empty() {
                return Winner::Undeclared { value: undeclared };
            }
            return Winner::Variable {
                base: None,
                overriders: dedup(self.conditional.into_iter().map(|(_, at)| at)),
            };
        };
        let overriders = dedup(
            (self.conditional.into_iter())
                .filter(|(other, _)| other.may_beat(standing))
                .map(|(_, at)| at),
        );
        if overriders.is_empty() {
            return Winner::Constant {
                place,
                value: constancy(&value),
            };
        }
        Winner::Variable {
            base: Some(place),
            overriders,
        }
    }
}

/// Rules in order, each once (a rule declaring a property twice).
fn dedup(places: impl Iterator<Item = (usize, usize)>) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = places.collect();
    out.dedup();
    out
}
