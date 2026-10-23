//! Scene: the display list the element tree produces and the renderer
//! consumes. This crate has no GPU or window dependency so its contents
//! can be asserted on in tests without a device.

mod batch;
mod color;
mod display;
mod scissor;
mod transform;

pub use batch::{Batch, Run};
pub use color::Color;
pub use display::{Item, Quad, Rect, Scene, Shadow, TextId};
pub use scissor::Scissor;
pub use transform::{Size, Transform};
