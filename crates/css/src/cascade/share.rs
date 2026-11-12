//! What a tree that shares computed styles between elements asks the
//! cascade (analysis PR 2): the rules a template key's elements can
//! match, whether each blocks sharing ([`unshareable`]), and the
//! attribute names and state flags any selector reads.
//!
//! The share key such a tree compares is the template key, the
//! values of [`Cascade::selected_attributes`] and the flags of
//! [`Cascade::selected_states`] on the element and on every ancestor
//! (a selector's ancestor compounds read them there), and the
//! parent's computed style (inherited values reach a child whether
//! or not the parent shared its own).
//!
//! `@media` and `@supports` conditions are frame-wide: they read the
//! [`super::Environment`] (viewport, resolution, root font size, colour
//! scheme, reduced motion), the same for every element, so they do not
//! block sharing; the tree must clear its share table whenever the
//! environment changes: the viewport, scale or font size among it. `@container` conditions read the nearest container's
//! laid-out size, which no share-key input carries, so they block it.

use super::Cascade;
use crate::ast::{Condition, Rule};
use crate::selector::reads_siblings_or_descendants;
use crate::tree::States;

/// Why a rule keeps the elements it can match from sharing one
/// computed style, for a tree to count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unshareable {
    /// Its selector reads siblings or descendants (`+`, `~`, a
    /// positional pseudo-class, `:empty`, `:has()`).
    ReadsSiblingsOrDescendants,
    /// It sits under `@container`, whose condition reads the nearest
    /// container's laid-out size.
    InContainer,
}

/// Why `rule` blocks sharing between the elements it can match, or
/// `None` when it does not.
#[must_use]
pub fn unshareable(rule: &Rule) -> Option<Unshareable> {
    if reads_siblings_or_descendants(&rule.selector) {
        return Some(Unshareable::ReadsSiblingsOrDescendants);
    }
    rule.conditions
        .iter()
        .any(reads_layout)
        .then_some(Unshareable::InContainer)
}

/// Whether `condition` reads layout rather than the frame-wide
/// environment.
fn reads_layout(condition: &Condition) -> bool {
    match condition {
        Condition::Container(_) => true,
        Condition::Media(_) | Condition::Supports(_) => false,
    }
}

impl Cascade {
    /// Adds what `selector` reads to the selected attributes and states.
    pub(super) fn note_selected(&mut self, selector: &crate::selector::Selector) {
        crate::selector::attribute_names(selector, &mut self.selected);
        self.selected_states = (self.selected_states).union(crate::selector::state_flags(selector));
    }

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
    /// id selector included: the attributes whose values the share key
    /// carries (module doc).
    #[must_use]
    pub fn selected_attributes(&self) -> &std::collections::BTreeSet<String> {
        &self.selected
    }

    /// Every state flag a selector of any sheet tests, anywhere in it:
    /// the flags the share key carries, for the element and each
    /// ancestor (module doc).
    #[must_use]
    pub fn selected_states(&self) -> States {
        self.selected_states
    }
}
