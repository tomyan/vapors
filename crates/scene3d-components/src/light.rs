//! `<Light kind={..} transform={..}>`: a directional or point light.
//! `kind` is the scene's closed [`Light`](vapors_scene3d::Light) enum,
//! fixed for the node's life; a directional light shines along its
//! node's -Z, a point light from its position. At most
//! [`MAX_LIGHTS`](vapors_scene3d::MAX_LIGHTS) in a scene: a ninth
//! refuses at mount.

use crate::component::node_component;
use crate::node::{Mounted, mount_node};
use crate::own_keys;
use vapors_dom::{DynDom, NodeId as ElementId, Snippet, StyleKey};
use vapors_scene3d::{Light as Kind, NodeKind, Transform};
use vapors_signals::{ReadSignal, Signal};

/// What a parent passes to [`Light::mount`].
pub struct LightProps {
    /// What it shines; required.
    pub kind: Option<Kind>,
    /// Where it sits in its parent; a change writes the lights uniform
    /// alone.
    pub transform: ReadSignal<Transform>,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl LightProps {
    /// Its anchor's tag, which the compiler puts in the key it passes.
    pub const ANCHOR_TAG: &'static str = "light";
}

impl Default for LightProps {
    fn default() -> Self {
        Self {
            kind: None,
            transform: Signal::new(Transform::IDENTITY).read_only(),
            anchor: None,
        }
    }
}

/// A light node.
pub struct Light {
    mounted: Mounted,
}

impl Light {
    /// Mounts the light in `parent`, a scene node's anchor, before
    /// `before`.
    ///
    /// # Panics
    ///
    /// When `parent` is not a scene node's anchor, `kind` is `None` or
    /// refused (`Light::check`), or the scene already holds eight lights.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "every component takes its props struct by value, so parents mount them alike"
    )]
    pub fn mount_before(
        dom: &impl DynDom,
        parent: ElementId,
        before: Option<ElementId>,
        props: LightProps,
    ) -> Self {
        let dom = &dom.dyn_dom();
        let Some(kind) = props.kind else {
            panic!("<Light> needs kind={{Some(..)}}: pass a Light::Directional or Light::Point")
        };
        let key = own_keys::anchor_key("Light", props.anchor, &own_keys::LIGHT);
        let node = ("Light", key, NodeKind::Light(kind));
        let mounted = mount_node(
            dom,
            (parent, before),
            node,
            props.transform,
            &Snippet::empty(),
        );
        Self { mounted }
    }
}

node_component!(Light, LightProps);
