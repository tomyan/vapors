//! Matching a selector against an element, right to left.

use super::structural;
use super::{
    AttributeOperator, AttributeSelector, Combinator, Compound, Element, PseudoClass, Selector,
};

/// Whether `selector` matches `element`.
pub fn matches<E: Element>(selector: &Selector, element: &E) -> bool {
    matches_anchored(selector, element, None)
}

/// Match with an optional anchor: when set, the leftmost compound must
/// additionally relate to the anchor element by the given combinator.
/// This is how `:has()` arguments are evaluated.
pub(super) fn matches_anchored<E: Element>(
    selector: &Selector,
    element: &E,
    anchor: Option<(&E, Combinator)>,
) -> bool {
    match_from(selector, selector.compounds.len() - 1, element, anchor)
}

fn match_from<E: Element>(
    sel: &Selector,
    index: usize,
    element: &E,
    anchor: Option<(&E, Combinator)>,
) -> bool {
    if !compound(&sel.compounds[index], element) {
        return false;
    }
    if index == 0 {
        return anchor.is_none_or(|(a, comb)| related(comb, element, |e| e == a));
    }
    related(sel.combinators[index - 1], element, |e| {
        match_from(sel, index - 1, e, anchor)
    })
}

/// Whether some element on the `combinator` side of `element` satisfies
/// `pred`: the parent, an ancestor, the previous sibling or any earlier
/// sibling.
fn related<E: Element>(combinator: Combinator, element: &E, pred: impl Fn(&E) -> bool) -> bool {
    match combinator {
        Combinator::Child => element.parent().is_some_and(|p| pred(&p)),
        Combinator::Descendant => {
            let mut next = element.parent();
            while let Some(e) = next {
                if pred(&e) {
                    return true;
                }
                next = e.parent();
            }
            false
        }
        Combinator::NextSibling => element.previous_sibling().is_some_and(|p| pred(&p)),
        Combinator::SubsequentSibling => {
            let mut next = element.previous_sibling();
            while let Some(e) = next {
                if pred(&e) {
                    return true;
                }
                next = e.previous_sibling();
            }
            false
        }
    }
}

fn compound<E: Element>(c: &Compound, e: &E) -> bool {
    if c.tag
        .as_ref()
        .is_some_and(|t| !t.eq_ignore_ascii_case(e.tag_name()))
    {
        return false;
    }
    if c.id.as_ref().is_some_and(|id| e.id() != Some(id.as_str())) {
        return false;
    }
    c.classes.iter().all(|class| e.has_class(class))
        && c.attributes.iter().all(|a| attribute(a, e))
        && c.pseudo_classes.iter().all(|p| pseudo(p, e))
}

fn attribute<E: Element>(a: &AttributeSelector, e: &E) -> bool {
    e.attribute(&a.name)
        .is_some_and(|value| attribute_matches(a, value))
}

/// Whether an attribute whose value is `value` satisfies `a`: the
/// operator and the case flag. The build-time analysis asks this of a
/// value a template fixes, so one function answers for both.
pub(crate) fn attribute_matches(a: &AttributeSelector, value: &str) -> bool {
    let (value, want) = if a.case_insensitive {
        (value.to_ascii_lowercase(), a.value.to_ascii_lowercase())
    } else {
        (value.to_owned(), a.value.clone())
    };
    match a.operator {
        AttributeOperator::Exists => true,
        AttributeOperator::Equals => value == want,
        AttributeOperator::Prefix => !want.is_empty() && value.starts_with(&want),
        AttributeOperator::Suffix => !want.is_empty() && value.ends_with(&want),
        AttributeOperator::Substring => !want.is_empty() && value.contains(&want),
        AttributeOperator::Includes => value.split_whitespace().any(|w| w == want),
        AttributeOperator::DashMatch => value == want || value.starts_with(&format!("{want}-")),
    }
}

fn pseudo<E: Element>(p: &PseudoClass, e: &E) -> bool {
    // one mapping from pseudo-class to flag, which sharing reads too
    if let Some(flag) = super::share::state_flag(p) {
        return e.states().contains(flag);
    }
    match p {
        PseudoClass::Empty => e.is_empty(),
        PseudoClass::Root => e.parent().is_none(),
        PseudoClass::Not(list) => !list.iter().any(|s| matches(s, e)),
        PseudoClass::Is(list) | PseudoClass::Where(list) | PseudoClass::Global(list) => {
            list.iter().any(|s| matches(s, e))
        }
        PseudoClass::Has(list) => list.iter().any(|r| structural::has(r, e)),
        other => structural::positional(other, e),
    }
}
