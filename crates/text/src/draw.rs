//! Drawing a [`Layout`]: each glyph is fetched from the atlas at its
//! quarter-pixel x step with the baseline snapped to a whole pixel, and
//! becomes one quad; the part of the atlas page that changed is uploaded.

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::layout::Layout;
use vapors_renderer::{ClipTable, KeptRange, Placed, TransformTable};
use vapors_scene::Size;

use crate::pipeline::{self, BlendParams, PARAMS_SIZE, TextPipeline};
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
        let Some(params) = self.prepare(target, atlas, (params, 0)) else {
            return;
        };
        self.draw_quads(target, quads, params);
    }

    /// Uploads what changed in the atlas page; the dynamic offset of the
    /// slot holding `params`, the target's viewport and the draw's first
    /// row entry `rows_base`, `None` when refused (the refusal is on the
    /// encoder).
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is smaller than this pipeline's texture.
    pub(crate) fn prepare(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        (params, rows_base): (BlendParams, u32),
    ) -> Option<u32> {
        self.match_page_size(target.device, atlas);
        self.upload_page(target, atlas);
        let value = params_bytes(params, target.viewport, rows_base);
        self.params
            .slot(target.encoder, target.queue, &self.work, value)
    }

    /// Draws `quads`, each on the page, with the params slot at `params`.
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
        self.identity.prepare_counted(target.queue, &self.work);
        self.no_clips.prepare_counted(target.queue, &self.work);
        self.record_draw(target, Vertices::Placed(&vertices), (None, params));
    }

    /// Records a draw of the vertices `from` holds with the params slot
    /// at `params`, through the kept bind group over `tables`, the
    /// transform and clip tables (the pipeline's identity table and its
    /// table of no clips for `None`).
    pub(crate) fn record_draw(
        &mut self,
        target: &RenderTarget<'_>,
        from: Vertices<'_>,
        (tables, params): (Option<(&TransformTable, &ClipTable)>, u32),
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
            identity,
            no_clips,
            samples,
            ..
        } = self;
        let (vertices, count) = match from {
            Vertices::Placed(placed) => (placed.slice(), vertex_count(placed.size)),
            Vertices::Kept(range) => {
                let count = vertex_count(kept.len(range));
                (kept.slice(target.encoder, range), count)
            }
        };
        let own = tables.is_none();
        let (table, clips) = tables.unwrap_or((identity, no_clips));
        let generation = [slots.generation(), table.generation(), clips.generation()];
        let bind_group = atlas_texture.bind_group((own, generation), |view| {
            pipeline::bind_group(
                target.device,
                work,
                layout,
                view,
                sampler,
                (slots, (table, clips)),
            )
        });
        target.draw_pass(("text", *samples), |pass| {
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

/// The params uniform's bytes: gamma, contrast, the viewport's width and
/// height in device pixels, then the draw's first row entry, padded to
/// [`PARAMS_SIZE`].
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
fn params_bytes(params: BlendParams, viewport: Size, rows_base: u32) -> [u8; PARAMS_SIZE] {
    let floats = [
        params.gamma,
        params.contrast,
        viewport.width as f32,
        viewport.height as f32,
    ];
    let mut bytes = [0; PARAMS_SIZE];
    for (at, float) in floats.into_iter().enumerate() {
        bytes[at * 4..][..4].copy_from_slice(&float.to_le_bytes());
    }
    bytes[16..20].copy_from_slice(&rows_base.to_le_bytes());
    bytes
}

#[path = "draw_vertices.rs"]
mod vertices;
pub(crate) use vertices::{rowed_vertex_bytes, vertex_bytes, vertex_count};
