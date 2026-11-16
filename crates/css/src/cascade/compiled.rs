//! An element's style from its key's compiled rules
//! (`docs/plans/build-time-styles.md`, section 2, "Used at run time"):
//! the always rules' winning declarations entered without running the
//! matcher, at their real standing in this cascade; the key's
//! conditional rules and every rule of a sheet the key does not
//! reference matched on the element. Ties between sheets are decided by
//! this cascade's order, so the result does not depend on the order
//! components mounted.

use super::collect::{self, Attached};
use super::{Cascade, ComputedStyle, Environment, compute, inline, presentation};
use crate::Result;
use crate::selector::Element;

/// A key's rules, as indices into a cascade's rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompiledRules {
    /// (rule, declaration) of each always declaration that wins a
    /// property for the key, entered without matching.
    pub base: Vec<(usize, usize)>,
    /// The key's conditional rules, matched on the element.
    pub tested: Vec<usize>,
    /// The sheets (by the order they were added) the key's winners
    /// reference, ascending: a candidate rule of any other sheet is
    /// another crate's, matched on the element.
    pub referenced: Vec<usize>,
}

impl Cascade {
    /// The index among this cascade's rules of rule `rule` of the sheet
    /// added `sheet`-th; `None` when there is no such rule.
    #[must_use]
    pub fn rule_index(&self, sheet: usize, rule: usize) -> Option<usize> {
        let of_sheet = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.sheet == sheet);
        of_sheet.map(|(index, _)| index).nth(rule)
    }

    /// Whether a sheet not in `referenced` has a rule that is a
    /// candidate for elements of `tag` with `classes`.
    #[must_use]
    pub fn foreign_reaches(&self, tag: &str, classes: &[&str], referenced: &[usize]) -> bool {
        let mut candidates = self.buckets.candidates_for_key(tag, classes).into_iter();
        candidates.any(|index| referenced.binary_search(&self.rules[index].sheet).is_err())
    }

    /// [`Cascade::compute`] from `compiled`.
    ///
    /// # Errors
    /// As [`Cascade::compute`].
    pub fn compute_compiled<E: Element>(
        &self,
        element: &E,
        parent: Option<&ComputedStyle>,
        env: &Environment,
        compiled: &CompiledRules,
    ) -> Result<ComputedStyle> {
        let inline = inline::declarations(element)?;
        let presentation = presentation::declarations(element)?;
        let attached = Attached {
            presentation: &presentation,
            inline: &inline,
        };
        let foreign = (self.candidate_rules(element))
            .filter(|rule| compiled.referenced.binary_search(&rule.sheet).is_err());
        let winners =
            collect::winners_compiled(&self.rules, compiled, foreign, (element, env), attached);
        compute::style(&winners, parent, env)
    }
}
