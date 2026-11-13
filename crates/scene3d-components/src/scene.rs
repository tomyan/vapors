//! `<Scene scene={..} surface={..} camera={..}>`: the root of a scene
//! drawn on a `gpu-surface`.

use crate::children::render_children;
use crate::component::node_component;
use crate::node::{Mounted, anchor};
use crate::own_keys;
use crate::placement::{Placement, place};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::SurfaceHandle;
use vapors_dom::{DynDom, NodeId as ElementId, Root, Snippet, StyleKey};
use vapors_scene3d::{CameraHandle, Scene as Store};
use vapors_signals::Owner;

/// What a parent passes to [`Scene::mount`].
#[derive(Default)]
pub struct SceneProps {
    /// The scene the surface's `SceneRenderer` draws; required.
    pub scene: Option<Rc<RefCell<Store>>>,
    /// The handle `ElementsApp::mount` returned for that renderer; each
    /// change to a node invalidates it. Required.
    pub surface: Option<SurfaceHandle>,
    /// The active camera, by the handle a `<Camera>` holds. `None`, or a
    /// handle no camera holds, is refused by the frame.
    pub camera: Option<CameraHandle>,
    /// Its nodes.
    pub children: Snippet,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

/// A scene's root group, with its children mounted under it.
pub struct Scene {
    mounted: Mounted,
}

impl Scene {
    /// Mounts the scene's anchor in `parent` before `before`, names the
    /// active camera and mounts the children under the root.
    ///
    /// # Panics
    ///
    /// When `scene` or `surface` is `None`.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "every component takes its props struct by value, so parents mount them alike"
    )]
    pub fn mount_before(
        dom: &impl DynDom,
        parent: ElementId,
        before: Option<ElementId>,
        props: SceneProps,
    ) -> Self {
        let dom = &dom.dyn_dom();
        let (Some(scene), Some(surface)) = (props.scene.clone(), props.surface.clone()) else {
            panic!(
                "<Scene> needs scene={{Some(..)}} and surface={{Some(..)}}: pass the scene its SceneRenderer draws and the handle ElementsApp::mount returned"
            )
        };
        scene.borrow_mut().set_camera(props.camera);
        let node = scene.borrow().root();
        let placement = Placement {
            scene,
            node,
            surface,
        };
        let owner = Owner::new();
        let key = props.anchor.unwrap_or(&own_keys::SCENE);
        let anchor = owner.with(|| {
            let anchor = anchor(dom, key, (parent, before));
            place(&mut *dom.borrow_mut(), anchor, placement.clone());
            placement.invalidate("Scene");
            render_children(dom, anchor, &props.children, "Scene");
            anchor
        });
        let mounted = Mounted {
            owner,
            roots: vec![Root::Node(anchor)],
            anchor,
            placement,
        };
        Self { mounted }
    }
}

node_component!(Scene, SceneProps);
