//! What a child inserted under or removed from a parent restyles
//! (splice.md (b2)): the inserted subtree, and among the siblings only
//! those the sheets' sibling combinators and positional pseudo-classes
//! can reach from the change (`SiblingDeps`, kept per sheet added).

use vapors_css::selector::Element as _;
use vapors_dom::NodeId;

use crate::css_element::ElementRef;
use crate::document::Document;
use crate::node_map::NodeSet;
use crate::style::SiblingDeps;

impl Document {
    /// Marks what `child` restyles: inserted under `parent` and already
    /// in place when `added`, else about to be removed from it and
    /// still in place.
    pub(crate) fn mark_children_changed(&mut self, parent: NodeId, child: NodeId, added: bool) {
        let features = self.sheets.features;
        if features.has {
            return self.dirty.everything();
        }
        if features.empty {
            return self.dirty.subtree(parent);
        }
        if added {
            self.dirty.subtree(child);
            // a moved child's old walks say nothing about its new siblings
            self.dirty.walked.forget(child);
        }
        // a text node moves no element among its siblings
        if self.arena.get(child, "restyle").element().is_some() {
            self.dirty.narrowed = true;
            self.mark_siblings(child, features.siblings);
        }
    }

    /// Marks the siblings of `child` that `deps` reaches: each alone,
    /// or with its subtree when a use is in a group before the subject's.
    fn mark_siblings(&mut self, child: NodeId, deps: SiblingDeps) {
        let element = ElementRef::new(&self.arena, child);
        for node in reached(element, deps, &mut self.dirty.walked) {
            if deps.descendants {
                self.dirty.subtree(node);
            } else {
                self.dirty.element(node);
            }
        }
    }
}

/// The siblings an every-later and an every-earlier walk reached this
/// update (`Dirty::walked`): a walk that meets one stops there, so the
/// marks of an update are bounded by the children and the changes, not
/// their product (entry 117, action 48).
#[derive(Default)]
pub(crate) struct Walked {
    later: NodeSet,
    earlier: NodeSet,
}

impl Walked {
    /// `node` was inserted, perhaps moved: a walk that meets it goes on.
    /// A guard no test can pin: the insert's own walk already covers
    /// what lies past the new place.
    fn forget(&mut self, node: NodeId) {
        self.later.remove(&node);
        self.earlier.remove(&node);
    }
}

/// The siblings of `element` that `deps` reaches, later then earlier.
fn reached(element: ElementRef<'_>, deps: SiblingDeps, walked: &mut Walked) -> Vec<NodeId> {
    let mut out = if deps.later {
        unwalked(element, ElementRef::next_sibling, &mut walked.later)
    } else {
        siblings(element, ElementRef::next_sibling, deps.next)
    };
    out.extend(if deps.earlier {
        unwalked(element, ElementRef::previous_sibling, &mut walked.earlier)
    } else {
        siblings(
            element,
            ElementRef::previous_sibling,
            usize::from(deps.previous),
        )
    });
    out
}

/// The element siblings of `from` in `step`'s direction up to the
/// first one already in `walked`, each added to it.
///
/// The stop holds by this invariant: every sibling past a node in
/// `walked`, in its direction, is marked this update. The walk that
/// added the node went on to the end, or to a walked node past which
/// the same holds. A child inserted since, a moved one included, is
/// marked with its subtree and runs its own walk from its new place
/// under the same sheets (and is dropped from `walked`,
/// `Walked::forget`). A removal leaves no unmarked sibling.
fn unwalked<'a>(
    from: ElementRef<'a>,
    step: fn(&ElementRef<'a>) -> Option<ElementRef<'a>>,
    walked: &mut NodeSet,
) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut next = step(&from);
    while let Some(sibling) = next.filter(|s| walked.insert(s.id)) {
        out.push(sibling.id);
        next = step(&sibling);
    }
    out
}

/// Up to `limit` element siblings of `from` in `step`'s direction.
fn siblings<'a>(
    from: ElementRef<'a>,
    step: fn(&ElementRef<'a>) -> Option<ElementRef<'a>>,
    limit: usize,
) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut next = step(&from);
    while let Some(sibling) = next.filter(|_| out.len() < limit) {
        out.push(sibling.id);
        next = step(&sibling);
    }
    out
}

#[cfg(test)]
#[path = "restyle_children_tests.rs"]
mod tests;
