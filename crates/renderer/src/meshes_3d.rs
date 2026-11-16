//! Mesh items on a scene node's plane (docs/plans/decal.md section 2):
//! an entry's triangles and the page's item (`MeshShape`, its affine now
//! to the node's plane units), drawn inside a scene's render pass under
//! the camera and the node's world matrix, with the page's exact-corner
//! coverage.

use crate::projective::{Bound, ProjectiveLayouts, ScenePassFormats, ScenePipeline};
use crate::{MeshPipeline, MeshShape, WorkCounter};
use vapors_shapes::MeshTriangle;

/// A pipeline that draws mesh items in a scene pass. Create once per
/// scene renderer, before the first frame.
pub struct ProjectiveMeshes(wgpu::RenderPipeline);

impl ProjectiveMeshes {
    /// Builds the pipeline on `layouts` for a pass of `formats`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        layouts: &ProjectiveLayouts,
        formats: ScenePassFormats,
    ) -> Self {
        let source = concat!(
            include_str!("projective.wgsl"),
            include_str!("meshes_3d.wgsl"),
            include_str!("meshes_shared.wgsl")
        );
        let buffers = crate::meshes_setup::buffers();
        let shader = ScenePipeline {
            label: "meshes 3d",
            source,
            buffers: &buffers,
        };
        Self(shader.build(device, layouts, formats))
    }

    /// An item's bytes, as [`ProjectiveMeshes::record`] reads them: the
    /// page's.
    #[must_use]
    pub fn item_bytes(item: &MeshShape) -> Vec<u8> {
        MeshPipeline::bytes(&[*item])
    }

    /// An entry's triangle bytes, as the page's entry buffer holds them.
    #[must_use]
    pub fn triangle_bytes(triangles: &[MeshTriangle]) -> Vec<u8> {
        crate::mesh_entries::triangle_bytes(triangles)
    }

    /// Draws `triangles` (bytes of [`ProjectiveMeshes::triangle_bytes`],
    /// their count read from the slice's size) under the item in `item`,
    /// in one draw in `pass`, under the camera and the node slot `bound`
    /// names.
    ///
    /// # Panics
    ///
    /// If `item` is not one item's bytes, or `triangles` is not whole
    /// triangles and the padding to 16 bytes an entry has.
    pub fn record(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        bound: Bound<'_>,
        (item, triangles): (wgpu::BufferSlice<'_>, wgpu::BufferSlice<'_>),
        work: &WorkCounter,
    ) {
        let one = (crate::mesh_shape::INSTANCE_FLOATS * 4) as u64;
        assert_eq!(
            item.size(),
            one,
            "meshes 3d: an item is {one} bytes; pass item_bytes() of one (meshes_3d.rs)"
        );
        let count = triangle_count(triangles.size());
        pass.set_pipeline(&self.0);
        pass.set_bind_group(0, bound.camera, &[]);
        pass.set_bind_group(1, bound.node, &[bound.offset]);
        pass.set_vertex_buffer(0, triangles);
        pass.set_vertex_buffer(1, item);
        work.draw(pass, 0..3, 0..count);
    }
}

/// How many triangles `bytes` of an entry hold: whole triangles, then
/// under 16 bytes of padding (`mesh_entries::mesh_entry_bytes`).
fn triangle_count(bytes: u64) -> u32 {
    let one = crate::mesh_entries::TRIANGLE_BYTES;
    let count = bytes / one;
    assert!(
        bytes - count * one < 16,
        "meshes 3d: {bytes} triangle bytes are not whole {one}-byte triangles and an entry's padding; pass triangle_bytes() (meshes_3d.rs)"
    );
    u32::try_from(count)
        .unwrap_or_else(|_| panic!("meshes 3d: {count} triangles are more than one draw counts"))
}
