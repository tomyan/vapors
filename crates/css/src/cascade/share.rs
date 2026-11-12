//! What a tree that shares computed styles between elements asks the
//! cascade (analysis PR 2): the rules a template key's elements can
//! match, and the attribute names any selector reads.

use super::Cascade;
use crate::ast::Rule;

impl Cascade {
    /// The rules any element a template key with `tag` and `classes`
    /// (its literal classes and every `class:` directive's) can match:
    /// a superset of [`Cascade::candidates`] for each such element.
    pub fn rules_for_key(&self, tag: &str, classes: &[&str]) -> impl Iterator<Item = &Rule> {
        self.buckets
            .candidates_for_key(tag, classes)
            .into_iter()
            .map(|i| &self.rules[i].rule)
    }

    /// Every attribute name a selector of any sheet reads, `id` for an
    /// id selector included: what a tree sharing computed styles must
    /// compare between elements.
    #[must_use]
    pub fn selected_attributes(&self) -> &std::collections::BTreeSet<String> {
        &self.selected
    }
}
