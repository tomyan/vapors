//! `<Camera handle={..} projection={..} transform={..}>`: a camera the
//! `<Scene>` names by handle. Its lens is a signal, so a zoom is a lens
//! change; its node stays rigid (`Scene::view_projection` refuses a
//! camera that scales or mirrors).

use crate::component::node_component;
use crate::node::{Mounted, mount_node, refuse};
use crate::own_keys;
use vapors_dom::{DynDom, NodeId as ElementId, Snippet, StyleKey};
use vapors_scene3d::{CameraHandle, NodeKind, Projection, Transform};
use vapors_signals::{Effect, ReadSignal, Signal};

/// What a parent passes to [`Camera::mount`].
pub struct CameraProps {
    /// The name the scene's `camera` prop points at; required.
    pub handle: Option<CameraHandle>,
    /// How it projects; required. A change writes the camera's lens and
    /// draws the surface again.
    pub projection: Option<ReadSignal<Projection>>,
    /// Where it sits in its parent, looking down its own -Z; a change
    /// writes the camera's uniform alone.
    pub transform: ReadSignal<Transform>,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl CameraProps {
    /// Its anchor's tag, which the compiler puts in the key it passes.
    pub const ANCHOR_TAG: &'static str = "camera";
}

impl Default for CameraProps {
    fn default() -> Self {
        Self {
            handle: None,
            projection: None,
            transform: Signal::new(Transform::IDENTITY).read_only(),
            anchor: None,
        }
    }
}

/// A camera node.
pub struct Camera {
    mounted: Mounted,
}

impl Camera {
    /// Mounts the camera in `parent`, a scene node's anchor, before
    /// `before`.
    ///
    /// # Panics
    ///
    /// When `parent` is not a scene node's anchor, or `handle` or
    /// `projection` is `None`.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "every component takes its props struct by value, so parents mount them alike"
    )]
    pub fn mount_before(
        dom: &impl DynDom,
        parent: ElementId,
        before: Option<ElementId>,
        props: CameraProps,
    ) -> Self {
        let dom = &dom.dyn_dom();
        let (Some(handle), Some(projection)) = (props.handle, props.projection) else {
            panic!("<Camera> needs handle={{Some(..)}} and projection={{Some(..)}}: pass both")
        };
        let key = own_keys::anchor_key("Camera", props.anchor, &own_keys::CAMERA);
        let lens = projection.get_untracked();
        let kind = NodeKind::Camera {
            handle,
            projection: lens,
        };
        let mounted = mount_node(
            dom,
            (parent, before),
            ("Camera", key, kind),
            props.transform,
            &Snippet::empty(),
        );
        let placement = mounted.placement.clone();
        mounted
            .owner
            .with(|| follow_projection(placement, projection));
        Self { mounted }
    }
}

node_component!(Camera, CameraProps);

/// Writes each later value of `projection` to the camera and draws its
/// surface again.
fn follow_projection(placement: crate::placement::Placement, projection: ReadSignal<Projection>) {
    Effect::new(move || {
        let lens = projection.get();
        let written = placement
            .scene
            .borrow_mut()
            .set_projection(placement.node, lens);
        if refuse("Camera", written) {
            placement.invalidate("Camera");
        }
    });
}
