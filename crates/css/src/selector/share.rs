//! What a selector reads beyond the element and its ancestors, for a
//! tree that shares one computed style between elements whose own
//! and ancestors' inputs are equal (analysis PR 2): a selector that
//! reads siblings or descendants cannot be answered from that chain,
//! and the attribute names selectors read are the ones such a tree
//! must compare.

use std::collections::BTreeSet;

use super::{Combinator, Compound, PseudoClass, RelativeSelector, Selector};

/// Whether matching `selector` can read an element's siblings
/// (`+`, `~`, the positional pseudo-classes), its descendants
/// (`:empty`, `:has()`), anywhere in it, including inside `:not()`,
/// `:is()`, `:where()`, `:global()` and `:has()`.
#[must_use]
pub fn reads_siblings_or_descendants(selector: &Selector) -> bool {
    selector
        .combinators
        .iter()
        .any(|c| matches!(c, Combinator::NextSibling | Combinator::SubsequentSibling))
        || selector.compounds.iter().any(reads_beyond_chain)
}

fn reads_beyond_chain(compound: &Compound) -> bool {
    compound.pseudo_classes.iter().any(|pseudo| match pseudo {
        PseudoClass::Not(list)
        | PseudoClass::Is(list)
        | PseudoClass::Where(list)
        | PseudoClass::Global(list) => list.iter().any(reads_siblings_or_descendants),
        PseudoClass::Has(_)
        | PseudoClass::Empty
        | PseudoClass::FirstChild
        | PseudoClass::LastChild
        | PseudoClass::OnlyChild
        | PseudoClass::FirstOfType
        | PseudoClass::LastOfType
        | PseudoClass::OnlyOfType
        | PseudoClass::NthChild(_)
        | PseudoClass::NthLastChild(_)
        | PseudoClass::NthOfType(_)
        | PseudoClass::NthLastOfType(_) => true,
        PseudoClass::Hover
        | PseudoClass::Focus
        | PseudoClass::FocusVisible
        | PseudoClass::FocusWithin
        | PseudoClass::Active
        | PseudoClass::Checked
        | PseudoClass::Disabled
        | PseudoClass::Enabled
        | PseudoClass::Invalid
        | PseudoClass::Root => false,
    })
}

/// Adds to `names` every attribute `selector` reads: the name of each
/// attribute selector, and `id` for an id selector, anywhere in it.
pub fn attribute_names(selector: &Selector, names: &mut BTreeSet<String>) {
    for compound in &selector.compounds {
        compound_attribute_names(compound, names);
    }
}

fn compound_attribute_names(compound: &Compound, names: &mut BTreeSet<String>) {
    if compound.id.is_some() {
        names.insert("id".to_owned());
    }
    for attribute in &compound.attributes {
        names.insert(attribute.name.clone());
    }
    for pseudo in &compound.pseudo_classes {
        match pseudo {
            PseudoClass::Not(list)
            | PseudoClass::Is(list)
            | PseudoClass::Where(list)
            | PseudoClass::Global(list) => list.iter().for_each(|s| attribute_names(s, names)),
            PseudoClass::Has(relatives) => {
                for RelativeSelector { selector, .. } in relatives {
                    attribute_names(selector, names);
                }
            }
            _ => {}
        }
    }
}
