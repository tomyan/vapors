//! Computed styles kept by slot, each an `Rc` so elements can share
//! one (`share`). A slot also holds the id of the node its style is
//! for: after a node is removed, a new node can take its slot, and
//! must not read the old node's style as its own. Every allocation
//! counts itself in and out of `live`, for `styles_live`.

use crate::slot_list;
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

use vapors_css::cascade::ComputedStyle;
use vapors_dom::NodeId;

use crate::arena::index_of;

/// One computed style allocation, counted while it lives.
pub(crate) struct Counted {
    style: ComputedStyle,
    live: Rc<Cell<usize>>,
}

impl Deref for Counted {
    type Target = ComputedStyle;
    fn deref(&self) -> &ComputedStyle {
        &self.style
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.live.set(self.live.get() - 1);
    }
}

/// Computed styles by slot; empty for text nodes and removed nodes.
#[derive(Default)]
pub(crate) struct Styles {
    slots: Vec<Option<(NodeId, Rc<Counted>)>>,
    live: Rc<Cell<usize>>,
}

impl Styles {
    /// Each per-slot list's length and capacity, for the growth test.
    #[cfg(test)]
    pub(crate) fn slot_lists(&self) -> Vec<(&'static str, usize, usize)> {
        vec![("styles.slots", self.slots.len(), self.slots.capacity())]
    }
    /// A new allocation of `style`, counted in `live`.
    pub(crate) fn alloc(&self, style: ComputedStyle) -> Rc<Counted> {
        self.live.set(self.live.get() + 1);
        Rc::new(Counted {
            style,
            live: Rc::clone(&self.live),
        })
    }

    /// How many allocations made here are alive, shared or not.
    pub(crate) fn live(&self) -> usize {
        self.live.get()
    }

    /// The allocation kept for `node`, if any: none when its slot holds
    /// another node's.
    pub(crate) fn rc(&self, node: NodeId) -> Option<&Rc<Counted>> {
        match self.slots.get(index_of(node) as usize)? {
            Some((owner, style)) if *owner == node => Some(style),
            _ => None,
        }
    }

    /// The style kept for `node`, if any.
    pub(crate) fn get(&self, node: NodeId) -> Option<&ComputedStyle> {
        self.rc(node).map(|style| &***style)
    }

    /// Keeps `style` for `node`.
    pub(crate) fn set(&mut self, node: NodeId, style: Rc<Counted>) {
        let slot = index_of(node) as usize;
        if self.slots.len() <= slot {
            slot_list::resize_with(&mut self.slots, slot + 1, || None);
        }
        self.slots[slot] = Some((node, style));
    }

    /// Drops what is kept for `node`, if its slot is still its.
    pub(crate) fn remove(&mut self, node: NodeId) {
        if self.rc(node).is_some() {
            self.slots[index_of(node) as usize] = None;
        }
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

    fn computed(arena: &Arena, node: NodeId) -> ComputedStyle {
        Cascade::new()
            .compute(&ElementRef::new(arena, node), None, &Environment::default())
            .expect("computes")
    }

    #[test]
    fn a_node_in_a_removed_nodes_slot_has_no_style_until_it_is_styled() {
        // Given: a style kept for a node, which is then freed, and a new
        // node that takes its slot
        let mut arena = Arena::default();
        let old = arena.insert(Node::new(Kind::Element(Element::new("div"))));
        let mut styles = Styles::default();
        let style = styles.alloc(computed(&arena, old));
        styles.set(old, style);
        arena.free(old);
        let new = arena.insert(Node::new(Kind::Element(Element::new("div"))));

        // When
        let found = styles.get(new);

        // Then
        assert_eq!(index_of(new), index_of(old));
        assert!(found.is_none());
    }

    #[test]
    fn live_counts_allocations_shared_or_not_until_the_last_holder_drops() {
        // Given
        let mut arena = Arena::default();
        let a = arena.insert(Node::new(Kind::Element(Element::new("div"))));
        let b = arena.insert(Node::new(Kind::Element(Element::new("div"))));
        let mut styles = Styles::default();
        let shared = styles.alloc(computed(&arena, a));

        // When: one allocation held by two slots, then one slot cleared
        styles.set(a, Rc::clone(&shared));
        styles.set(b, shared);
        let while_shared = styles.live();
        styles.remove(a);
        let after_one = styles.live();
        styles.remove(b);

        // Then
        assert_eq!((while_shared, after_one, styles.live()), (1, 1, 0));
    }
}
