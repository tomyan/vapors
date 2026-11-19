//! `SceneRenderer`: the scene on a `gpu-surface`.

use crate::camera::{CameraUniform, view_projection};
use crate::depth::{Attachments, attachments};
use crate::draw::{MeshNode, Pass, materials_built, mesh_nodes, record};
use crate::geometry::GpuGeometry;
use crate::lights::LightsUniform;
use crate::log::Tracked;
use crate::pipeline::Pipeline;
use crate::slots::Slots;
use crate::{FrameError, RendererLog, Resource};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::{SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};
use vapors_renderer::Samples;
use vapors_scene3d::{MaterialId, Meshes, NodeId, Scene};

/// Draws a [`Scene`] shared with the application, with the geometry
/// given to its [`Meshes`]. Each render borrows the `Meshes` mutably to
/// release unclaimed geometry, so a borrow held across a frame panics. See the crate docs for what a render does.
pub struct SceneRenderer {
    scene: Rc<RefCell<Scene>>,
    meshes: Rc<RefCell<Meshes>>,
    mounted: Option<Mounted>,
    attachments: Option<Tracked<Attachments>>,
    /// The samples its pass draws at (`docs/plans/flat-scene-f7.md`).
    samples: Samples,
    log: Rc<RendererLog>,
}

/// What `mount` builds.
struct Mounted {
    pipeline: Tracked<Pipeline>,
    camera: Tracked<CameraUniform>,
    lights: Tracked<LightsUniform>,
    slots: Tracked<Slots>,
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
            attachments: None,
            samples: Samples::One,
            log: Rc::default(),
        }
    }

    /// Makes its pass draw at `samples`, resolving into the backing at
    /// 4; before it is mounted.
    #[must_use]
    pub fn with_samples(mut self, samples: Samples) -> Self {
        self.samples = samples;
        self
    }

    /// Its log, which outlives it: whether it has been dropped.
    #[must_use]
    pub fn log(&self) -> Rc<RendererLog> {
        Rc::clone(&self.log)
    }
}

impl Drop for SceneRenderer {
    fn drop(&mut self) {
        self.log.drop_renderer();
    }
}

impl SurfaceRenderer for SceneRenderer {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        let log = &self.log;
        let pipeline = Pipeline::new(device, self.meshes.borrow().materials(), self.samples);
        self.mounted = Some(Mounted {
            camera: Tracked::new(CameraUniform::new(device, &pipeline), log, Resource::Camera),
            lights: Tracked::new(LightsUniform::new(device), log, Resource::Lights),
            slots: Tracked::new(Slots::new(device, &pipeline), log, Resource::Slots),
            geometry: GpuGeometry::new(log),
            pipeline: Tracked::new(pipeline, log, Resource::Pipeline),
        });
        Ok(())
    }

    fn resize(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<(), SurfaceError> {
        let made = attachments(device, size, self.samples);
        self.attachments = Some(Tracked::new(made, &self.log, Resource::Attachments));
        Ok(())
    }

    fn render(&mut self, mut frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let (Some(mounted), Some(attached)) = (self.mounted.as_mut(), self.attachments.as_ref())
        else {
            return Err(FrameError::NotReady.into());
        };
        release_geometry(&self.meshes, &mut mounted.geometry);
        let meshes = self.meshes.borrow();
        let mut scene = self.scene.borrow_mut();
        scene.update();
        let nodes = mesh_nodes(&scene, &meshes)?;
        materials_built(&nodes, mounted.pipeline.materials.len())?;
        mounted.geometry.upload_missing(&frame, &nodes);
        mounted
            .camera
            .write(&frame, view_projection(&scene, frame.size)?);
        mounted.lights.write(&frame, &scene)?;
        mounted.write_slots(&frame, &scene, &nodes)?;
        mounted.draw(&mut frame, attached, &nodes)?;
        self.log.rendered();
        Ok(())
    }
}

/// Releases the pieces no handle claims any more, and drops this
/// renderer's buffers of every piece its `Meshes` no longer holds,
/// whichever renderer's sweep released it.
fn release_geometry(meshes: &RefCell<Meshes>, uploaded: &mut GpuGeometry) {
    meshes.borrow_mut().release_unclaimed();
    let meshes = meshes.borrow();
    uploaded.forget_released(|id| meshes.holds(id));
}

impl Mounted {
    /// Writes the slot of each shown mesh node that does not hold the
    /// node's current world version: new, moved, recoloured, or in a
    /// grown buffer. A hidden node's slot is written once it is shown.
    fn write_slots(
        &mut self,
        frame: &SurfaceFrame<'_>,
        scene: &Scene,
        nodes: &[MeshNode],
    ) -> Result<(), FrameError> {
        let ids: Vec<NodeId> = nodes.iter().map(|mesh| mesh.node).collect();
        self.slots.assign(frame.device, &self.pipeline, &ids)?;
        for mesh in nodes.iter().filter(|mesh| mesh.visible) {
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
        attached: &Attachments,
        nodes: &[MeshNode],
    ) -> Result<(), FrameError> {
        // in tree order: of two fragments at one depth the first drawn wins
        // (depth `Less`), so the material does not reorder them
        let draws = nodes
            .iter()
            .filter(|mesh| mesh.visible)
            .map(|mesh| {
                let material = mesh.material.map(MaterialId::index);
                Ok((material, self.offset(mesh.node), self.geometry.get(mesh)?))
            })
            .collect::<Result<Vec<_>, FrameError>>()?;
        let pass = Pass {
            pipelines: (&self.pipeline.render, &self.pipeline.materials),
            camera: self.camera.bind_group(),
            slots: self.slots.bind_group(),
            colour: (attached.colour.as_ref(), frame.view),
            depth: &attached.depth,
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
