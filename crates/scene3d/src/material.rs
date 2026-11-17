//! Application materials (one-scene D5 (b), R2): a mesh drawn by a WGSL
//! function the application gives, in place of its colour. The function
//! sees only what any renderer can give it ([`MATERIAL_WGSL`]: the
//! fragment's world position, its position in the scene's viewport and
//! the viewport's size, the mesh's colour and the vertex shade), so the
//! scene renderer draws it today and the painter's pass can draw it once
//! the scene moves there (`docs/plans/flat-scene.md`, F8). A mesh's
//! material is fixed when `<Mesh>` mounts: a page binding `material`
//! to a value that later changes keeps the first, and nothing refuses
//! the change (a known limit: the compiler does not refuse a reactive
//! value for a prop that is not a `Signal`).

use std::fmt;

use crate::{MaterialError, Meshes, NodeId, NodeKind, Scene, StoreError};

/// The contract a material's WGSL is written against: it defines
/// `fn material(at: MaterialInput) -> vec4<f32>`, the fragment's colour,
/// sRGB-encoded as the mesh's colour is, and may define functions and
/// constants of its own beside it. The renderer puts this struct before
/// it.
pub const MATERIAL_WGSL: &str = "
// What a material is given for each fragment (vapors-scene3d `material.rs`).
struct MaterialInput {
    // The fragment's position in world space.
    world: vec3<f32>,
    // The fragment's position in the scene's viewport: device pixels from
    // the viewport's top left, whatever target the scene is drawn into.
    screen: vec2<f32>,
    // The scene's viewport's size in device pixels.
    size: vec2<f32>,
    // The mesh's colour (its anchor's CSS `color`), sRGB-encoded.
    colour: vec4<f32>,
    // The vertex shade, interpolated (one-scene D4).
    shade: f32,
}
";

/// A material: a label for errors and its WGSL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    /// Names it in a renderer's refusals.
    pub label: String,
    /// The functions, `material` among them ([`MATERIAL_WGSL`]).
    pub wgsl: String,
}

/// A material in one [`Meshes`]; a mesh node names it
/// ([`Scene::set_material`]). It carries its `Meshes`' id, so a renderer
/// can refuse one from another `Meshes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId {
    meshes: u64,
    index: usize,
}

impl MaterialId {
    /// Its place among its `Meshes`' materials, in the order given.
    #[must_use]
    pub fn index(self) -> usize {
        self.index
    }
}

impl fmt::Display for MaterialId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "material {} of meshes {}", self.index, self.meshes)
    }
}

impl Meshes {
    /// Holds `material` for the renderers that draw this `Meshes`; each
    /// builds its pipeline when it mounts, so give it before then.
    ///
    /// # Errors
    ///
    /// [`MaterialError`]: its WGSL reads more than the contract, binds
    /// anything, defines a name the renderers use, or has no
    /// `material` function of the contract's shape
    /// (`material_check.rs`).
    pub fn material(&mut self, material: Material) -> Result<MaterialId, MaterialError> {
        crate::material_check::check(&material)?;
        self.materials.push(material);
        Ok(MaterialId {
            meshes: self.id,
            index: self.materials.len() - 1,
        })
    }

    /// The materials given, in order: [`MaterialId::index`] is the place.
    #[must_use]
    pub fn materials(&self) -> &[Material] {
        &self.materials
    }

    /// Whether `material` is one of this `Meshes`' materials.
    #[must_use]
    pub fn issued_material(&self, material: MaterialId) -> bool {
        material.meshes == self.id && material.index < self.materials.len()
    }
}

impl Scene {
    /// Draws mesh `id` with `material`, or its colour for `None`; whether
    /// it changed.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::NotMesh`] for a group or
    /// camera.
    pub fn set_material(
        &mut self,
        id: NodeId,
        material: Option<MaterialId>,
    ) -> Result<bool, StoreError> {
        let node = self.node_mut(id)?;
        if !matches!(node.kind, NodeKind::Mesh { .. }) {
            return Err(StoreError::NotMesh(id, "set_material"));
        }
        let changed = node.material != material;
        node.material = material;
        Ok(changed)
    }

    /// The material mesh `id` is drawn with; `None` for its colour, and
    /// for a group or camera.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn material(&self, id: NodeId) -> Result<Option<MaterialId>, StoreError> {
        Ok(self.node(id)?.material)
    }
}

#[cfg(test)]
#[path = "material_tests.rs"]
mod tests;
