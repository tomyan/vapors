//! A vapors-elements [`Document`](vapors_elements::Document) on screen:
//! [`ScenePainter`] draws an update's scene with the renderer's quad
//! pipeline and the text crate's glyph pipeline, in scene order, and
//! [`ElementsApp`] is the platform `App` that updates and paints the
//! document every frame, natively and on the web canvas.

mod cursor;
mod elements_app;
pub mod frame;
mod painter;
mod pointer;
#[cfg(test)]
mod scroll_fixture;
mod touch_scroll;
mod wheel_scroll;

pub use elements_app::{ElementsApp, Failure, FrameTimesHook, PerFrame};
pub use painter::ScenePainter;
