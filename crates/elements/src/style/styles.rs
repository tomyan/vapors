//! Computed styles kept by slot. A slot also holds the id of the node
//! its style is for: after a node is removed, a new node can take its
//! slot, and must not read the old node's style as its own.

use vapors_css::cascade::ComputedStyle;
use vapors_dom::NodeId;

use crate::arena::index_of;

/// Computed styles by slot; empty for text nodes and removed nodes.
#[derive(Default)]
pub(crate) struct Styles(Vec<Option<(NodeId, ComputedStyle)>>);

impl Styles {
    /// The style kept for `node`, if any: none when its slot holds
    /// another node's.
    pub(crate) fn get(&self, node: NodeId) -> Option<&ComputedStyle> {
        match self.0.get(index_of(node) as usize)? {
            Some((owner, style)) if *owner == node => Some(style),
            _ => None,
        }
    }

    /// Keeps `style` for `node`.
    pub(crate) fn set(&mut self, node: NodeId, style: ComputedStyle) {
        let slot = index_of(node) as usize;
        if self.0.len() <= slot {
            self.0.resize_with(slot + 1, || None);
        }
        self.0[slot] = Some((node, style));
    }

    /// The element's style.
    ///
    /// # Panics
    /// When none is kept for `node`: the caller walked a different tree
    /// from the one styled.
    pub(crate) fn of(&self, node: NodeId) -> &ComputedStyle {
        self.get(node).unwrap_or_else(|| {
            panic!("{node:?} has no computed style; it was not in the styled tree")
        })
    }
}

#[cfg(test)]
mod tests {
    use vapors_css::cascade::{Cascade, Environment};

    use super::*;
    use crate::arena::Arena;
    use crate::css_element::ElementRef;
    use crate::node::{Element, Kind, Node};

    #[test]
    fn a_node_in_a_removed_nodes_slot_has_no_style_until_it_is_styled() {
        // Given: a style kept for a node, which is then freed, and a new
        // node that takes its slot
        let mut arena = Arena::default();
        let old = arena.insert(Node::new(Kind::Element(Element::new("div"))));
        let style = Cascade::new()
            .compute(&ElementRef::new(&arena, old), None, &Environment::default())
            .expect("computes");
        let mut styles = Styles::default();
        styles.set(old, style);
        arena.free(old);
        let new = arena.insert(Node::new(Kind::Element(Element::new("div"))));

        // When
        let found = styles.get(new);

        // Then
        assert_eq!(index_of(new), index_of(old));
        assert!(found.is_none());
    }
}
