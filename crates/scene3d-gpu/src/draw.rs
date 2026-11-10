//! Finding the mesh nodes and recording their draws.

use crate::FrameError;
use crate::geometry::Uploaded;
use vapors_app::WorkCounter;
use vapors_scene3d::{Colour, GeometryId, Meshes, NodeId, NodeKind, Scene};

/// A mesh node to draw.
pub(crate) struct MeshNode {
    pub(crate) node: NodeId,
    pub(crate) geometry: GeometryId,
    pub(crate) colour: Colour,
}

/// Every mesh node under the root, parents before children, each
/// naming geometry `meshes` issued.
pub(crate) fn mesh_nodes(scene: &Scene, meshes: &Meshes) -> Result<Vec<MeshNode>, FrameError> {
    let mut found = Vec::new();
    let mut stack = vec![scene.root()];
    while let Some(node) = stack.pop() {
        found.extend(mesh_node(node, scene.kind(node)?, meshes)?);
        stack.extend(scene.children(node)?.iter().rev());
    }
    Ok(found)
}

/// `node` to draw when it is a mesh naming geometry `meshes` issued.
fn mesh_node(
    node: NodeId,
    kind: &NodeKind,
    meshes: &Meshes,
) -> Result<Option<MeshNode>, FrameError> {
    let NodeKind::Mesh { geometry, colour } = *kind else {
        return Ok(None);
    };
    if !meshes.issued(geometry) {
        return Err(FrameError::ForeignGeometry { node, geometry });
    }
    Ok(Some(MeshNode {
        node,
        geometry,
        colour,
    }))
}

/// What one pass draws with.
pub(crate) struct Pass<'a> {
    pub(crate) pipeline: &'a wgpu::RenderPipeline,
    pub(crate) camera: &'a wgpu::BindGroup,
    pub(crate) slots: &'a wgpu::BindGroup,
    pub(crate) colour: &'a wgpu::TextureView,
    pub(crate) depth: &'a wgpu::TextureView,
    /// Counts each draw.
    pub(crate) work: &'a WorkCounter,
}

/// Clears the backing to transparent and the depth to far, then draws
/// each `(slot offset, geometry)`.
pub(crate) fn record(
    encoder: &mut wgpu::CommandEncoder,
    pass: &Pass<'_>,
    draws: &[(u32, &Uploaded)],
) {
    let mut render = begin(encoder, pass);
    render.set_pipeline(pass.pipeline);
    render.set_bind_group(0, pass.camera, &[]);
    for (offset, geometry) in draws {
        render.set_bind_group(1, pass.slots, &[*offset]);
        render.set_vertex_buffer(0, geometry.vertices.slice(..));
        render.set_index_buffer(geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
        let indices = 0..geometry.index_count;
        pass.work.draw_indexed(&mut render, indices, 0, 0..1);
    }
}

fn begin<'e>(encoder: &'e mut wgpu::CommandEncoder, pass: &Pass<'_>) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene3d"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: pass.colour,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
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
