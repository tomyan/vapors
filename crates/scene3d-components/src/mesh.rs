//! `<Mesh geometry={..} colour={..} transform={..}>`: geometry drawn in
//! one colour.

use crate::component::node_component;
use crate::node::{Mounted, mount_node, refuse};
use crate::own_keys;
use crate::placement::Placement;
use vapors_dom::{DynDom, NodeId as ElementId, Snippet, StyleKey};
use vapors_scene3d::{Colour, GeometryId, NodeKind, Transform};
use vapors_signals::{Effect, Signal};

/// What a parent passes to [`Mesh::mount`].
pub struct MeshProps {
    /// The triangles, from the renderer's `Meshes`; required.
    pub geometry: Option<GeometryId>,
    /// Drawn unlit; a change writes the mesh's uniform alone.
    pub colour: Signal<Colour>,
    /// Where the mesh sits in its parent; a change writes its uniform
    /// alone.
    pub transform: Signal<Transform>,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl Default for MeshProps {
    fn default() -> Self {
        Self {
            geometry: None,
            colour: Signal::new(Colour::BLACK),
            transform: Signal::new(Transform::IDENTITY),
            anchor: None,
        }
    }
}

/// A mesh node.
pub struct Mesh {
    mounted: Mounted,
}

impl Mesh {
    /// Mounts the mesh in `parent`, a scene node's anchor, before
    /// `before`.
    ///
    /// # Panics
    ///
    /// When `parent` is not a scene node's anchor, or `geometry` is
    /// `None`.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "every component takes its props struct by value, so parents mount them alike"
    )]
    pub fn mount_before(
        dom: &impl DynDom,
        parent: ElementId,
        before: Option<ElementId>,
        props: MeshProps,
    ) -> Self {
        let dom = &dom.dyn_dom();
        let geometry = props.geometry.unwrap_or_else(|| {
            panic!("<Mesh> has no geometry: pass geometry={{Some(id)}} with an id from the renderer's Meshes")
        });
        let colour = props.colour.get_untracked();
        let key = props.anchor.unwrap_or(&own_keys::MESH);
        let kind = ("Mesh", key, NodeKind::Mesh { geometry, colour });
        let mounted = mount_node(
            dom,
            (parent, before),
            kind,
            props.transform,
            &Snippet::empty(),
        );
        mounted
            .owner
            .with(|| follow_colour(mounted.placement.clone(), props.colour));
        Self { mounted }
    }
}

/// Writes each value of `colour` to the mesh and invalidates its surface.
fn follow_colour(placement: Placement, colour: Signal<Colour>) {
    Effect::new(move || {
        let value = colour.get();
        let written = placement
            .scene
            .borrow_mut()
            .set_colour(placement.node, value);
        refuse("Mesh", written);
        placement.invalidate("Mesh");
    });
}

node_component!(Mesh, MeshProps);
