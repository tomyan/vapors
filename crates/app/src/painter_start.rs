//! What a painter does once, before the first frame (`App::start`,
//! one-scene F0): each surface renderer's `mount`, the glyph atlas's
//! new page uploaded, and each surface's backing made at its laid-out
//! size, so the first frame pays for none of them.

use vapors_elements::Painted;

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

    /// At `App::start`, after the document's first update: each surface
    /// of `painted` gets its backing at its laid-out size, its renderer
    /// resized, and the backing cleared, so a first frame at the start's
    /// size neither allocates it nor is the first submit to use its
    /// memory; a first frame at another size remakes it.
    ///
    /// # Errors
    ///
    /// [`PaintError`] naming a surface with no renderer, one too large
    /// for the device, or one whose `mount` or `resize` failed.
    pub fn start_surfaces(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        painted: &Painted,
    ) -> Result<(), PaintError> {
        let started = self
            .surfaces
            .start_at((device, queue), &self.images, painted);
        let _start_up = self.take_work();
        started
    }

    /// The format of the targets it draws into.
    #[must_use]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}
