//! Drawing a run of the scene's outer box shadows: a rounded rectangle
//! blurred by a Gaussian, in closed form along one axis and four
//! samples along the other, drawn outside the box that casts it.

use vapors_scene::Shadow;

use crate::instanced::{Instanced, instance_bytes, instanced_source, rowed_instances};
use crate::quads::rgba;
use crate::{ClipTable, GpuWork, KeptRange, Target, TransformTable};

/// A pipeline that draws shadows. Create once per target format.
pub struct ShadowPipeline(Instanced);

impl ShadowPipeline {
    /// Four-float attributes an instance holds.
    const VEC4S: u32 = 7;

    /// Bytes per instance in a draw's or a kept buffer: the attributes and
    /// the row index after them (`instanced.rs`).
    pub const INSTANCE_BYTES: usize = instance_bytes(Self::VEC4S);

    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self(Instanced::new(
            device,
            format,
            "shadows",
            &instanced_source(include_str!("shadows.wgsl")),
            Self::VEC4S,
        ))
    }

    /// Draw `shadows` into `target`, in one draw call.
    pub fn draw(&mut self, target: &Target<'_>, shadows: &[Shadow]) {
        self.0.draw(target, &Self::bytes(shadows));
    }

    /// The instance bytes of `shadows`, as a draw writes them.
    #[must_use]
    pub fn bytes(shadows: &[Shadow]) -> Vec<u8> {
        Self::bytes_in(shadows, (0, 0), &[])
    }

    /// The instance bytes of `shadows`, each in frame `frame` (its entry
    /// in the transform table), item `i` in row `rows[i]` of its batch's
    /// range, or every item in row 0 for an empty `rows`.
    ///
    /// # Panics
    ///
    /// When `rows` is neither empty nor one index an item, or names a row
    /// past a batch's range ([`RowOffsets::CAPACITY`](crate::RowOffsets::CAPACITY)).
    #[must_use]
    pub fn bytes_in(shadows: &[Shadow], (frame, clip): (u32, u32), rows: &[u32]) -> Vec<u8> {
        rowed_instances(
            ("shadows", Self::VEC4S as usize),
            (shadows, (frame, clip), rows),
            instance,
        )
    }

    /// Writes `bytes` ([`ShadowPipeline::bytes`] of a run) into the kept buffer,
    /// for [`ShadowPipeline::draw_kept`] in this and later frames. `None`
    /// when the buffer they need is over the device's `max_buffer_size`:
    /// the refusal is on the target's encoder.
    pub fn keep(&mut self, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        self.0.keep(target, bytes)
    }

    /// Draws the shadows `kept` holds into `target`, each at its frame's
    /// entry in `table` and its row's in its batch's range from
    /// `rows_base`, in one draw call.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn draw_kept(
        &mut self,
        target: &Target<'_>,
        kept: &KeptRange,
        table: (&mut TransformTable, &mut ClipTable, u32),
    ) {
        self.0.draw_kept(target, kept, table);
    }

    /// Gives `kept`'s bytes back. Draws of it already recorded still
    /// read them: they are reused once a newer encoder writes or draws.
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
