//! `<Scene scene={..} surface={..} camera={..}>`: the root of a scene
//! drawn on a `gpu-surface`.

use crate::children::render_children;
use crate::component::node_component;
use crate::hits::{SceneHits, hit_source};
use crate::node::{Mounted, anchor};
use crate::own_keys;
use crate::placement::{Placement, place};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::SurfaceHandle;
use vapors_dom::{Dom, DynDom, NodeId as ElementId, Root, Snippet, StyleKey};
use vapors_scene3d::{Ambient, CameraHandle, Scene as Store};
use vapors_signals::Owner;

/// What a parent passes to [`Scene::mount`].
pub struct SceneProps {
    /// The scene the surface's `SceneRenderer` draws; required.
    pub scene: Option<Rc<RefCell<Store>>>,
    /// The handle `ElementsApp::mount` returned for that renderer; each
    /// change to a node invalidates it. Required.
    pub surface: Option<SurfaceHandle>,
    /// The active camera, by the handle a `<Camera>` holds. `None`, or a
    /// handle no camera holds, is refused by the frame.
    pub camera: Option<CameraHandle>,
    /// The scene's ambient light; none by default.
    pub ambient: Ambient,
    /// Its nodes.
    pub children: Snippet,
    /// The anchor's key, which the compiler passes for a tag carrying
    /// `class`, `class:` or `style`; `None` takes the component's own.
    pub anchor: Option<&'static StyleKey>,
}

impl Default for SceneProps {
    fn default() -> Self {
        Self {
            scene: None,
            surface: None,
            camera: None,
            ambient: Ambient::NONE,
            children: Snippet::default(),
            anchor: None,
        }
    }
}

impl SceneProps {
    /// Its anchor's tag, which the compiler puts in the key it passes.
    pub const ANCHOR_TAG: &'static str = "scene";
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
        scene.borrow_mut().set_ambient(props.ambient);
        let owner = Owner::new();
        let key = own_keys::anchor_key("Scene", props.anchor, &own_keys::SCENE);
        let (anchor, placement) = owner.with(|| {
            let anchor = anchor(dom, key, (parent, before));
            let placement = root_placement(dom, anchor, scene, surface);
            place(&mut *dom.borrow_mut(), anchor, placement.clone());
            placement.invalidate("Scene");
            render_children(dom, anchor, &props.children, "Scene");
            (anchor, placement)
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

/// The root node's placement, the scene's hits answered on `surface`
/// from now on: watched before any child mounts, so each child's
/// `hits_moved` finds the watch.
fn root_placement(
    dom: &Rc<RefCell<dyn Dom>>,
    anchor: ElementId,
    scene: Rc<RefCell<Store>>,
    surface: SurfaceHandle,
) -> Placement {
    let hits = SceneHits::new(dom, anchor);
    let source = hit_source(Rc::clone(&scene), Rc::clone(&hits));
    dom.borrow_mut().watch_hits(anchor, surface.name(), source);
    let node = scene.borrow().root();
    Placement {
        scene,
        node,
        surface,
        hits,
    }
}

node_component!(Scene, SceneProps);
