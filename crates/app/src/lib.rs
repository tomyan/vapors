//! A vapors-elements [`Document`](vapors_elements::Document) on screen:
//! [`ScenePainter`] draws an update's scene with the renderer's quad
//! pipeline and the text crate's glyph pipeline, in scene order, and
//! [`ElementsApp`] is the platform `App` that updates and paints the
//! document every frame, natively and on the web canvas.
//! [`DocumentInput`] routes platform input into a document, and
//! [`ScenePainter::draw_over`] paints one over what a target already
//! holds, for an app that shows a document over its own drawing.

mod cursor;
mod document_input;
mod elements_app;
pub mod frame;
#[cfg(test)]
mod gpu_surface_backing_tests;
mod painter;
mod pointer;
#[cfg(test)]
mod scroll_fixture;
mod surface;
mod tooltip;
mod touch_scroll;
mod wheel_scroll;

pub use document_input::{DocumentInput, Given, Route, Unplaced};
pub use elements_app::{
    ElementsApp, Failure, FrameDecision, FrameTimesHook, GiveError, InvalidateError, Invalidated,
    PerFrame, SurfaceHandle,
};
pub use painter::{
    DrawOverError, FrameCounts, MountError, PaintError, SUPPORTED, ScenePainter, SurfaceState,
    UnsupportedFormat,
};
pub use surface::{SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};
pub use vapors_renderer::GpuWork;
