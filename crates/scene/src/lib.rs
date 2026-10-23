//! Scene: the display list the element tree produces and the renderer
//! consumes. This crate has no GPU or window dependency so its contents
//! can be asserted on in tests without a device.

mod color;
mod display;
mod transform;

pub use color::Color;
pub use display::{Batch, Item, Quad, Rect, Scene, Shadow, TextId};
pub use transform::{Size, Transform};
