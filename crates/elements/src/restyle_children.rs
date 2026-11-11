//! What a child inserted under or removed from a parent restyles
//! (splice.md (b2)): the inserted subtree, and among the siblings only
//! those the sheets' sibling combinators and positional pseudo-classes
//! can reach from the change (`SiblingDeps`, kept per sheet added).

use vapors_css::selector::Element as _;
use vapors_dom::NodeId;

use crate::css_element::ElementRef;
use crate::document::Document;
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
        let next = if deps.later { usize::MAX } else { deps.next };
        let previous = if deps.earlier {
            usize::MAX
        } else {
            usize::from(deps.previous)
        };
        let mut reached = siblings(element, ElementRef::next_sibling, next);
        reached.extend(siblings(element, ElementRef::previous_sibling, previous));
        for node in reached {
            if deps.descendants {
                self.dirty.subtree(node);
            } else {
                self.dirty.element(node);
            }
        }
    }
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
