//! Drawing a run of the scene's outer box shadows: a rounded rectangle
//! blurred by a Gaussian, in closed form along one axis and four
//! samples along the other, drawn outside the box that casts it.

use vapors_scene::Shadow;

use crate::instanced::{Instanced, float_bytes};
use crate::quads::rgba;
use crate::{GpuWork, KeptRange, Target};

/// A pipeline that draws shadows. Create once per target format.
pub struct ShadowPipeline(Instanced);

impl ShadowPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self(Instanced::new(
            device,
            format,
            "shadows",
            include_str!("shadows.wgsl"),
            7,
        ))
    }

    /// Draw `shadows` into `target`, in one draw call.
    pub fn draw(&mut self, target: &Target<'_>, shadows: &[Shadow]) {
        self.0.draw(target, &Self::bytes(shadows));
    }

    /// The instance bytes of `shadows`, as a draw writes them.
    #[must_use]
    pub fn bytes(shadows: &[Shadow]) -> Vec<u8> {
        float_bytes(shadows.iter().flat_map(instance))
    }

    /// Writes `bytes` ([`ShadowPipeline::bytes`] of a run) into the kept buffer,
    /// for [`ShadowPipeline::draw_kept`] in this and later frames. `None`
    /// when the buffer they need is over the device's `max_buffer_size`:
    /// the refusal is on the target's encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.0.keep(target, bytes)
    }

    /// Draws the shadows `kept` holds into `target`, in one draw call.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn draw_kept(&mut self, target: &Target<'_>, kept: &KeptRange) {
        self.0.draw_kept(target, kept);
    }

    /// Gives `kept`'s bytes back. Release a range only in a frame that
    /// does not draw it.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn release(&mut self, kept: KeptRange) {
        self.0.release(kept);
    }

    /// Bytes held in the kept buffer.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        self.0.retained_bytes()
    }

    /// The bind groups kept: one once this pipeline has drawn.
    #[must_use]
    pub fn bind_groups_kept(&self) -> u64 {
        self.0.bind_groups_kept()
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.0.work.take()
    }
}

/// The shadow's seven four-float attributes, as `shadows.wgsl` reads them:
/// the quad to draw ([`Shadow::drawn`], which culling also reads), the
/// shape, its radii, the caster, its radii, and the colour, and the
/// standard deviation.
fn instance(shadow: &Shadow) -> Vec<f32> {
    let sigma = shadow.blur / 2.0;
    let (r, c, quad) = (shadow.rect, shadow.caster, shadow.drawn());
    let mut floats = vec![quad.left, quad.top, quad.right, quad.bottom];
    floats.extend([r.left, r.top, r.right, r.bottom]);
    floats.extend(shadow.radii);
    floats.extend([c.left, c.top, c.right, c.bottom]);
    floats.extend(shadow.caster_radii);
    let [red, green, blue, alpha] = rgba(shadow.color);
    floats.extend([red, green, blue, alpha]);
    floats.push(sigma);
    floats.extend([0.0; 3]);
    floats
}
