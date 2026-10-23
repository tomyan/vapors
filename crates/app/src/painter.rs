//! Drawing an update: clear to the canvas colour, then each run of the
//! scene's items with the pipeline for its kind, in painting order.

use vapors_elements::Painted;
use vapors_renderer::{QuadPipeline, ShadowPipeline, Target, clear_view};
use vapors_scene::{Batch, Item};
use vapors_text::{AtlasConfig, AtlasError, BlendParams, GlyphAtlas, RenderTarget, TextPipeline};

/// The pipelines and glyph atlas for one target format.
pub struct ScenePainter {
    shadows: ShadowPipeline,
    quads: QuadPipeline,
    text: TextPipeline,
    atlas: GlyphAtlas,
}

impl ScenePainter {
    /// Pipelines for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let atlas = GlyphAtlas::new(AtlasConfig::default());
        Self {
            shadows: ShadowPipeline::new(device, format),
            quads: QuadPipeline::new(device, format),
            text: TextPipeline::new(device, format, &atlas),
            atlas,
        }
    }

    /// Clears `target` to the canvas colour and draws the scene over it.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when a glyph cannot be rasterised or a frame's
    /// glyphs do not fit the atlas page; the frame is then incomplete.
    pub fn draw(&mut self, target: &Target<'_>, painted: &Painted) -> Result<(), AtlasError> {
        clear_view(target.device, target.queue, target.view, painted.canvas());
        self.atlas.begin_frame();
        for batch in painted.scene().batches() {
            match batch {
                Batch::Shadows(shadows) => self.shadows.draw(target, &shadows),
                Batch::Quads(quads) => self.quads.draw(target, &quads),
                Batch::Texts(items) => self.draw_texts(target, painted, items)?,
            }
        }
        Ok(())
    }

    fn draw_texts(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        items: &[Item],
    ) -> Result<(), AtlasError> {
        let text_target = RenderTarget {
            device: target.device,
            queue: target.queue,
            view: target.view,
            viewport: target.viewport,
        };
        for item in items {
            if let Item::Text { text, origin } = item {
                let layout = painted.text(*text);
                self.text.draw(
                    &text_target,
                    &mut self.atlas,
                    layout,
                    *origin,
                    BlendParams::default(),
                )?;
            }
        }
        Ok(())
    }
}
