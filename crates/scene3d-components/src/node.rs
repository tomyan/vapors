//! What every node component does: an inert anchor element, a scene
//! node under the parent's, a transform effect, and its children.

use crate::children::render_children;
use crate::placement::{Placement, place, placement_of};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use vapors_dom::{Dom, NodeId as ElementId, Root, Snippet};
use vapors_scene3d::{NodeId, NodeKind, StoreError, Transform};
use vapors_signals::{Effect, Owner, Signal, on_cleanup};

/// The anchor's inline style: no box, so no layout, no paint and no
/// place in the accessibility tree.
pub(crate) const ANCHOR_STYLE: &str = "display: none";

/// A mounted node component: its owner, its one root (the anchor) and
/// the scene node it holds.
pub(crate) struct Mounted {
    pub(crate) owner: Owner,
    pub(crate) roots: Vec<Root>,
    pub(crate) placement: Placement,
}

/// The anchor element, in `parent` before `before`.
pub(crate) fn anchor(
    dom: &Rc<RefCell<dyn Dom>>,
    parent: ElementId,
    before: Option<ElementId>,
) -> ElementId {
    let mut dom = dom.borrow_mut();
    let anchor = dom.create_element("div");
    dom.set_attribute(anchor, "style", ANCHOR_STYLE);
    dom.insert_before(parent, anchor, before);
    anchor
}

/// Mounts a node of `kind` under the node whose anchor is `parent`,
/// moved by `transform` from now on, with `children` in its anchor.
pub(crate) fn mount_node(
    dom: &Rc<RefCell<dyn Dom>>,
    (parent, before): (ElementId, Option<ElementId>),
    (component, kind): (&'static str, NodeKind),
    transform: Signal<Transform>,
    children: &Snippet,
) -> Mounted {
    let above = placement_of(&*dom.borrow(), parent, component);
    let owner = Owner::new();
    owner.with(|| {
        let anchor = anchor(dom, parent, before);
        let node = above.add(component, kind, transform.get_untracked());
        let placement = Placement { node, ..above };
        place(&mut *dom.borrow_mut(), anchor, placement.clone());
        follow_transform(component, placement.clone(), transform);
        remove_on_cleanup(component, placement.clone());
        render_children(dom, anchor, children, component);
        Mounted {
            owner,
            roots: vec![Root::Node(anchor)],
            placement,
        }
    })
}

/// Writes each value of `transform` to the node and invalidates its
/// surface: the node's world matrix, and its uniform alone, change.
fn follow_transform(component: &'static str, placement: Placement, transform: Signal<Transform>) {
    Effect::new(move || {
        let local = transform.get();
        let written = placement
            .scene
            .borrow_mut()
            .set_transform(placement.node, local);
        refuse(component, written);
        placement.invalidate(component);
    });
}

/// Removes the node, with its subtree, when the component's owner is
/// disposed (an `{if}` hiding it, a `{for}` dropping its row, an
/// unmount), and invalidates the surface. An owner's cleanups run
/// before its children's, so a child whose ancestor's removal took it
/// already finds it stale, and that alone is let through.
fn remove_on_cleanup(component: &'static str, placement: Placement) {
    on_cleanup(move || {
        let removed = placement.scene.borrow_mut().remove(placement.node);
        match removed {
            Ok(_) | Err(StoreError::Stale(_)) => {}
            Err(error) => panic!("<{component}>: {error}"),
        }
        placement.invalidate(component);
    });
}

impl Placement {
    /// Adds a node of `kind` under this one.
    fn add(&self, component: &str, kind: NodeKind, local: Transform) -> NodeId {
        let added = self.scene.borrow_mut().add(self.node, kind, local);
        refuse(component, added)
    }

    /// Renders the scene's surface on the next frame.
    ///
    /// # Panics
    ///
    /// When the surface's renderer was unmounted or its loop has ended.
    pub(crate) fn invalidate(&self, component: &str) {
        refuse(component, self.surface.invalidate());
    }
}

/// `result`'s value; its error stops the work, naming `component`.
pub(crate) fn refuse<T, E: Display>(component: &str, result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("<{component}>: {error}"))
}
