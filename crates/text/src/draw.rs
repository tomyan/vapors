//! Drawing a [`Layout`]: each glyph is fetched from the atlas at its
//! quarter-pixel x step with the baseline snapped to a whole pixel, and
//! becomes one quad; the atlas page is uploaded when it changed.

use vapors_scene::Size;

use crate::atlas::{AtlasError, GlyphAtlas};
use crate::layout::Layout;
use crate::pipeline::{BlendParams, FLOATS_PER_VERTEX, TextPipeline};
use crate::quads::quads;

/// Where a draw goes: the device and queue to use, the texture view to
/// draw into, and its size in pixels.
pub struct RenderTarget<'a> {
    /// Device the pipeline was made on.
    pub device: &'a wgpu::Device,
    /// Queue to submit to.
    pub queue: &'a wgpu::Queue,
    /// The view drawn into; pixels under no glyph keep what was there.
    pub view: &'a wgpu::TextureView,
    /// Size of the view in pixels.
    pub viewport: Size,
}

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
        let page = atlas.coverage_page().size();
        assert_eq!(
            page,
            self.texture.width(),
            "atlas page is {page} px but the text pipeline's texture is {} px",
            self.texture.width()
        );
        let quads = quads(atlas, layout, origin, target.viewport)?;
        self.upload_page(target.queue, atlas);
        target
            .queue
            .write_buffer(&self.uniforms, 0, &params_bytes(params));
        self.draw_quads(target, &quads);
        Ok(())
    }

    fn upload_page(&self, queue: &wgpu::Queue, atlas: &mut GlyphAtlas) {
        let page = atlas.coverage_page_mut();
        if !page.dirty() {
            return;
        }
        let size = page.size();
        queue.write_texture(
            self.texture.as_image_copy(),
            page.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size),
                rows_per_image: Some(size),
            },
            self.texture.size(),
        );
        page.mark_clean();
    }

    fn draw_quads(&mut self, target: &RenderTarget<'_>, quads: &[f32]) {
        if quads.is_empty() {
            return;
        }
        let (device, queue) = (target.device, target.queue);
        let bytes: Vec<u8> = quads.iter().flat_map(|f| f.to_le_bytes()).collect();
        let vertex_count = u32::try_from(quads.len() / FLOATS_PER_VERTEX).expect("fits u32");
        let bind_group = self.bind_group(device);
        self.reserve_vertices(device, bytes.len() as u64);
        let vertices = self.vertices.as_ref().expect("reserved above");
        queue.write_buffer(vertices, 0, &bytes);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("text"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.draw(0..vertex_count, 0..1);
        }
        queue.submit([encoder.finish()]);
    }
}

fn params_bytes(params: BlendParams) -> Vec<u8> {
    [params.gamma, params.contrast, 0.0, 0.0]
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}
