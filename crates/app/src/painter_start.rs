//! What a painter does once, before the first frame (`App::start`,
//! one-scene F0): each surface renderer's `mount`, and the glyph
//! atlas's new page uploaded, so the first frame pays for neither.

use super::ScenePainter;
use super::errors::PaintError;

impl ScenePainter {
    /// Calls `mount` on each surface renderer not yet mounted, inside
    /// its GPU error scope, and uploads the atlas page's new part on
    /// `queue`. What it writes is start-up's, counted in no frame.
    ///
    /// # Errors
    ///
    /// [`PaintError`] naming the surface whose `mount` failed; every
    /// later frame of that surface is refused too.
    pub fn start(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), PaintError> {
        self.surfaces.start(device)?;
        self.text.upload_atlas(queue, &mut self.atlas);
        let _start_up = self.take_work();
        Ok(())
    }

    /// The format of the targets it draws into.
    #[must_use]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}
