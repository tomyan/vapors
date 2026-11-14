//! The 3D scene's maths, validation and node store (3D lane slice 2,
//! `docs/plans/3d-2.md`).
//!
//! Conventions, the same everywhere in the scene:
//!
//! - Right-handed: +X right, +Y up, +Z towards the viewer; a camera
//!   looks down its own -Z.
//! - Units are the application's; the examples use metres.
//! - Angles are radians (`f32`). A positive rotation about an axis turns
//!   counter-clockwise looking down that axis towards the origin.
//! - Column vectors: a matrix applies as `matrix * point`. A node's world
//!   matrix is `parent_world * local`, and a [`Transform`]'s matrix is
//!   `translation * rotation * scale`: scale first, translation last.
//! - Clip depth runs 0 (near) to 1 (far) and NDC +Y is up, as in wgpu
//!   (glam's `camera::rh` view and `proj::directx` projections).
//!
//! Every constructor refuses input it cannot use ([`SceneError`]),
//! naming the field and the fix; nothing is clamped or replaced.
//!
//! A [`Scene`] holds the nodes: groups, meshes and cameras in a tree
//! under one root, named by generational [`NodeId`]s, so the id of a
//! removed node is refused ([`StoreError::Stale`]) and never names the
//! node that took its slot. [`Scene::update`] recomputes the world
//! matrices of the nodes whose transform or parent changed, and of their
//! descendants, and returns them. The root names the active camera by
//! [`CameraHandle`].
//!
//! A mesh node names its triangles by a [`GeometryHandle`], issued by a
//! [`Meshes`] that validates them and holds them for each scene
//! renderer (`vapors-scene3d-gpu`) to upload once, while any handle to
//! them lives (the application's, or a mesh node's).

mod appearance;
mod camera;
mod colour;
mod error;
mod geometry_handle;
mod handle;
mod mesh_error;
mod meshes;
mod node;
mod pick;
mod projection;
mod store;
mod store_error;
mod transform;
mod view;
mod world;

#[cfg(test)]
mod test_support;

pub use colour::Colour;
pub use error::SceneError;
pub use geometry_handle::GeometryHandle;
pub use glam::{Mat4, Quat, Vec3};
pub use handle::{CameraHandle, NodeId};
pub use mesh_error::{MeshError, VERTEX_BYTES};
pub use meshes::{Geometry, GeometryId, Meshes};
pub use node::NodeKind;
pub use pick::{PickAt, PickError, pick};
pub use projection::{Projection, project, view_projection};
pub use store::Scene;
pub use store_error::StoreError;
pub use transform::Transform;
pub use view::View;
