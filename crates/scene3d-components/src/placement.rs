//! Where a node component's children go. Each `<Scene>`, `<Group>`,
//! `<Mesh>` and `<Camera>` has one anchor element; the anchor carries a
//! key into this thread's table of placements, and a component mounted
//! in an anchor (directly, or by an `{if}` or `{for}` in it, at any
//! later time) reads its parent node there.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use vapors_app::SurfaceHandle;
use vapors_dom::{Dom, NodeId as ElementId};
use vapors_scene3d::{NodeId, Scene};
use vapors_signals::on_cleanup;

/// The anchor attribute holding the key of its placement.
pub(crate) const KEY_ATTRIBUTE: &str = "data-scene3d-node";

/// A scene node and what a change to it must reach.
#[derive(Clone)]
pub(crate) struct Placement {
    pub(crate) scene: Rc<RefCell<Scene>>,
    pub(crate) node: NodeId,
    pub(crate) surface: SurfaceHandle,
}

thread_local! {
    static PLACED: RefCell<HashMap<u64, Placement>> = RefCell::new(HashMap::new());
    static NEXT_KEY: Cell<u64> = const { Cell::new(0) };
}

/// Marks `anchor` as holding `placement`'s node, until the current
/// owner is disposed. Keys are never reused, so a stale key finds
/// nothing rather than another node.
pub(crate) fn place<D: Dom + ?Sized>(dom: &mut D, anchor: ElementId, placement: Placement) {
    let key = NEXT_KEY.with(|next| next.replace(next.get() + 1));
    dom.set_attribute(anchor, KEY_ATTRIBUTE, &key.to_string());
    PLACED.with(|placed| placed.borrow_mut().insert(key, placement));
    on_cleanup(move || {
        PLACED.with(|placed| placed.borrow_mut().remove(&key));
    });
}

/// The placement of the node whose anchor is `parent`.
///
/// # Panics
///
/// When `parent` is not a mounted node component's anchor: `component`
/// is outside any `<Scene>`.
pub(crate) fn placement_of<D: Dom + ?Sized>(
    dom: &D,

    parent: ElementId,
    component: &str,
) -> Placement {
    let key = dom.current_attribute(parent, KEY_ATTRIBUTE);
    let found = key
        .and_then(|key| key.parse::<u64>().ok())
        .and_then(|key| PLACED.with(|placed| placed.borrow().get(&key).cloned()));
    found.unwrap_or_else(|| {
        panic!(
            "<{component}> is placed in an element that is not a scene node: put it directly inside a <Scene>, <Group> or a block ({{if}}, {{for}}) inside one"
        )
    })
}
