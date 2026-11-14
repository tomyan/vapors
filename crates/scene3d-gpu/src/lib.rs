//! The 3D scene's renderer: a `SurfaceRenderer` drawing a
//! [`vapors_scene3d::Scene`] on a `gpu-surface` (3D lane slice 2,
//! `docs/plans/3d-2.md`).
//!
//! [`SceneRenderer`] shares the scene and its [`vapors_scene3d::Meshes`]
//! with the application, as `Rc<RefCell<..>>`; several renderers may
//! share one `Meshes`. Each render it runs
//! [`vapors_scene3d::Scene::update`], uploads each piece of geometry a
//! mesh node names that this renderer has not uploaded yet, writes the
//! uniform of each mesh node whose world version differs from the one
//! it last wrote for that node (so a node another caller updated is
//! written too), writes the camera's view-projection when it changed,
//! and draws every mesh node through that one view-projection,
//! depth-tested. The backing is cleared to transparent first, so the
//! page shows where no mesh draws. A removed mesh node's uniform slot
//! is reused by the next one added. Every upload, uniform write and draw
//! goes through `SurfaceFrame::work`, so the painter's `FrameCounts`
//! holds them under `surfaces`.

mod camera;
mod depth;
mod draw;
mod error;
mod geometry;
mod log;
mod pipeline;
mod renderer;
mod slot_table;
mod slots;

pub use error::FrameError;
pub use log::{RendererLog, Resource};
pub use renderer::SceneRenderer;
