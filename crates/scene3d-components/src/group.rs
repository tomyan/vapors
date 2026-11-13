//! `<Group transform={..}>`: holds children and draws nothing.

use crate::component::node_component;
use crate::node::{Mounted, mount_node};
use crate::own_keys;
use vapors_dom::{DynDom, NodeId as ElementId, Snippet, StyleKey};
use vapors_scene3d::{NodeKind, Transform};
use vapors_signals::Signal;

/// What a parent passes to [`Group::mount`].
pub struct GroupProps {
    /// Where the group sits in its parent: a change moves it and its
    /// subtree, and writes their uniforms alone.
    pub transform: Signal<Transform>,
    /// Its nodes.
    pub children: Snippet,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl GroupProps {
    /// Its anchor's tag, which the compiler puts in the key it passes.
    pub const ANCHOR_TAG: &'static str = "group";
}

impl Default for GroupProps {
    fn default() -> Self {
        Self {
            transform: Signal::new(Transform::IDENTITY),
            children: Snippet::empty(),
            anchor: None,
        }
    }
}

/// A group node.
pub struct Group {
    mounted: Mounted,
}

impl Group {
    /// Mounts the group in `parent`, a scene node's anchor, before
    /// `before`.
    ///
    /// # Panics
    ///
    /// When `parent` is not a scene node's anchor.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "every component takes its props struct by value, so parents mount them alike"
    )]
    pub fn mount_before(
        dom: &impl DynDom,
        parent: ElementId,
        before: Option<ElementId>,
        props: GroupProps,
    ) -> Self {
        let dom = &dom.dyn_dom();
        let key = own_keys::anchor_key("Group", props.anchor, &own_keys::GROUP);
        let kind = ("Group", key, NodeKind::Group);
        let mounted = mount_node(
            dom,
            (parent, before),
            kind,
            props.transform,
            &props.children,
        );
        Self { mounted }
    }
}

node_component!(Group, GroupProps);
