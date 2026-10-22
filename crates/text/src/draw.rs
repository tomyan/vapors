//! Drawing a [`Layout`]: each glyph is fetched from the atlas at its
//! quarter-pixel x step with the baseline snapped to a whole pixel, and
//! becomes one quad; the atlas page is uploaded when it changed.

use vapors_scene::{Color, Size};

use crate::atlas::{AtlasError, AtlasGlyph, GlyphAtlas};
use crate::layout::Layout;
use crate::pipeline::{BlendParams, FLOATS_PER_VERTEX, TextPipeline};
use crate::raster::{GlyphRequest, Hinting, subpixel_of};

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

/// Vertex floats for every inked glyph of `layout`.
fn quads(
    atlas: &mut GlyphAtlas,
    layout: &Layout,
    origin: [f32; 2],
    viewport: Size,
) -> Result<Vec<f32>, AtlasError> {
    let hinting = Hinting::for_scale(layout.scale());
    let page = atlas.coverage_page().size();
    let mut floats = Vec::new();
    for glyph in layout.glyphs() {
        let (pixel_x, subpixel) = subpixel_of(origin[0] + glyph.x);
        let baseline = (origin[1] + glyph.y).round();
        let placed = atlas.get(&GlyphRequest {
            font: glyph.font,
            id: glyph.id,
            size: glyph.size,
            coords: glyph.coords,
            subpixel,
            hinting,
        })?;
        if placed.is_empty() {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // pixel positions
        let top_left = [(pixel_x + placed.left) as f32, baseline + placed.top as f32];
        push_quad(&mut floats, top_left, &placed, page, viewport, glyph.color);
    }
    Ok(floats)
}

#[allow(clippy::cast_precision_loss)] // atlas and viewport sizes are far below 2^24
fn push_quad(
    floats: &mut Vec<f32>,
    top_left: [f32; 2],
    glyph: &AtlasGlyph,
    page: u32,
    viewport: Size,
    color: Color,
) {
    let [x, y, w, h] = glyph.rect;
    let (u0, v0) = (x as f32 / page as f32, y as f32 / page as f32);
    let (u1, v1) = ((x + w) as f32 / page as f32, (y + h) as f32 / page as f32);
    let (left, top) = (top_left[0], top_left[1]);
    let (right, bottom) = (left + w as f32, top + h as f32);
    let mut corner = |px: f32, py: f32, u: f32, v: f32| {
        let [cx, cy] = to_clip([px, py], viewport);
        floats.extend_from_slice(&[cx, cy, u, v, color.r, color.g, color.b, color.a]);
    };
    corner(left, top, u0, v0);
    corner(right, top, u1, v0);
    corner(left, bottom, u0, v1);
    corner(right, top, u1, v0);
    corner(right, bottom, u1, v1);
    corner(left, bottom, u0, v1);
}

/// Viewport pixels (origin top-left, y down) to clip space (-1..1, y up).
#[allow(clippy::cast_precision_loss)] // pixel counts are far below 2^24
fn to_clip(point: [f32; 2], viewport: Size) -> [f32; 2] {
    [
        point[0] / viewport.width as f32 * 2.0 - 1.0,
        1.0 - point[1] / viewport.height as f32 * 2.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::PageKind;

    #[test]
    fn a_quad_maps_the_glyph_rect_to_its_atlas_coordinates() {
        // Given: a 4x2 glyph at page (8, 16) in a 32 px page, placed at (10, 20)
        let glyph = AtlasGlyph {
            page: PageKind::Coverage,
            rect: [8, 16, 4, 2],
            left: 0,
            top: 0,
            notdef: false,
        };
        let viewport = Size {
            width: 100,
            height: 50,
        };
        let mut floats = Vec::new();

        // When
        push_quad(
            &mut floats,
            [10.0, 20.0],
            &glyph,
            32,
            viewport,
            Color::default(),
        );

        // Then: six vertices; the first is the top-left, the fifth the bottom-right
        assert_eq!(floats.len(), 6 * FLOATS_PER_VERTEX);
        assert_close(&floats[0..4], &[-0.8, 0.2, 0.25, 0.5]);
        assert_close(
            &floats[4 * FLOATS_PER_VERTEX..][..4],
            &[-0.72, 0.12, 0.375, 0.5625],
        );
    }

    fn assert_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-6, "{actual:?} vs {expected:?}");
        }
    }
}
