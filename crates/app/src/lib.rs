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
mod fling;
mod fling_booster;
mod fling_curve;
pub mod frame;
#[cfg(test)]
mod gpu_start_tests;
#[cfg(test)]
mod gpu_surface_backing_tests;
mod painter;
mod pointer;
#[cfg(test)]
mod scroll_fixture;
mod shape_bounds;
mod shape_entry;
mod shape_error;
mod shape_store;
mod shape_store_meshes;
mod shape_store_runtime;
mod surface;
mod thumb_fade;
mod tooltip;
mod tooltip_box;
mod tooltip_error;
mod tooltip_place;
mod tooltip_place_check;
mod tooltip_style;
mod touch_scroll;
mod wheel_scroll;

pub use document_input::{
    CURVE_SPEED_LIMIT, DocumentInput, FlingError, Given, MAX_FLING_SPEED, Route, Unplaced,
};
pub use elements_app::{
    ElementsApp, Failure, FrameDecision, FrameTimesHook, FrameTiming, GiveError, InvalidateError,
    Invalidated, PerFrame, Preparing, StartState, SurfaceHandle,
};
pub use painter::{
    DrawOverError, FrameCounts, MountError, NewPainterError, PaintError, SUPPORTED, ScenePainter,
    SurfaceState, UnsupportedFormat,
};
pub use shape_store::{ShapeError, ShapeStore, kind_of, shape_item};
pub use surface::{
    SURFACE_FORMAT, SURFACE_PHASE, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize,
};
pub use tooltip::{TOOLTIP_DELAY, Tooltip, TooltipTrigger};
pub use tooltip_error::TooltipError;
pub use tooltip_place::{
    Anchor, CURSOR_OFFSET_X, CURSOR_OFFSET_Y, KEYBOARD_GAP, PlacementError, PlacementField, Point,
    place_tooltip,
};
pub use tooltip_style::check_tooltip_style;
pub use vapors_renderer::{GpuMissing, GpuReading, GpuResolution, GpuTime, GpuWork, WorkCounter};
