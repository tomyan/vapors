//! Which scroll containers a scroll over an element tries (F2c S9).

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::layout::{Content, LayoutBox};

/// Which scroll container a scroll over an element moves first, and
/// which one each container passes a scroll it cannot take to: by
/// containing block, as clips pass (`crate::chain`).
#[derive(Debug, Default)]
pub(crate) struct Chains {
    first: HashMap<NodeId, NodeId>,
    outer: HashMap<NodeId, NodeId>,
}

impl Chains {
    /// The chains of the laid-out `boxes`, where `scrolls` says which
    /// box is a scroll container.
    pub(crate) fn of(boxes: &[LayoutBox], scrolls: &dyn Fn(usize) -> bool) -> Self {
        let node = |index: usize| match boxes[index].content {
            Content::Element(node) => Some(node),
            Content::Text(_) => None,
        };
        let around = crate::chain::by_containing_block(boxes, None, &|index, own| {
            if scrolls(index) { node(index) } else { own }
        });
        let mut chains = Self::default();
        for (index, own) in around.into_iter().enumerate() {
            if let Some(element) = node(index) {
                chains.link(element, scrolls(index), own);
            }
        }
        chains
    }

    /// Records where a scroll over `element` goes first, and, when it is
    /// a scroll container, which one it passes a scroll out to: `own`,
    /// the nearest around it.
    fn link(&mut self, element: NodeId, scrolls: bool, own: Option<NodeId>) {
        let first = if scrolls { Some(element) } else { own };
        if let Some(first) = first {
            self.first.insert(element, first);
        }
        if let (true, Some(outer)) = (scrolls, own) {
            self.outer.insert(element, outer);
        }
    }

    /// The scroll containers a scroll over `element` tries, innermost
    /// first.
    pub(crate) fn from(&self, element: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        let mut next = self.first.get(&element).copied();
        while let Some(container) = next {
            found.push(container);
            next = self.outer.get(&container).copied();
        }
        found
    }
}
