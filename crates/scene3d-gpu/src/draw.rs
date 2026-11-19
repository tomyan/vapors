//! Finding the mesh nodes and recording their draws.

use crate::FrameError;
use crate::geometry::Uploaded;
use crate::lit::LitPipeline;
use vapors_app::WorkCounter;
use vapors_scene3d::{
    Colour, GeometryHandle, GeometryId, MaterialId, Meshes, NodeId, NodeKind, Scene, Shading,
};

/// A mesh node to draw.
pub(crate) struct MeshNode {
    pub(crate) node: NodeId,
    pub(crate) geometry: GeometryId,
    /// The node's claim, through which its triangles are read, as
    /// `pick` reads them: one answer to which triangles a node has.
    pub(crate) handle: GeometryHandle,
    pub(crate) colour: Colour,
    /// Drawn (`Scene::visible`); a hidden mesh keeps its slot.
    pub(crate) visible: bool,
    /// The application material it is drawn with (`Scene::material`),
    /// or none for its colour.
    pub(crate) material: Option<MaterialId>,
    /// Drawn lit (`Shading::Lit`), by the lit pipeline.
    pub(crate) lit: bool,
    /// Drawn with a specular highlight too (`Shading::Glossy`).
    pub(crate) glossy: bool,
}

/// Every mesh node under the root, parents before children, each
/// naming geometry `meshes` issued.
pub(crate) fn mesh_nodes(scene: &Scene, meshes: &Meshes) -> Result<Vec<MeshNode>, FrameError> {
    let mut found = Vec::new();
    let mut stack = vec![scene.root()];
    while let Some(node) = stack.pop() {
        found.extend(mesh_node(node, scene, meshes)?);
        stack.extend(scene.children(node)?.iter().rev());
    }
    Ok(found)
}

/// `node` to draw when it is a mesh naming geometry `meshes` issued.
fn mesh_node(node: NodeId, scene: &Scene, meshes: &Meshes) -> Result<Option<MeshNode>, FrameError> {
    let NodeKind::Mesh { geometry, colour } = scene.kind(node)? else {
        return Ok(None);
    };
    let (handle, colour) = (geometry.clone(), *colour);
    let geometry = handle.id();
    if !meshes.issued(geometry) {
        return Err(FrameError::ForeignGeometry { node, geometry });
    }
    let material = scene.material(node)?;
    if let Some(material) = material.filter(|&material| !meshes.issued_material(material)) {
        return Err(FrameError::ForeignMaterial { node, material });
    }
    let shading = scene.shading(node)?;
    Ok(Some(MeshNode {
        node,
        geometry,
        handle,
        colour,
        visible: scene.visible(node)?,
        material,
        lit: shading != Shading::Unlit,
        glossy: shading == Shading::Glossy,
    }))
}

/// Refuses a node naming a material its `Meshes` was given after the
/// renderer built its `built` material pipelines at mount (the id names
/// the material's label).
pub(crate) fn materials_built(nodes: &[MeshNode], built: usize) -> Result<(), FrameError> {
    let late = nodes.iter().find_map(|mesh| {
        mesh.material
            .filter(|material| material.index() >= built)
            .map(|m| (mesh, m))
    });
    match late {
        Some((mesh, material)) => Err(FrameError::MaterialMissing {
            node: mesh.node,
            material,
            built,
        }),
        None => Ok(()),
    }
}

/// What one pass draws with.
pub(crate) struct Pass<'a> {
    /// The colour pipeline, and each material's in `Meshes`' order.
    pub(crate) pipelines: (&'a wgpu::RenderPipeline, &'a [wgpu::RenderPipeline]),
    /// The lit pipeline, once a lit mesh has made it.
    pub(crate) lit: Option<&'a LitPipeline>,
    pub(crate) camera: &'a wgpu::BindGroup,
    pub(crate) slots: &'a wgpu::BindGroup,
    /// The view drawn into: the multisampled one at 4 samples, resolved
    /// into the backing's, else the backing's.
    pub(crate) colour: (Option<&'a wgpu::TextureView>, &'a wgpu::TextureView),
    pub(crate) depth: &'a wgpu::TextureView,
    /// Counts each draw.
    pub(crate) work: &'a WorkCounter,
}

/// The pipeline a mesh draws with.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    Colour,
    Material(usize),
    Lit,
    Glossy,
}

/// One mesh's draw: pipeline, slot offset and geometry.
pub(crate) type Draw<'a> = (Which, u32, &'a Uploaded);

/// Clears the backing to transparent and the depth to far, then draws
/// each of `draws`, setting the pipeline where the material changes.
pub(crate) fn record(encoder: &mut wgpu::CommandEncoder, pass: &Pass<'_>, draws: &[Draw<'_>]) {
    let mut render = begin(encoder, pass);
    render.set_bind_group(0, pass.camera, &[]);
    let mut set = None;
    for (which, offset, geometry) in draws {
        if set != Some(*which) {
            set_pipeline(&mut render, pass, *which);
            set = Some(*which);
        }
        render.set_bind_group(1, pass.slots, &[*offset]);
        render.set_vertex_buffer(0, geometry.vertices.slice(..));
        if let (Which::Lit | Which::Glossy, Some(normals)) = (which, &geometry.normals) {
            render.set_vertex_buffer(1, normals.slice(..));
        }
        render.set_index_buffer(geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
        let indices = 0..geometry.index_count;
        pass.work.draw_indexed(&mut render, indices, 0, 0..1);
    }
}

fn set_pipeline(render: &mut wgpu::RenderPass<'_>, pass: &Pass<'_>, which: Which) {
    let (colour, materials) = pass.pipelines;
    match (which, pass.lit) {
        (Which::Colour, _) => render.set_pipeline(colour),
        (Which::Material(at), _) => render.set_pipeline(&materials[at]),
        (Which::Lit | Which::Glossy, Some(lit)) => {
            let glossy = which == Which::Glossy;
            render.set_pipeline(if glossy { &lit.glossy } else { &lit.pipeline });
            render.set_bind_group(2, &lit.bind_group, &[]);
        }
        (Which::Lit | Which::Glossy, None) => {
            unreachable!("a lit mesh makes the lit pipeline before it draws")
        }
    }
}

fn begin<'e>(encoder: &'e mut wgpu::CommandEncoder, pass: &Pass<'_>) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene3d"),
        color_attachments: &[Some(colour_attachment(pass.colour))],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: pass.depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// The pass clears its colour to transparent each frame. At 4 samples
/// it draws into the multisampled view and resolves into the backing's,
/// discarding the samples: nothing loads them.
fn colour_attachment<'a>(
    (multisampled, backing): (Option<&'a wgpu::TextureView>, &'a wgpu::TextureView),
) -> wgpu::RenderPassColorAttachment<'a> {
    let store = if multisampled.is_some() {
        wgpu::StoreOp::Discard
    } else {
        wgpu::StoreOp::Store
    };
    wgpu::RenderPassColorAttachment {
        view: multisampled.unwrap_or(backing),
        depth_slice: None,
        resolve_target: multisampled.map(|_| backing),
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store,
        },
    }
}
