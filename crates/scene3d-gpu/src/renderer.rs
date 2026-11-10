//! `SceneRenderer`: the scene on a `gpu-surface`.

use crate::camera::{CameraUniform, view_projection};
use crate::draw::{MeshNode, Pass, mesh_nodes, record, uploaded};
use crate::geometry::GpuGeometry;
use crate::pipeline::Pipeline;
use crate::slots::Slots;
use crate::{FrameError, depth};
use std::cell::RefCell;
use std::collections::HashSet;
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
        let mut meshes = self.meshes.borrow_mut();
        mounted.geometry.upload_queued(frame.device, &mut meshes);
        let mut scene = self.scene.borrow_mut();
        let changed: HashSet<NodeId> = scene.update().into_iter().collect();
        let nodes = mesh_nodes(&scene, &meshes)?;
        mounted
            .camera
            .write(frame.queue, view_projection(&scene, frame.size)?);
        mounted.write_slots(&frame, &scene, &nodes, &changed)?;
        mounted.draw(&mut frame, depth, &nodes);
        Ok(())
    }
}

impl Mounted {
    /// Writes the slot of each mesh node that is new, recomputed by this
    /// frame's update, or in a reallocated buffer.
    fn write_slots(
        &mut self,
        frame: &SurfaceFrame<'_>,
        scene: &Scene,
        nodes: &[MeshNode],
        changed: &HashSet<NodeId>,
    ) -> Result<(), FrameError> {
        let ids: Vec<NodeId> = nodes.iter().map(|mesh| mesh.node).collect();
        let unwritten: HashSet<NodeId> = self
            .slots
            .assign(frame.device, &self.pipeline, &ids)
            .into_iter()
            .collect();
        for mesh in nodes {
            if changed.contains(&mesh.node) || unwritten.contains(&mesh.node) {
                let offset = self.offset(mesh.node);
                self.slots
                    .write(frame.queue, offset, scene.world(mesh.node)?, mesh.colour);
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut SurfaceFrame<'_>, depth: &wgpu::TextureView, nodes: &[MeshNode]) {
        let draws: Vec<_> = nodes
            .iter()
            .map(|mesh| {
                (
                    self.offset(mesh.node),
                    uploaded(&self.geometry, mesh.geometry),
                )
            })
            .collect();
        let pass = Pass {
            pipeline: &self.pipeline.render,
            camera: self.camera.bind_group(),
            slots: self.slots.bind_group(),
            colour: frame.view,
            depth,
        };
        record(frame.encoder, &pass, &draws);
    }

    fn offset(&self, node: NodeId) -> u32 {
        self.slots
            .offset(node)
            .expect("every mesh node is assigned a slot before it is written or drawn")
    }
}
