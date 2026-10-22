//! Sibling-position pseudo-classes and `:has()`.

use super::matching::matches_anchored;
use super::{Combinator, Element, PseudoClass, RelativeSelector};

/// `:first-child` and the other position-based pseudo-classes.
pub(super) fn positional<E: Element>(p: &PseudoClass, e: &E) -> bool {
    let of_type = matches!(
        p,
        PseudoClass::FirstOfType
            | PseudoClass::LastOfType
            | PseudoClass::OnlyOfType
            | PseudoClass::NthOfType(_)
            | PseudoClass::NthLastOfType(_)
    );
    let before = count(e, Element::previous_sibling, of_type);
    let after = count(e, Element::next_sibling, of_type);
    let index = before + 1;
    let from_end = after + 1;
    match p {
        PseudoClass::FirstChild | PseudoClass::FirstOfType => before == 0,
        PseudoClass::LastChild | PseudoClass::LastOfType => after == 0,
        PseudoClass::OnlyChild | PseudoClass::OnlyOfType => before == 0 && after == 0,
        PseudoClass::NthChild(n) | PseudoClass::NthOfType(n) => n.matches(index),
        PseudoClass::NthLastChild(n) | PseudoClass::NthLastOfType(n) => n.matches(from_end),
        _ => unreachable!("matching.rs routes only positional pseudo-classes here"),
    }
}

/// How many siblings lie in `step`'s direction, optionally only those of
/// the same tag.
fn count<E: Element>(e: &E, step: fn(&E) -> Option<E>, of_type: bool) -> i32 {
    let mut n = 0;
    let mut next = step(e);
    while let Some(s) = next {
        if !of_type || s.tag_name().eq_ignore_ascii_case(e.tag_name()) {
            n += 1;
        }
        next = step(&s);
    }
    n
}

/// Whether some element in the relative selector's scope matches it with
/// `anchor` as the anchor.
pub(super) fn has<E: Element>(r: &RelativeSelector, anchor: &E) -> bool {
    let candidates = match r.combinator {
        Combinator::Child | Combinator::Descendant => descendants(anchor),
        Combinator::NextSibling | Combinator::SubsequentSibling => {
            let mut out = Vec::new();
            let mut next = anchor.next_sibling();
            while let Some(s) = next {
                next = s.next_sibling();
                out.push(s.clone());
                out.extend(descendants(&s));
            }
            out
        }
    };
    candidates
        .iter()
        .any(|c| matches_anchored(&r.selector, c, Some((anchor, r.combinator))))
}

fn descendants<E: Element>(e: &E) -> Vec<E> {
    let mut out = Vec::new();
    for child in e.children() {
        out.extend(descendants(&child));
        out.push(child);
    }
    out
}
