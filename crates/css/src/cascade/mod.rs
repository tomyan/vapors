//! The cascade: stylesheets by origin in, computed styles per element out.
//!
//! Cascade order (Cascade Level 4 section 6): origin and importance
//! (user agent normal, author normal, author important, user agent
//! important), then specificity, then source order. Unmatched properties
//! inherit when the table says so, else take their initial value.

mod buckets;
mod collect;
mod compute;
mod compute_resolve;
mod computed;
mod conditions;
mod environment;
mod media;
mod media_eval;
mod media_feature;
mod media_items;
mod media_range;
mod restyle;
mod supports;

pub use computed::ComputedStyle;
pub use environment::Environment;
pub use restyle::RestyleScope;

pub(crate) use compute::resolve_for;

use crate::Result;
use crate::ast::{Keyframes, Rule, Stylesheet};
use crate::selector::Element;
use conditions::Conditions;

/// Where a stylesheet comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The element layer's defaults.
    UserAgent,
    /// The component's `<style>` and any sheet the app adds.
    Author,
}

/// Check one at-rule condition at parse time.
pub(crate) fn validate_condition(condition: &crate::ast::Condition) -> Result<()> {
    Conditions::parse(std::slice::from_ref(condition)).map(|_| ())
}

/// One rule with its conditions parsed, in cascade order.
pub(crate) struct CascadeRule {
    pub(crate) origin: Origin,
    pub(crate) rule: Rule,
    pub(crate) conditions: Conditions,
    /// Position in the combined source order.
    pub(crate) order: usize,
}

/// Stylesheets by origin, ready to compute styles.
pub struct Cascade {
    rules: Vec<CascadeRule>,
    keyframes: Vec<Keyframes>,
    buckets: buckets::Buckets,
}

impl Default for Cascade {
    fn default() -> Self {
        Self::new()
    }
}

impl Cascade {
    /// An empty cascade.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            keyframes: Vec::new(),
            buckets: buckets::Buckets::default(),
        }
    }

    /// Add a user-agent stylesheet.
    #[must_use]
    pub fn user_agent(self, sheet: Stylesheet) -> Self {
        self.add(Origin::UserAgent, sheet)
    }

    /// Add an author stylesheet.
    #[must_use]
    pub fn author(self, sheet: Stylesheet) -> Self {
        self.add(Origin::Author, sheet)
    }

    fn add(mut self, origin: Origin, sheet: Stylesheet) -> Self {
        for rule in sheet.rules {
            let conditions =
                Conditions::parse(&rule.conditions).expect("the parser accepted these conditions");
            self.buckets.insert(&rule.selector, self.rules.len());
            self.rules.push(CascadeRule {
                origin,
                order: self.rules.len(),
                rule,
                conditions,
            });
        }
        self.keyframes.extend(sheet.keyframes);
        self
    }

    /// The `@keyframes` block named `name`; the last one declared wins.
    #[must_use]
    pub fn keyframes(&self, name: &str) -> Option<&Keyframes> {
        self.keyframes.iter().rev().find(|k| k.name == name)
    }

    /// The rules whose subject compound could match `element`: those
    /// bucketed under its id, one of its classes, its tag, or none of
    /// these, in source order. The matcher runs on these only.
    pub fn candidates<E: Element>(&self, element: &E) -> impl Iterator<Item = &Rule> {
        self.candidate_rules(element).map(|r| &r.rule)
    }

    pub(crate) fn candidate_rules<E: Element>(
        &self,
        element: &E,
    ) -> impl Iterator<Item = &CascadeRule> {
        self.buckets
            .candidates(element)
            .into_iter()
            .map(|i| &self.rules[i])
    }

    /// The computed style of one element given its parent's.
    ///
    /// # Errors
    /// A `var()` that does not resolve, or a substituted value that fails
    /// the property's grammar, naming the declaration's position.
    pub fn compute<E: Element>(
        &self,
        element: &E,
        parent: Option<&ComputedStyle>,
        env: &Environment,
    ) -> Result<ComputedStyle> {
        let winners = collect::winners(self.candidate_rules(element), element, env);
        compute::style(&winners, parent, env)
    }

    /// The computed style of `root` and every descendant, in document
    /// order.
    ///
    /// # Errors
    /// As [`Cascade::compute`], for the first element that fails.
    pub fn compute_tree<E: Element>(
        &self,
        root: &E,
        env: &Environment,
    ) -> Result<Vec<(E, ComputedStyle)>> {
        let mut out = Vec::new();
        self.compute_subtree(root, None, env, &mut out)?;
        Ok(out)
    }

    fn compute_subtree<E: Element>(
        &self,
        element: &E,
        parent: Option<&ComputedStyle>,
        env: &Environment,
        out: &mut Vec<(E, ComputedStyle)>,
    ) -> Result<()> {
        let style = self.compute(element, parent, env)?;
        let index = out.len();
        out.push((element.clone(), style));
        for child in element.children() {
            let parent_style = out[index].1.clone();
            self.compute_subtree(&child, Some(&parent_style), env, out)?;
        }
        Ok(())
    }

    /// Which elements a flip of `flag` on one element can restyle. See
    /// `docs/css.md`, "Incremental restyle".
    #[must_use]
    pub fn restyle_scope(&self, flag: crate::tree::States) -> RestyleScope {
        restyle::scope(self.rules.iter().map(|r| &r.rule), flag)
    }
}
