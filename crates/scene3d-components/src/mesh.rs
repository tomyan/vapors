//! `<Mesh geometry={..} transform={..}>`: geometry drawn in one colour,
//! its anchor's computed `color`, and shown while its `visibility` is
//! `visible` (docs/plans/css-for-3d.md C3).

use crate::component::node_component;
use crate::node::{Mounted, mount_node};
use crate::own_keys;
use crate::placement::Placement;
use vapors_dom::{Appearance, DynDom, NodeId as ElementId, Snippet, StyleKey};
use vapors_scene3d::{Colour, GeometryHandle, NodeKind, Transform};
use vapors_signals::Signal;

/// What a parent passes to [`Mesh::mount`].
pub struct MeshProps {
    /// The triangles, from the renderer's `Meshes`; required.
    pub geometry: Option<GeometryHandle>,
    /// Where the mesh sits in its parent; a change writes its uniform
    /// alone.
    pub transform: Signal<Transform>,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl MeshProps {
    /// Its anchor's tag, which the compiler puts in the key it passes.
    pub const ANCHOR_TAG: &'static str = "mesh";
}

impl Default for MeshProps {
    fn default() -> Self {
        Self {
            geometry: None,
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
        let key = own_keys::anchor_key("Mesh", props.anchor, &own_keys::MESH);
        // black until the first update gives the anchor's colour
        let colour = Colour::BLACK;
        let kind = ("Mesh", key, NodeKind::Mesh { geometry, colour });
        let mounted = mount_node(
            dom,
            (parent, before),
            kind,
            props.transform,
            &Snippet::empty(),
        );
        let sink = appearance_sink(mounted.placement.clone());
        dom.borrow_mut().watch_appearance(mounted.anchor, sink);
        Self { mounted }
    }
}

/// Writes each appearance the anchor reports to the mesh, asking its
/// surface for a frame first: when that is refused the mesh is left as
/// drawn, so the anchor's next report (vapors-elements reports a refused
/// appearance again) writes it and asks again, rather than finding the
/// mesh already changed and the frame never asked for. vapors-elements
/// reports only a changed appearance, so each report is a frame's worth.
fn appearance_sink(placement: Placement) -> vapors_dom::AppearanceSink {
    Box::new(
        move |Appearance {
                  rgb: [r, g, b],
                  visible,
              }| {
            let failed = |error: &dyn std::fmt::Display| format!("<Mesh>: {error}");
            let colour = Colour::srgb(r, g, b).map_err(|e| failed(&e))?;
            placement.surface.invalidate().map_err(|e| failed(&e))?;
            let mut scene = placement.scene.borrow_mut();
            let written = scene.set_appearance(placement.node, colour, visible);
            written.map(drop).map_err(|e| failed(&e))
        },
    )
}

node_component!(Mesh, MeshProps);
