//! A range of a [`Retained`](crate::Retained) buffer, held by whoever
//! kept it until it is released (`retained.rs`).

use vapors_scene::Size;

/// A range of a [`Retained`](crate::Retained) buffer, held until it is
/// given back with [`Retained::release`](crate::Retained::release); read
/// only from the buffer that kept it.
#[derive(Debug, PartialEq, Eq)]
#[must_use = "a kept range is held until it is released"]
pub struct KeptRange {
    pub(crate) owner: u64,
    pub(crate) slot: u32,
    /// The viewport of the target clip-space bytes were kept on.
    pub(crate) viewport: Option<Size>,
}

impl KeptRange {
    /// The viewport of the target an image pipeline kept this range's
    /// clip-space quads on; `None` for device-pixel bytes.
    #[must_use]
    pub fn viewport(&self) -> Option<Size> {
        self.viewport
    }

    /// This range, its bytes in clip space for `viewport`.
    pub(crate) fn on(self, viewport: Size) -> Self {
        let viewport = Some(viewport);
        Self { viewport, ..self }
    }
}
