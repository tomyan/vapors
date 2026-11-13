//! `<Camera handle={..} projection={..} transform={..}>`: a camera the
//! `<Scene>` names by handle.

use crate::component::node_component;
use crate::node::{Mounted, mount_node};
use crate::own_keys;
use vapors_dom::{DynDom, NodeId as ElementId, Snippet, StyleKey};
use vapors_scene3d::{CameraHandle, NodeKind, Projection, Transform};
use vapors_signals::Signal;

/// What a parent passes to [`Camera::mount`].
pub struct CameraProps {
    /// The name the scene's `camera` prop points at; required.
    pub handle: Option<CameraHandle>,
    /// How it projects; required.
    pub projection: Option<Projection>,
    /// Where it sits in its parent, looking down its own -Z; a change
    /// writes the camera's uniform alone.
    pub transform: Signal<Transform>,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl Default for CameraProps {
    fn default() -> Self {
        Self {
            handle: None,
            projection: None,
            transform: Signal::new(Transform::IDENTITY),
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
        let key = props.anchor.unwrap_or(&own_keys::CAMERA);
        let kind = ("Camera", key, NodeKind::Camera { handle, projection });
        let mounted = mount_node(
            dom,
            (parent, before),
            kind,
            props.transform,
            &Snippet::empty(),
        );
        Self { mounted }
    }
}

node_component!(Camera, CameraProps);
