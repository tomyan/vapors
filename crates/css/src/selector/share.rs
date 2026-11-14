//! What a selector reads beyond the element and its ancestors, for a
//! tree that shares one computed style between elements whose own
//! and ancestors' inputs are equal (analysis PR 2): a selector that
//! reads siblings or descendants cannot be answered from that chain,
//! and the attribute names and state flags selectors read are the
//! ones such a tree must compare. Each walk goes through [`reads`] and
//! [`crosses_siblings`], which list every pseudo-class and combinator,
//! so a new form fails to compile here rather than being skipped.

use std::collections::BTreeSet;

use super::{Combinator, Compound, PseudoClass, RelativeSelector, Selector};
use crate::tree::States;

/// What one pseudo-class reads.
pub(crate) enum Reads<'s> {
    /// The selectors inside it, on the same element (`:is()`,
    /// `:where()`, `:global()`).
    Nested(&'s [Selector]),
    /// `:not()`: the selectors inside it, none of which may match.
    Negated(&'s [Selector]),
    /// `:has()`: the selectors after it, on descendants or siblings.
    Relative(&'s [RelativeSelector]),
    /// Siblings or descendants: the positional pseudo-classes, `:empty`.
    Structure,
    /// A state flag the tree sets on the element.
    State(States),
    /// Whether the element has a parent: the chain only.
    Root,
}

pub(crate) fn reads(pseudo: &PseudoClass) -> Reads<'_> {
    match pseudo {
        PseudoClass::Not(list) => Reads::Negated(list),
        PseudoClass::Is(list) | PseudoClass::Where(list) | PseudoClass::Global(list) => {
            Reads::Nested(list)
        }
        PseudoClass::Has(relatives) => Reads::Relative(relatives),
        PseudoClass::Empty
        | PseudoClass::FirstChild
        | PseudoClass::LastChild
        | PseudoClass::OnlyChild
        | PseudoClass::FirstOfType
        | PseudoClass::LastOfType
        | PseudoClass::OnlyOfType
        | PseudoClass::NthChild(_)
        | PseudoClass::NthLastChild(_)
        | PseudoClass::NthOfType(_)
        | PseudoClass::NthLastOfType(_) => Reads::Structure,
        PseudoClass::Hover => Reads::State(States::HOVER),
        PseudoClass::Focus => Reads::State(States::FOCUS),
        PseudoClass::FocusVisible => Reads::State(States::FOCUS_VISIBLE),
        PseudoClass::FocusWithin => Reads::State(States::FOCUS_WITHIN),
        PseudoClass::Active => Reads::State(States::ACTIVE),
        PseudoClass::Checked => Reads::State(States::CHECKED),
        PseudoClass::Disabled => Reads::State(States::DISABLED),
        PseudoClass::Enabled => Reads::State(States::ENABLED),
        PseudoClass::Invalid => Reads::State(States::INVALID),
        PseudoClass::Root => Reads::Root,
    }
}

/// The state flag `pseudo` tests, for matching; `None` for one that
/// is not a state.
pub(crate) fn state_flag(pseudo: &PseudoClass) -> Option<States> {
    match reads(pseudo) {
        Reads::State(flag) => Some(flag),
        Reads::Nested(_)
        | Reads::Negated(_)
        | Reads::Relative(_)
        | Reads::Structure
        | Reads::Root => None,
    }
}

/// Each state pseudo-class with its flag, as [`reads`] maps them: the
/// one list that names a flag (`state_names`).
const STATE_PSEUDO_CLASSES: [(&str, States); 9] = [
    (":hover", States::HOVER),
    (":focus", States::FOCUS),
    (":focus-visible", States::FOCUS_VISIBLE),
    (":focus-within", States::FOCUS_WITHIN),
    (":active", States::ACTIVE),
    (":checked", States::CHECKED),
    (":disabled", States::DISABLED),
    (":enabled", States::ENABLED),
    (":invalid", States::INVALID),
];

/// The pseudo-class names of the flags in `states`, for messages.
#[must_use]
pub fn state_names(states: States) -> Vec<&'static str> {
    STATE_PSEUDO_CLASSES
        .iter()
        .filter(|(_, flag)| states.contains(*flag))
        .map(|(name, _)| *name)
        .collect()
}

/// Whether `combinator` steps to a sibling rather than an ancestor.
fn crosses_siblings(combinator: Combinator) -> bool {
    match combinator {
        Combinator::NextSibling | Combinator::SubsequentSibling => true,
        Combinator::Descendant | Combinator::Child => false,
    }
}

/// Whether matching `selector` can read an element's siblings
/// (`+`, `~`, the positional pseudo-classes), its descendants
/// (`:empty`, `:has()`), anywhere in it, including inside `:not()`,
/// `:is()`, `:where()`, `:global()` and `:has()`. A tree deciding
/// whether a rule blocks sharing asks [`crate::cascade::unshareable`],
/// which also covers the rule's `@container` conditions.
pub(crate) fn reads_siblings_or_descendants(selector: &Selector) -> bool {
    selector.combinators.iter().any(|c| crosses_siblings(*c))
        || selector.compounds.iter().any(reads_beyond_chain)
}

fn reads_beyond_chain(compound: &Compound) -> bool {
    compound
        .pseudo_classes
        .iter()
        .any(|pseudo| match reads(pseudo) {
            Reads::Nested(list) | Reads::Negated(list) => {
                list.iter().any(reads_siblings_or_descendants)
            }
            Reads::Relative(_) | Reads::Structure => true,
            Reads::State(_) | Reads::Root => false,
        })
}

/// Adds to `names` every attribute `selector` reads: the name of each
/// attribute selector, and `id` for an id selector, anywhere in it.
pub fn attribute_names(selector: &Selector, names: &mut BTreeSet<String>) {
    for compound in &selector.compounds {
        if compound.id.is_some() {
            names.insert("id".to_owned());
        }
        for attribute in &compound.attributes {
            names.insert(attribute.name.clone());
        }
        for selector in nested(compound) {
            attribute_names(selector, names);
        }
    }
}

/// Every state flag `selector` tests, anywhere in it.
#[must_use]
pub fn state_flags(selector: &Selector) -> States {
    let mut flags = States::NONE;
    for compound in &selector.compounds {
        for pseudo in &compound.pseudo_classes {
            flags = flags.union(state_flag(pseudo).unwrap_or(States::NONE));
        }
        for selector in nested(compound) {
            flags = flags.union(state_flags(selector));
        }
    }
    flags
}

/// The selectors inside `compound`'s pseudo-classes, `:has()`'s
/// included.
fn nested(compound: &Compound) -> Vec<&Selector> {
    let mut out = Vec::new();
    for pseudo in &compound.pseudo_classes {
        match reads(pseudo) {
            Reads::Nested(list) | Reads::Negated(list) => out.extend(list),
            Reads::Relative(relatives) => {
                out.extend(
                    relatives
                        .iter()
                        .map(|RelativeSelector { selector, .. }| selector),
                );
            }
            Reads::Structure | Reads::State(_) | Reads::Root => {}
        }
    }
    out
}
