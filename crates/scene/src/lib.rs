//! Scene: the display list the element tree produces and the renderer
//! consumes. This crate has no GPU or window dependency so its contents
//! can be asserted on in tests without a device.

mod batch;
mod bounds;
mod color;
mod display;
mod image;
mod scissor;
mod shape;
mod transform;

pub use batch::{Batch, Batched, Run, batched, culled};
pub use color::Color;
pub use display::{IconId, ImageId, Item, Quad, Rect, Scene, Shadow, SurfaceId, TextId};
pub use image::RgbaImage;
pub use scissor::Scissor;
pub use shape::{Shape, ShapeRef};
pub use transform::{Size, Transform};
