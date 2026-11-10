//! The 3D scene's maths and validation (3D lane slice 2,
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

mod error;
mod projection;
mod transform;
mod view;

pub use error::SceneError;
pub use glam::{Mat4, Quat, Vec3};
pub use projection::{Projection, project, view_projection};
pub use transform::Transform;
pub use view::View;
