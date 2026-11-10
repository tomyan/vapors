//! The 3D scene's renderer: a `SurfaceRenderer` drawing a
//! [`vapors_scene3d::Scene`] on a `gpu-surface` (3D lane slice 2,
//! `docs/plans/3d-2.md`).
//!
//! [`SceneRenderer`] shares the scene and its [`vapors_scene3d::Meshes`]
//! with the application, as `Rc<RefCell<..>>`. Each render it uploads
//! the geometry given since the last (each piece once), runs
//! [`vapors_scene3d::Scene::update`], writes the uniform of each mesh
//! node it returned or that has none yet, writes the camera's
//! view-projection when it changed, and draws every mesh node through
//! that one view-projection, depth-tested. The backing is cleared to
//! transparent first, so the page shows where no mesh draws.

mod camera;
mod depth;
mod draw;
mod error;
mod geometry;
mod pipeline;
mod renderer;
mod slots;

pub use error::FrameError;
pub use renderer::SceneRenderer;
