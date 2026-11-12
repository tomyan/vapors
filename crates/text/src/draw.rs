//! Drawing a [`Layout`]: each glyph is fetched from the atlas at its
//! quarter-pixel x step with the baseline snapped to a whole pixel, and
//! becomes one quad; the part of the atlas page that changed is uploaded.

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::layout::Layout;
use crate::pipeline::{self, BlendParams, FLOATS_PER_VERTEX, TextPipeline};
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
        let quads = quads(atlas, texts, target.viewport)?;
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
        self.match_page_size(target.device, atlas);
        self.upload_page(target, atlas);
        let params = params_bytes(params);
        let Some(params) = self
            .params
            .slot(target.encoder, target.queue, &self.work, params)
        else {
            return;
        };
        self.draw_quads(target, quads, params);
    }

    /// A texture of the atlas page's size when the page grew; the page
    /// is whole in its dirty rect then, so the upload fills it.
    ///
    /// # Panics
    ///
    /// If the page is smaller than the texture: pages only grow, so this
    /// pipeline was made for another atlas.
    fn match_page_size(&mut self, device: &wgpu::Device, atlas: &GlyphAtlas) {
        let page = atlas.coverage_page().size();
        let texture = self.atlas_texture.texture().width();
        assert!(
            page >= texture,
            "atlas page is {page} px but the text pipeline's texture is {texture} px; pages \
             only grow, so this pipeline was made for another atlas"
        );
        if page > texture {
            self.atlas_texture.replace(device, page);
        }
    }

    /// Upload the part of the atlas page that changed since the last
    /// upload: the bounding rectangle of the glyphs added, or the whole
    /// page when it is new or was repacked. Its rows go packed tight;
    /// `Queue::write_texture`, unlike a buffer-to-texture copy, takes
    /// any `bytes_per_row` and pads rows itself when staging. A repack
    /// moves glyphs that draws already recorded this frame read, so those
    /// are submitted first: a frame that repacks submits once more.
    fn upload_page(&self, target: &RenderTarget<'_>, atlas: &mut GlyphAtlas) {
        let page = atlas.coverage_page_mut();
        let Some(rect @ [x, y, width, height]) = page.dirty_rect() else {
            return;
        };
        if page.cleared() {
            target.encoder.flush(target.queue);
        }
        self.work.write_texture(
            target.queue,
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d { x, y, z: 0 },
                ..self.atlas_texture.texture().as_image_copy()
            },
            &page.region_pixels(rect),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * page.bytes_per_pixel()),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        page.mark_clean();
    }

    /// Draws `quads` with the params slot at `params`.
    fn draw_quads(&mut self, target: &RenderTarget<'_>, quads: &[f32], params: u32) {
        if quads.is_empty() {
            return;
        }
        let bytes: Vec<u8> = quads.iter().flat_map(|f| f.to_le_bytes()).collect();
        let vertex_count = u32::try_from(quads.len() / FLOATS_PER_VERTEX).expect("fits u32");
        let Some(vertices) = self
            .vertices
            .push(target.encoder, target.queue, &self.work, &bytes)
        else {
            return;
        };
        let _encoding = tracing::trace_span!("draw step", phase = "encode").entered();
        let Self {
            pipeline,
            layout,
            sampler,
            params: slots,
            atlas_texture,
            work,
            ..
        } = self;
        let bind_group = atlas_texture.bind_group(slots.generation(), |view| {
            pipeline::bind_group(target.device, work, layout, view, sampler, slots)
        });
        target.draw_pass(|pass| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[params]);
            pass.set_vertex_buffer(0, vertices.slice());
            work.draw(pass, 0..vertex_count, 0..1);
        });
    }
}

/// The params uniform's bytes: gamma and contrast, padded to 16.
fn params_bytes(params: BlendParams) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&params.gamma.to_le_bytes());
    bytes[4..8].copy_from_slice(&params.contrast.to_le_bytes());
    bytes
}
