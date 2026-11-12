//! The atlas page on the GPU: the text pipeline's texture kept the
//! page's size, and what changed in the page uploaded before a draw.

use crate::atlas::GlyphAtlas;
use crate::draw::RenderTarget;
use crate::pipeline::TextPipeline;

impl TextPipeline {
    /// A texture of the atlas page's size when the page grew; the page
    /// is whole in its dirty rect then, so the upload fills it.
    ///
    /// # Panics
    ///
    /// If the page is smaller than the texture: pages only grow, so this
    /// pipeline was made for another atlas.
    pub(crate) fn match_page_size(&mut self, device: &wgpu::Device, atlas: &GlyphAtlas) {
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
    pub(crate) fn upload_page(&self, target: &RenderTarget<'_>, atlas: &mut GlyphAtlas) {
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
}
