//! Scene: the display list the element tree produces and the renderer
//! consumes. This crate has no GPU or window dependency so its contents
//! can be asserted on in tests without a device.

mod color;

pub use color::Color;
