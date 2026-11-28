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
    /// A walk over every later (or earlier) sibling stops at one an
    /// earlier such walk of this update reached: every sibling past it
    /// is marked already, or was inserted since and marked itself, so
    /// the marks of an update are bounded by the children and the
    /// changes, not their product (entry 117, action 48).
    fn mark_siblings(&mut self, child: NodeId, deps: SiblingDeps) {
        let element = ElementRef::new(&self.arena, child);
        let walked = &mut self.dirty.walked;
        let mut reached = if deps.later {
            unwalked(element, ElementRef::next_sibling, &mut walked.later)
        } else {
            siblings(element, ElementRef::next_sibling, deps.next)
        };
        reached.extend(if deps.earlier {
            unwalked(element, ElementRef::previous_sibling, &mut walked.earlier)
        } else {
            siblings(
                element,
                ElementRef::previous_sibling,
                usize::from(deps.previous),
            )
        });
        for node in reached {
            if deps.descendants {
                self.dirty.subtree(node);
            } else {
                self.dirty.element(node);
            }
        }
    }
}

/// The element siblings of `from` in `step`'s direction up to the
/// first one already in `walked`, each added to it.
///
/// The stop holds by this invariant: every sibling past a node in
/// `walked`, in its direction, is marked this update. The walk that
/// added the node went on to the end. A child inserted since, a moved
/// one included, is marked with its subtree and runs its own walk from
/// its new place under the same sheets, and is dropped from `walked`
/// on insert (`Walked::forget`). A removal leaves no unmarked sibling.
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
