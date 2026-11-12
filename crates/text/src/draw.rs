//! Drawing a [`Layout`]: each glyph is fetched from the atlas at its
//! quarter-pixel x step with the baseline snapped to a whole pixel, and
//! becomes one quad; the part of the atlas page that changed is uploaded.

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::layout::Layout;
use vapors_renderer::{KeptRange, Placed};
use vapors_scene::Size;

use crate::pipeline::{self, BlendParams, TextPipeline, VERTEX_SIZE};
use crate::quads::{TextDraw, quads};

/// Where a draw goes: [`vapors_renderer::Target`], under the name the
/// text pipeline has always used.
pub type RenderTarget<'a> = vapors_renderer::Target<'a>;

impl TextPipeline {
    /// Draw `layout` with its origin at `origin` (device pixels, y down)
    /// into `target`.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when a glyph cannot be rasterised or the layout's
    /// glyphs overflow the atlas page; nothing is drawn then.
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is smaller than this pipeline's texture: it was
    /// made for another atlas, and the message gives both sizes.
    pub fn draw(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        layout: &Layout,
        origin: [f32; 2],
        params: BlendParams,
    ) -> Result<(), AtlasError> {
        self.draw_texts(target, atlas, &[TextDraw { layout, origin }], params)
    }

    /// Draw `texts` into `target` in one draw call, in order: where
    /// their glyphs overlap, later ones blend over earlier ones as they
    /// would drawn one at a time.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when a glyph cannot be rasterised or the texts'
    /// glyphs overflow the atlas page; nothing is drawn then.
    ///
    /// # Panics
    ///
    /// As [`TextPipeline::draw`].
    pub fn draw_texts(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        texts: &[TextDraw<'_>],
        params: BlendParams,
    ) -> Result<(), AtlasError> {
        let quads = quads(atlas, texts, None)?;
        self.record(target, atlas, &quads, params);
        Ok(())
    }

    /// Upload what changed in the atlas page, set `params`, and draw `quads`.
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is smaller than this pipeline's texture.
    pub(crate) fn record(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        quads: &[f32],
        params: BlendParams,
    ) {
        let Some(params) = self.prepare(target, atlas, params) else {
            return;
        };
        self.draw_quads(target, quads, params);
    }

    /// Uploads what changed in the atlas page; the dynamic offset of the
    /// slot holding `params` and the target's viewport, `None` when
    /// refused (the refusal is on the encoder).
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is smaller than this pipeline's texture.
    pub(crate) fn prepare(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        params: BlendParams,
    ) -> Option<u32> {
        self.match_page_size(target.device, atlas);
        self.upload_page(target, atlas);
        let params = params_bytes(params, target.viewport);
        self.params
            .slot(target.encoder, target.queue, &self.work, params)
    }

    /// Draws `quads` with the params slot at `params`.
    fn draw_quads(&mut self, target: &RenderTarget<'_>, quads: &[f32], params: u32) {
        if quads.is_empty() {
            return;
        }
        let bytes = vertex_bytes(quads);
        let Some(vertices) = self
            .vertices
            .push(target.encoder, target.queue, &self.work, &bytes)
        else {
            return;
        };
        self.record_draw(target, Vertices::Placed(&vertices), params);
    }

    /// Records a draw of the vertices `from` holds with the params slot
    /// at `params`, through the kept bind group.
    pub(crate) fn record_draw(
        &mut self,
        target: &RenderTarget<'_>,
        from: Vertices<'_>,
        params: u32,
    ) {
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let Self {
            pipeline,
            layout,
            sampler,
            params: slots,
            atlas_texture,
            work,
            kept,
            ..
        } = self;
        let (vertices, count) = match from {
            Vertices::Placed(placed) => (placed.slice(), vertex_count(placed.size)),
            Vertices::Kept(range) => {
                let count = vertex_count(kept.len(range));
                (kept.slice(target.encoder, range), count)
            }
        };
        let bind_group = atlas_texture.bind_group(slots.generation(), |view| {
            pipeline::bind_group(target.device, work, layout, view, sampler, slots)
        });
        target.draw_pass(|pass| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[params]);
            pass.set_vertex_buffer(0, vertices);
            work.draw(pass, 0..count, 0..1);
        });
    }
}

/// Where a draw's vertices are: this frame's arena, or kept.
#[derive(Clone, Copy)]
pub(crate) enum Vertices<'a> {
    Placed(&'a Placed),
    Kept(&'a KeptRange),
}

/// The params uniform's bytes: gamma, contrast and the viewport's width
/// and height in device pixels.
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
fn params_bytes(params: BlendParams, viewport: Size) -> [u8; 16] {
    let floats = [
        params.gamma,
        params.contrast,
        viewport.width as f32,
        viewport.height as f32,
    ];
    let mut bytes = [0; 16];
    for (at, float) in floats.into_iter().enumerate() {
        bytes[at * 4..][..4].copy_from_slice(&float.to_le_bytes());
    }
    bytes
}

/// Little-endian bytes of vertex floats.
pub(crate) fn vertex_bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// How many vertices `bytes` bytes hold.
pub(crate) fn vertex_count(bytes: u64) -> u32 {
    u32::try_from(bytes / VERTEX_SIZE).expect("more text vertices in one batch than fit u32")
}
