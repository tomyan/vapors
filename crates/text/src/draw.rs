//! Drawing a [`Layout`]: each glyph is fetched from the atlas at its
//! quarter-pixel x step with the baseline snapped to a whole pixel, and
//! becomes one quad; the part of the atlas page that changed is uploaded.

use vapors_renderer::Placed;

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::layout::Layout;
use crate::pipeline::{BlendParams, FLOATS_PER_VERTEX, TextPipeline};
use crate::quads::quads;

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
    /// If `atlas`'s page is not the size this pipeline's texture was made
    /// for; that is a caller bug and the message gives both sizes.
    pub fn draw(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        layout: &Layout,
        origin: [f32; 2],
        params: BlendParams,
    ) -> Result<(), AtlasError> {
        let quads = quads(atlas, layout, origin, target.viewport)?;
        self.record(target, atlas, &quads, params);
        Ok(())
    }

    /// Upload what changed in the atlas page, set `params`, and draw `quads`.
    ///
    /// # Panics
    ///
    /// If `atlas`'s page is not the size this pipeline's texture was made for.
    pub(crate) fn record(
        &mut self,
        target: &RenderTarget<'_>,
        atlas: &mut GlyphAtlas,
        quads: &[f32],
        params: BlendParams,
    ) {
        let page = atlas.coverage_page().size();
        assert_eq!(
            page,
            self.texture.width(),
            "atlas page is {page} px but the text pipeline's texture is {} px",
            self.texture.width()
        );
        self.upload_page(target, atlas);
        let params = params_bytes(params);
        let params = self
            .uniforms
            .push(target.encoder, target.queue, &self.work, &params);
        self.draw_quads(target, quads, &params);
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
                ..self.texture.as_image_copy()
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

    fn draw_quads(&mut self, target: &RenderTarget<'_>, quads: &[f32], params: &Placed) {
        if quads.is_empty() {
            return;
        }
        let bytes: Vec<u8> = quads.iter().flat_map(|f| f.to_le_bytes()).collect();
        let vertex_count = u32::try_from(quads.len() / FLOATS_PER_VERTEX).expect("fits u32");
        let bind_group = self.bind_group(target.device, params);
        let vertices = self
            .vertices
            .push(target.encoder, target.queue, &self.work, &bytes);
        target.draw_pass("text", |pass| {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vertices.slice());
            self.work.draw(pass, 0..vertex_count, 0..1);
        });
    }
}

fn params_bytes(params: BlendParams) -> Vec<u8> {
    [params.gamma, params.contrast, 0.0, 0.0]
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}
