//! `SceneRenderer`: the scene on a `gpu-surface`.

use crate::camera::{CameraUniform, view_projection};
use crate::draw::{MeshNode, Pass, mesh_nodes, record};
use crate::geometry::GpuGeometry;
use crate::pipeline::Pipeline;
use crate::slots::Slots;
use crate::{FrameError, depth};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::{SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};
use vapors_scene3d::{Meshes, NodeId, Scene};

/// Draws a [`Scene`] shared with the application, with the geometry
/// given to its [`Meshes`]. See the crate docs for what a render does.
pub struct SceneRenderer {
    scene: Rc<RefCell<Scene>>,
    meshes: Rc<RefCell<Meshes>>,
    mounted: Option<Mounted>,
    depth: Option<wgpu::TextureView>,
}

/// What `mount` builds.
struct Mounted {
    pipeline: Pipeline,
    camera: CameraUniform,
    slots: Slots,
    geometry: GpuGeometry,
}

impl SceneRenderer {
    /// A renderer for `scene`, drawing geometry from `meshes`.
    #[must_use]
    pub fn new(scene: Rc<RefCell<Scene>>, meshes: Rc<RefCell<Meshes>>) -> Self {
        Self {
            scene,
            meshes,
            mounted: None,
            depth: None,
        }
    }
}

impl SurfaceRenderer for SceneRenderer {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        let pipeline = Pipeline::new(device);
        self.mounted = Some(Mounted {
            camera: CameraUniform::new(device, &pipeline),
            slots: Slots::new(device, &pipeline),
            geometry: GpuGeometry::default(),
            pipeline,
        });
        Ok(())
    }

    fn resize(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<(), SurfaceError> {
        self.depth = Some(depth::texture(device, size));
        Ok(())
    }

    fn render(&mut self, mut frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let (Some(mounted), Some(depth)) = (self.mounted.as_mut(), self.depth.as_ref()) else {
            return Err(FrameError::NotReady.into());
        };
        let meshes = self.meshes.borrow();
        let mut scene = self.scene.borrow_mut();
        scene.update();
        let nodes = mesh_nodes(&scene, &meshes)?;
        let find = |id| meshes.get(id);
        mounted.geometry.upload_missing(&frame, &nodes, find)?;
        mounted
            .camera
            .write(&frame, view_projection(&scene, frame.size)?);
        mounted.write_slots(&frame, &scene, &nodes)?;
        Ok(mounted.draw(&mut frame, depth, &nodes)?)
    }
}

impl Mounted {
    /// Writes the slot of each mesh node that does not hold the node's
    /// current world version: new, moved, or in a grown buffer.
    fn write_slots(
        &mut self,
        frame: &SurfaceFrame<'_>,
        scene: &Scene,
        nodes: &[MeshNode],
    ) -> Result<(), FrameError> {
        let ids: Vec<NodeId> = nodes.iter().map(|mesh| mesh.node).collect();
        self.slots.assign(frame.device, &self.pipeline, &ids)?;
        for mesh in nodes {
            let version = scene.world_version(mesh.node)?;
            if !self.slots.holds(mesh.node, version) {
                let world = scene.world(mesh.node)?;
                self.slots
                    .write(frame, mesh.node, version, world, mesh.colour);
            }
        }
        Ok(())
    }

    fn draw(
        &self,
        frame: &mut SurfaceFrame<'_>,
        depth: &wgpu::TextureView,
        nodes: &[MeshNode],
    ) -> Result<(), FrameError> {
        let draws = nodes
            .iter()
            .map(|mesh| Ok((self.offset(mesh.node), self.geometry.get(mesh)?)))
            .collect::<Result<Vec<_>, FrameError>>()?;
        let pass = Pass {
            pipeline: &self.pipeline.render,
            camera: self.camera.bind_group(),
            slots: self.slots.bind_group(),
            colour: frame.view,
            depth,
            work: frame.work,
        };
        record(frame.encoder, &pass, &draws);
        Ok(())
    }

    fn offset(&self, node: NodeId) -> u32 {
        self.slots
            .offset(node)
            .expect("every mesh node is assigned a slot before it is written or drawn")
    }
}
