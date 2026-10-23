//! The incremental restyle rule: from the selectors that use a state
//! pseudo-class, which elements a flip of that flag on one element can
//! restyle. Over-approximates, never under.

use crate::ast::Rule;
use crate::selector::{Combinator, Element, PseudoClass, Selector};
use crate::tree::States;

/// Which elements, relative to the flipped element, need restyling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // four independent scopes a caller unions; an enum would hide that
pub struct RestyleScope {
    /// The element itself: the flag is used on a subject compound.
    pub element: bool,
    /// Its descendants: the flag is used on an ancestor compound.
    pub descendants: bool,
    /// Its later siblings and their descendants: the flag is used on a
    /// compound joined by `+` or `~`.
    pub later_siblings: bool,
    /// Everything: the flag is used inside `:has()`, or inside a nested
    /// selector where the position cannot be told.
    pub whole_tree: bool,
}

impl RestyleScope {
    fn merge(&mut self, other: Self) {
        self.element |= other.element;
        self.descendants |= other.descendants;
        self.later_siblings |= other.later_siblings;
        self.whole_tree |= other.whole_tree;
    }

    /// The elements in scope for a flip on `element`, in document order.
    pub fn elements<E: Element>(&self, element: &E) -> Vec<E> {
        if self.whole_tree {
            let mut root = element.clone();
            while let Some(p) = root.parent() {
                root = p;
            }
            let mut out = vec![root.clone()];
            descendants(&root, &mut out);
            return out;
        }
        let mut out = Vec::new();
        if self.element {
            out.push(element.clone());
        }
        if self.descendants {
            descendants(element, &mut out);
        }
        if self.later_siblings {
            let mut next = element.next_sibling();
            while let Some(s) = next {
                next = s.next_sibling();
                out.push(s.clone());
                descendants(&s, &mut out);
            }
        }
        out
    }
}

fn descendants<E: Element>(e: &E, out: &mut Vec<E>) {
    for child in e.children() {
        out.push(child.clone());
        descendants(&child, out);
    }
}

/// The scope for `flag` over every rule. A rule whose declarations reach
/// into the subtree (an inherited property, `font-size` which drives em
/// and ch, or a custom property) widens an element-only scope to the
/// element's descendants.
pub(super) fn scope<'a>(rules: impl Iterator<Item = &'a Rule>, flag: States) -> RestyleScope {
    let mut scope = RestyleScope::default();
    for rule in rules {
        let mut this = selector_scope(&rule.selector, flag);
        if this.element && reaches_subtree(rule) {
            this.descendants = true;
        }
        scope.merge(this);
    }
    scope
}

fn reaches_subtree(rule: &Rule) -> bool {
    rule.declarations.iter().any(|d| {
        d.name.starts_with("--")
            || d.longhands
                .iter()
                .any(|l| l.id.inherited() || l.id.name() == "font-size")
    })
}

fn selector_scope(selector: &Selector, flag: States) -> RestyleScope {
    let mut scope = RestyleScope::default();
    let last = selector.compounds.len() - 1;
    for (i, compound) in selector.compounds.iter().enumerate() {
        let used = compound
            .pseudo_classes
            .iter()
            .fold(Use::None, |acc, p| acc.max(pseudo_use(p, flag)));
        match used {
            Use::None => {}
            Use::Elsewhere => scope.whole_tree = true,
            Use::Here if i == last => scope.element = true,
            // The combinator right after the flagged compound decides:
            // everything further right stays inside what it selects.
            Use::Here => match selector.combinators[i] {
                Combinator::NextSibling | Combinator::SubsequentSibling => {
                    scope.later_siblings = true;
                }
                Combinator::Child | Combinator::Descendant => scope.descendants = true,
            },
        }
    }
    scope
}

/// How a compound uses the flag.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Use {
    None,
    /// On this compound, directly or on the subject of a nested selector.
    Here,
    /// Somewhere whose position relative to this compound is not fixed.
    Elsewhere,
}

fn pseudo_use(p: &PseudoClass, flag: States) -> Use {
    let direct = match p {
        PseudoClass::Hover => States::HOVER,
        PseudoClass::Focus => States::FOCUS,
        PseudoClass::FocusVisible => States::FOCUS_VISIBLE,
        PseudoClass::FocusWithin => States::FOCUS_WITHIN,
        PseudoClass::Active => States::ACTIVE,
        PseudoClass::Checked => States::CHECKED,
        PseudoClass::Disabled => States::DISABLED,
        PseudoClass::Enabled => States::ENABLED,
        PseudoClass::Not(list)
        | PseudoClass::Is(list)
        | PseudoClass::Where(list)
        | PseudoClass::Global(list) => {
            return nested_use(list, flag);
        }
        PseudoClass::Has(list) => {
            let used = list
                .iter()
                .any(|r| selector_scope(&r.selector, flag) != RestyleScope::default());
            return if used { Use::Elsewhere } else { Use::None };
        }
        _ => return Use::None,
    };
    if direct == flag { Use::Here } else { Use::None }
}

/// A nested selector's subject counts as here; its other compounds are
/// elsewhere.
fn nested_use(list: &[Selector], flag: States) -> Use {
    list.iter().fold(Use::None, |acc, s| {
        let inner = selector_scope(s, flag);
        let this = if inner.descendants || inner.later_siblings || inner.whole_tree {
            Use::Elsewhere
        } else if inner.element {
            Use::Here
        } else {
            Use::None
        };
        acc.max(this)
    })
}
