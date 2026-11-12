//! Text and icon vertices kept between frames (IU4 PR 3,
//! `docs/plans/iu4.md` section 2): a batch's vertex bytes, in device
//! pixels, built once with each glyph or icon mask they name pinned in
//! the atlas (`atlas_pins.rs`), written into the pipeline's kept buffer
//! and drawn from it on targets of any viewport, until released. Bytes
//! built at an atlas generation behind the atlas's name moved glyphs:
//! build them again.

use vapors_renderer::KeptRange;

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::atlas_pins::AtlasPins;
use crate::draw::{RenderTarget, Vertices, vertex_bytes};
use crate::draw_icon::{IconDraw, icon_quads};
use crate::pipeline::{BlendParams, TextPipeline};
use crate::quads::{TextDraw, quads};

impl TextPipeline {
    /// The vertex bytes of `texts` as [`TextPipeline::draw_texts`] draws
    /// them, and the pins on every glyph they name.
    ///
    /// # Errors
    ///
    /// As [`TextPipeline::draw_texts`]; nothing is pinned then.
    pub fn text_bytes(
        atlas: &mut GlyphAtlas,
        texts: &[TextDraw<'_>],
    ) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
        let mut pins = atlas.new_pins();
        let floats = quads(atlas, texts, Some(&mut pins))?;
        Ok((vertex_bytes(&floats), pins))
    }

    /// The vertex bytes of `icons` as [`TextPipeline::draw_icons`] draws
    /// them, and the pins on every icon mask they name.
    ///
    /// # Errors
    ///
    /// As [`TextPipeline::draw_icons`]; nothing is pinned then.
    pub fn icon_bytes(
        atlas: &mut GlyphAtlas,
        icons: &[IconDraw<'_>],
    ) -> Result<(Vec<u8>, AtlasPins), AtlasError> {
        let mut pins = atlas.new_pins();
        let floats = icon_quads(atlas, icons, Some(&mut pins))?;
        Ok((vertex_bytes(&floats), pins))
    }

    /// Writes `bytes` ([`TextPipeline::text_bytes`] or
    /// [`TextPipeline::icon_bytes`]) into the kept buffer, for
    /// [`TextPipeline::draw_kept`] in this and later frames. `None`
    /// when the buffer they need is over the device's `max_buffer_size`:
    /// the refusal is on the target's encoder.
    ///
    /// # Panics
    ///
    /// If `bytes` is empty: a batch with no ink has nothing to keep.
    pub fn keep(&mut self, target: &RenderTarget<'_>, bytes: &[u8]) -> Option<KeptRange> {
        let (frame, queue) = (target.encoder, target.queue);
        self.kept.keep(frame, queue, &self.work, bytes)
    }

    /// Uploads what changed in `atlas`'s page and draws the vertices
    /// `kept` holds into `target` with `params`, in one draw call; not
    /// when the params slot is refused (the refusal is on the encoder).
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline, or as
    /// [`TextPipeline::draw`] for the atlas.
    pub fn draw_kept(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        kept: &KeptRange,
        params: BlendParams,
    ) {
        let Some(params) = self.prepare(target, atlas, params) else {
            return;
        };
        self.record_draw(target, Vertices::Kept(kept), params);
    }

    /// Gives `kept`'s bytes back. Draws of it already recorded still
    /// read them: they are reused once a newer encoder writes or draws.
    ///
    /// # Panics
    ///
    /// If `kept` was kept by another pipeline.
    pub fn release(&mut self, kept: KeptRange) {
        self.kept.release(kept);
    }

    /// Bytes held in the kept buffer.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        self.kept.live_bytes()
    }
}
